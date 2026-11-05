use serde::Deserialize;
use std::fmt::{Display, Write as _};
use std::fs;
use std::io;

const CONFIG: &str = "leaves.toml";
const FRAG: &str = "assets/shaders/leaves.frag";
const VERT: &str = "assets/shaders/leaves.vert";
const TEMPLATE: &str = "assets/scripts/leaves.js";

const MARKER: &str = "#pragma leaves_config";
const MAX_LAYERS: usize = 8;

const MAX_SIZE_RATIO: f64 = 0.1875;

fn err(msg: impl Display) -> io::Error {
    io::Error::other(msg.to_string())
}

fn ensure(ok: bool, msg: impl Display) -> io::Result<()> {
    if ok { Ok(()) } else { Err(err(format!("{CONFIG}: {msg}"))) }
}

fn read(path: &str) -> io::Result<String> {
    fs::read_to_string(path).map_err(|e| err(format!("Failed to read '{path}': {e}")))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    opacity: f64,
    render: Render,
    wind: Wind,
    layers: Vec<Layer>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Render {
    max_dpr: f64,
    max_fps: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wind {
    direction: [f64; 2],
    speed: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Layer {
    cell: f64,
    size: f64,
    density: f64,
    speed: f64,
    blur: f64,
    opacity: f64,
}

impl Config {
    fn parse(text: &str) -> io::Result<Self> {
        let cfg: Config =
            toml::from_str(text).map_err(|e| err(format!("Failed to parse {CONFIG}: {e}")))?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> io::Result<()> {
        let n = self.layers.len();
        ensure((1..=MAX_LAYERS).contains(&n), format!("need 1 to {MAX_LAYERS} layers, got {n}"))?;
        ensure((0.0..=2.0).contains(&self.opacity), "opacity must be within 0 to 2")?;

        ensure(
            self.render.max_dpr.is_finite() && (0.5..=4.0).contains(&self.render.max_dpr),
            "render.max_dpr must be within 0.5 to 4",
        )?;
        ensure(self.render.max_fps <= 240, "render.max_fps must be at most 240")?;

        let [dx, dy] = self.wind.direction;
        ensure(
            dx.is_finite() && dy.is_finite() && dx.hypot(dy) > 1e-6,
            "wind.direction must be a non-zero vector",
        )?;
        ensure(self.wind.speed.is_finite() && self.wind.speed >= 0.0, "wind.speed must be >= 0")?;

        for (i, l) in self.layers.iter().enumerate() {
            let at = format!("layers[{i}]");
            let all = [l.cell, l.size, l.density, l.speed, l.blur, l.opacity];
            ensure(all.iter().all(|v| v.is_finite()), format!("{at}: values must be finite"))?;
            ensure(l.cell > 0.0 && l.size > 0.0, format!("{at}: cell and size must be > 0"))?;
            ensure(
                l.density > 0.0 && l.density <= 1.0,
                format!("{at}: density must be within (0, 1]"),
            )?;
            ensure(l.speed >= 0.0 && l.blur >= 0.0, format!("{at}: speed and blur must be >= 0"))?;
            ensure((0.0..=1.0).contains(&l.opacity), format!("{at}: opacity must be within 0 to 1"))?;
            ensure(
                l.size / l.cell <= MAX_SIZE_RATIO,
                format!(
                    "{at}: size/cell is {:.3}, above {MAX_SIZE_RATIO}; the shader only tests the 2x2 \
                     nearest cells, so bigger leaves would be clipped. Raise cell or lower size.",
                    l.size / l.cell
                ),
            )?;
        }
        Ok(())
    }

    fn glsl_prelude(&self) -> String {
        let n = self.layers.len();
        let columns: [(&str, fn(&Layer) -> f64); 6] = [
            ("CELL", |l| l.cell),
            ("SIZE", |l| l.size),
            ("DENS", |l| l.density),
            ("SPD", |l| l.speed),
            ("BLUR", |l| l.blur),
            ("OPAC", |l| l.opacity),
        ];

        let mut s = format!("const int LAYERS = {n};\n");
        for (name, get) in columns {
            let values: Vec<String> = self.layers.iter().map(|l| glsl_float(get(l))).collect();
            let _ = writeln!(s, "const float {name}[LAYERS] = float[LAYERS]({});", values.join(", "));
        }

        let [dx, dy] = self.wind.direction;
        let len = dx.hypot(dy);
        let _ = writeln!(s, "const float OPACITY = {};", glsl_float(self.opacity));
        let _ = writeln!(s, "const vec2 WIND = vec2({}, {});", glsl_float(dx / len), glsl_float(dy / len));
        let _ = write!(s, "const float WIND_SPEED = {};", glsl_float(self.wind.speed));
        s
    }

    fn js_config(&self) -> String {
        format!(
            "{{\n    maxDpr: {:?},\n    maxFps: {},\n  }}",
            self.render.max_dpr,
            self.render.max_fps,
        )
    }
}

fn glsl_float(v: f64) -> String {
    format!("{v:?}")
}

fn splice_prelude(src: &str, prelude: &str) -> io::Result<String> {
    let mut found = 0;
    let lines: Vec<&str> = src
        .lines()
        .map(|line| {
            if line.trim() == MARKER {
                found += 1;
                prelude
            } else {
                line
            }
        })
        .collect();
    if found != 1 {
        return Err(err(format!("{FRAG}: expected exactly one '{MARKER}' line, found {found}")));
    }
    Ok(lines.join("\n") + "\n")
}

fn compact_glsl(src: &str) -> String {
    let mut bare = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        let next = chars.peek().copied();
        if c == '/' && next == Some('/') {
            for n in chars.by_ref() {
                if n == '\n' {
                    bare.push('\n');
                    break;
                }
            }
        } else if c == '/' && next == Some('*') {
            chars.next();
            let mut prev = '\0';
            for n in chars.by_ref() {
                if prev == '*' && n == '/' {
                    break;
                }
                if n == '\n' {
                    bare.push('\n');
                }
                prev = n;
            }
            bare.push(' ');
        } else {
            bare.push(c);
        }
    }
    bare.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn uniform_names(glsl: &str) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    for line in glsl.lines() {
        let Some(rest) = line.strip_prefix("uniform ") else { continue };
        let decl = rest.strip_suffix(';').filter(|d| !d.contains(','));
        let name = decl
            .and_then(|d| d.split_whitespace().last())
            .and_then(|tok| tok.split('[').next())
            .filter(|n| !n.is_empty());
        match name {
            Some(n) => names.push(n.to_string()),
            None => {
                return Err(err(format!(
                    "{FRAG}: cannot read uniform declaration '{line}' (one name per line, ending in ';')"
                )));
            }
        }
    }
    Ok(names)
}

fn uniforms_used_by_js(js: &str) -> Vec<String> {
    let bytes = js.as_bytes();
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut used = Vec::new();
    for (at, _) in js.match_indices("loc.") {
        if at > 0 && is_ident(bytes[at - 1]) {
            continue;
        }
        let tail = &js[at + 4..];
        let end = tail.bytes().position(|b| !is_ident(b)).unwrap_or(tail.len());
        if end > 0 && !used.iter().any(|u: &String| u == &tail[..end]) {
            used.push(tail[..end].to_string());
        }
    }
    used
}

fn check_uniform_parity(declared: &[String], used: &[String]) -> io::Result<()> {
    for u in used {
        if !declared.contains(u) {
            return Err(err(format!("{TEMPLATE} sets loc.{u}, but {FRAG} declares no such uniform")));
        }
    }
    for d in declared {
        if !used.contains(d) {
            return Err(err(format!("{FRAG} declares uniform {d}, but {TEMPLATE} never sets it")));
        }
    }
    Ok(())
}

fn js_template_literal(glsl: &str, what: &str) -> io::Result<String> {
    if glsl.contains(['`', '\\']) || glsl.contains("${") {
        return Err(err(format!("{what}: GLSL may not contain backticks, backslashes or '${{'")));
    }
    Ok(format!("`{glsl}`"))
}

fn fill_template(template: &str, parts: &[(&str, String)]) -> io::Result<String> {
    for (key, _) in parts {
        let n = template.matches(key).count();
        if n != 1 {
            return Err(err(format!("{TEMPLATE}: expected exactly one {key}, found {n}")));
        }
    }
    let mut out = template.to_string();
    for (key, value) in parts {
        out = out.replacen(key, value, 1);
    }
    Ok(out)
}

fn render(config: &str, vert: &str, frag: &str, template: &str) -> io::Result<String> {
    let cfg = Config::parse(config)?;

    let frag_full = splice_prelude(frag, &cfg.glsl_prelude())?;
    let frag_min = compact_glsl(&frag_full);
    let vert_min = compact_glsl(vert);

    let declared = uniform_names(&frag_min)?;
    check_uniform_parity(&declared, &uniforms_used_by_js(template))?;

    let uniforms = declared.iter().map(|n| format!("'{n}'")).collect::<Vec<_>>().join(", ");
    let body = fill_template(
        template,
        &[
            ("__CONFIG__", cfg.js_config()),
            ("__UNIFORMS__", format!("[{uniforms}]")),
            ("__VERT__", js_template_literal(&vert_min, VERT)?),
            ("__FRAG__", js_template_literal(&frag_min, FRAG)?),
        ],
    )?;

    let banner = format!(
        "// Generated by src/leaves.rs from {CONFIG}, {VERT}, {FRAG} and {TEMPLATE}. Do not edit.\n"
    );
    Ok(banner + &body)
}

pub fn build() -> io::Result<String> {
    let config = read(CONFIG)?;
    let vert = read(VERT)?;
    let frag = read(FRAG)?;
    let template = read(TEMPLATE)?;

    render(&config, &vert, &frag, &template)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
opacity = 1.0
[render]
max_dpr = 1.25
max_fps = 60
[wind]
direction = [3.0, 4.0]
speed = 34.0
[[layers]]
cell = 100.0
size = 10.0
density = 0.5
speed = 1.0
blur = 1.0
opacity = 0.3
"#;

    #[test]
    fn glsl_floats_always_have_a_point_or_exponent() {
        for v in [0.0, 1.0, 120.0, 0.45, 1e-7, 1e21] {
            let s = glsl_float(v);
            assert!(s.contains('.') || s.contains('e'), "{s}");
        }
    }

    #[test]
    fn prelude_normalises_wind_and_sizes_arrays() {
        let p = Config::parse(GOOD).unwrap().glsl_prelude();
        assert!(p.contains("const int LAYERS = 1;"));
        assert!(p.contains("const float CELL[LAYERS] = float[LAYERS](100.0);"));
        assert!(p.contains("const vec2 WIND = vec2(0.6, 0.8);"), "{p}");
    }

    #[test]
    fn oversized_leaves_are_rejected() {
        let bad = GOOD.replace("size = 10.0", "size = 25.0");
        let e = Config::parse(&bad).unwrap_err().to_string();
        assert!(e.contains("2x2"), "{e}");
    }

    #[test]
    fn typos_in_the_config_are_rejected() {
        let bad = GOOD.replace("density", "densty");
        assert!(Config::parse(&bad).is_err());
    }

    #[test]
    fn comments_and_blank_lines_are_stripped() {
        let src = "#version 300 es\n// line\nfloat a; /* block\nspanning */ float b;\n\n   float c; // tail\n";
        let out = compact_glsl(src);
        assert_eq!(out, "#version 300 es\nfloat a;\nfloat b;\nfloat c;");
    }

    #[test]
    fn uniforms_are_listed_with_array_suffixes_removed() {
        let src = "uniform vec2 uSize;\nuniform vec3 uTop[4];\nconst float X = 1.0;";
        assert_eq!(uniform_names(src).unwrap(), ["uSize", "uTop"]);
        assert!(uniform_names("uniform float a, b;").is_err());
    }

    #[test]
    fn uniform_parity_catches_both_directions() {
        let declared = vec!["uA".to_string(), "uB".to_string()];
        assert!(check_uniform_parity(&declared, &["uA".into(), "uB".into()]).is_ok());
        assert!(check_uniform_parity(&declared, &["uA".into()]).is_err());
        assert!(check_uniform_parity(&declared, &["uA".into(), "uB".into(), "uC".into()]).is_err());
    }

    #[test]
    fn js_uniform_scan_ignores_lookalikes() {
        let js = "gl.uniform1f(loc.uTime, t); block.x; loc.uTime; loc.<name>";
        assert_eq!(uniforms_used_by_js(js), ["uTime"]);
    }

    #[test]
    fn shipped_files_render() {
        let js = render(
            &read(CONFIG).unwrap(),
            &read(VERT).unwrap(),
            &read(FRAG).unwrap(),
            &read(TEMPLATE).unwrap(),
        )
        .unwrap();
        for key in ["__CONFIG__", "__UNIFORMS__", "__VERT__", "__FRAG__"] {
            assert!(!js.contains(key), "unfilled {key}");
        }
        assert!(js.contains("`#version 300 es\n"));
        assert!(!js.contains("#pragma"));
    }
}