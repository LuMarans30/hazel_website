use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::{Display, Write as _};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const SCRIPT_TEMPLATE: &str = "assets/scripts/theme.js";
const SPEC_MARKER: &str = "__SPEC__";

const FAVICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E\
     %3Crect width='32' height='32' rx='9' fill='%23{light}'/%3E\
     %3Cpath d='M0 20c5-6 8-6 13 0s8 6 13 0 4-3 6-2v14H0z' fill='%23{dark}'/%3E%3C/svg%3E";

fn err(msg: impl Display) -> io::Error {
    io::Error::other(msg.to_string())
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(try_from = "String")]
struct Rgb([f64; 3]);

impl TryFrom<String> for Rgb {
    type Error = String;

    fn try_from(s: String) -> Result<Self, String> {
        let h = s
            .strip_prefix('#')
            .filter(|h| h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| format!("expected a color like \"#1f6b45\", got \"{s}\""))?;
        let ch = |i: usize| f64::from(u8::from_str_radix(&h[i..i + 2], 16).unwrap());
        Ok(Rgb([ch(0), ch(2), ch(4)]))
    }
}

impl Rgb {
    fn lerp(self, other: Rgb, w: f64) -> Rgb {
        Rgb(std::array::from_fn(|i| {
            self.0[i] + (other.0[i] - self.0[i]) * w
        }))
    }

    fn channels(self) -> [u8; 3] {
        self.0.map(|v| v.round() as u8)
    }

    fn digits(self) -> String {
        let [r, g, b] = self.channels();
        format!("{r:02x}{g:02x}{b:02x}")
    }

    fn hex(self) -> String {
        format!("#{}", self.digits())
    }

    fn triplet(self) -> String {
        let [r, g, b] = self.channels();
        format!("{r} {g} {b}")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Leaves {
    top: [Rgb; 4],
    bot: [Rgb; 4],
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scheme {
    bg: Rgb,
    text: Rgb,
    muted: Rgb,
    accent: Rgb,
    btn: Rgb,
    btn_fg: Rgb,
    h1_a: Rgb,
    h1_b: Rgb,
    ink: Rgb,
    placeholder: Rgb,
    blobs: [Rgb; 4],
    leaves: Leaves,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Lightbox {
    glow_1: Rgb,
    glow_2: Rgb,
    base: Rgb,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Season {
    light: Scheme,
    dark: Scheme,
    lightbox: Lightbox,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Alpha {
    blobs: [f64; 4],
    pill: f64,
    placeholder: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PerScheme<T> {
    light: T,
    dark: T,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    peaks: [u32; 4],
    alpha: PerScheme<Alpha>,
    fixed: PerScheme<BTreeMap<String, String>>,
    spring: Season,
    summer: Season,
    autumn: Season,
    winter: Season,
}

impl ThemeFile {
    fn season(&self, i: usize) -> &Season {
        match i {
            0 => &self.spring,
            1 => &self.summer,
            2 => &self.autumn,
            _ => &self.winter,
        }
    }
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn blend_for_day(day: i64, peaks: [u32; 4]) -> (usize, usize, f64) {
    let peaks = peaks.map(i64::from);
    let mut n = (day - 1).rem_euclid(365) + 1;
    if n < peaks[0] {
        n += 365;
    }
    let mut i = 0;
    while i < 3 && n >= peaks[i + 1] {
        i += 1;
    }
    let start = peaks[i];
    let end = if i < 3 { peaks[i + 1] } else { peaks[0] + 365 };
    let u = (n - start) as f64 / (end - start) as f64;
    (i, (i + 1) % 4, u * u * (3.0 - 2.0 * u))
}

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub css: String,
    pub script: String,
    pub theme_color_light: String,
    pub theme_color_dark: String,
    pub favicon: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Fmt {
    Hex,
    Triplet,
    Alpha(String),
}

impl Fmt {
    fn apply(&self, c: Rgb) -> String {
        match self {
            Fmt::Hex => c.hex(),
            Fmt::Triplet => c.triplet(),
            Fmt::Alpha(a) => format!("rgb({} / {a})", c.triplet()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Decl {
    Lit(String, String),
    Color(String, usize, Fmt),
}

#[derive(Debug, Default)]
struct Vars {
    decls: Vec<Decl>,
    colors: Vec<Rgb>,
}

impl Vars {
    fn lit(&mut self, name: &str, value: &str) {
        self.decls.push(Decl::Lit(name.into(), value.into()));
    }

    fn color(&mut self, name: &str, c: Rgb, fmt: Fmt) {
        self.decls.push(Decl::Color(name.into(), self.colors.len(), fmt));
        self.colors.push(c);
    }
}

fn scheme_vars(s: &Scheme, alpha: &Alpha, fixed: &BTreeMap<String, String>) -> Vars {
    let mut v = Vars::default();
    for (name, value) in fixed {
        v.lit(name, value);
    }
    v.color("bg", s.bg, Fmt::Hex);
    v.color("text", s.text, Fmt::Hex);
    v.color("muted", s.muted, Fmt::Hex);
    v.color("accent", s.accent, Fmt::Hex);
    v.color("btn-bg", s.btn, Fmt::Hex);
    v.color("btn-fg", s.btn_fg, Fmt::Hex);
    v.color("h1-a", s.h1_a, Fmt::Hex);
    v.color("h1-b", s.h1_b, Fmt::Hex);
    v.color("ink", s.ink, Fmt::Triplet);
    v.color("pill", s.accent, Fmt::Alpha(alpha.pill.to_string()));
    v.color("placeholder", s.placeholder, Fmt::Alpha(alpha.placeholder.to_string()));
    for (i, (c, a)) in s.blobs.iter().zip(alpha.blobs).enumerate() {
        v.color(&format!("blob-{}", i + 1), *c, Fmt::Alpha(a.to_string()));
    }
    for (i, (top, bot)) in s.leaves.top.iter().zip(&s.leaves.bot).enumerate() {
        v.color(&format!("leaf-top-{}", i + 1), *top, Fmt::Hex);
        v.color(&format!("leaf-bot-{}", i + 1), *bot, Fmt::Hex);
    }
    v
}

fn light_vars(file: &ThemeFile, s: &Season) -> Vars {
    let mut v = scheme_vars(&s.light, &file.alpha.light, &file.fixed.light);
    v.color("lb-1", s.lightbox.glow_1, Fmt::Triplet);
    v.color("lb-2", s.lightbox.glow_2, Fmt::Triplet);
    v.color("lb-base", s.lightbox.base, Fmt::Triplet);
    v
}

fn dark_vars(file: &ThemeFile, s: &Season) -> Vars {
    scheme_vars(&s.dark, &file.alpha.dark, &file.fixed.dark)
}

struct Palette {
    decls: Vec<Decl>,
    seasons: Vec<Vec<Rgb>>,
}

impl Palette {
    fn new(file: &ThemeFile, vars: impl Fn(&ThemeFile, &Season) -> Vars) -> Self {
        let all: Vec<Vars> = (0..4).map(|i| vars(file, file.season(i))).collect();
        debug_assert!(all.iter().all(|v| v.decls == all[0].decls));
        Palette {
            decls: all[0].decls.clone(),
            seasons: all.iter().map(|v| v.colors.clone()).collect(),
        }
    }

    fn blend(&self, from: usize, to: usize, w: f64) -> Vec<Rgb> {
        self.seasons[from]
            .iter()
            .zip(&self.seasons[to])
            .map(|(a, b)| a.lerp(*b, w))
            .collect()
    }

    fn css(&self, colors: &[Rgb]) -> String {
        let mut out = String::new();
        for d in &self.decls {
            let _ = match d {
                Decl::Lit(name, value) => write!(out, "--{name}:{value};"),
                Decl::Color(name, slot, fmt) => write!(out, "--{name}:{};", fmt.apply(colors[*slot])),
            };
        }
        out
    }

    fn get(&self, name: &str, colors: &[Rgb]) -> io::Result<Rgb> {
        self.decls
            .iter()
            .find_map(|d| match d {
                Decl::Color(n, slot, _) if n == name => Some(colors[*slot]),
                _ => None,
            })
            .ok_or_else(|| err(format!("theme has no color variable '--{name}'")))
    }

    fn json(&self) -> String {
        let mut s = String::from("{\"decls\":[");
        for (i, d) in self.decls.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let _ = match d {
                Decl::Lit(name, value) => write!(s, "[{},{}]", json_str(name), json_str(value)),
                Decl::Color(name, slot, Fmt::Hex) => write!(s, "[{},{slot},\"h\"]", json_str(name)),
                Decl::Color(name, slot, Fmt::Triplet) => write!(s, "[{},{slot},\"t\"]", json_str(name)),
                Decl::Color(name, slot, Fmt::Alpha(a)) => {
                    write!(s, "[{},{slot},\"a\",{}]", json_str(name), json_str(a))
                }
            };
        }
        s.push_str("],\"seasons\":[");
        for (i, colors) in self.seasons.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let packed: String = colors.iter().map(|c| c.digits()).collect();
            let _ = write!(s, "\"{packed}\"");
        }
        s.push_str("]}");
        s
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn favicon(light_accent: Rgb, dark_accent: Rgb) -> String {
    FAVICON
        .replace("{light}", &light_accent.digits())
        .replace("{dark}", &dark_accent.digits())
}

fn fill_script(template: &str, spec: &str) -> io::Result<String> {
    let n = template.matches(SPEC_MARKER).count();
    if n != 1 {
        return Err(err(format!("{SCRIPT_TEMPLATE}: expected exactly one {SPEC_MARKER}, found {n}")));
    }
    Ok(template.replacen(SPEC_MARKER, spec, 1))
}

fn today() -> io::Result<i64> {
    let days = (SystemTime::now().duration_since(UNIX_EPOCH).map_err(err)?.as_secs() / 86_400) as i64;
    let (y, _, _) = civil_from_days(days);
    Ok(days - days_from_civil(y, 1, 1) + 1)
}

impl Theme {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|e| err(format!("Failed to read theme at '{}': {e}", path.display())))?;
        let template = fs::read_to_string(SCRIPT_TEMPLATE)
            .map_err(|e| err(format!("Failed to read '{SCRIPT_TEMPLATE}': {e}")))?;
        Self::resolve(&text, &template, today()?)
            .map_err(|e| err(format!("{} (in '{}')", e, path.display())))
    }

    fn resolve(toml_text: &str, template: &str, day: i64) -> io::Result<Self> {
        let file: ThemeFile =
            toml::from_str(toml_text).map_err(|e| err(format!("Failed to parse theme: {e}")))?;

        let light = Palette::new(&file, light_vars);
        let dark = Palette::new(&file, dark_vars);

        let (from, to, w) = blend_for_day(day, file.peaks);
        let lc = light.blend(from, to, w);
        let dc = dark.blend(from, to, w);

        let mut css = String::from(":root{");
        css.push_str(&light.css(&lc));
        css.push_str("}@media (prefers-color-scheme:dark){:root{");
        css.push_str(&dark.css(&dc));
        css.push_str("}}");

        let spec = format!(
            "{{\"peaks\":{:?},\"favicon\":{},\"light\":{},\"dark\":{}}}",
            file.peaks,
            json_str(FAVICON),
            light.json(),
            dark.json(),
        );

        Ok(Theme {
            css,
            script: fill_script(template, &spec)?,
            theme_color_light: light.get("bg", &lc)?.hex(),
            theme_color_dark: dark.get("bg", &dc)?.hex(),
            favicon: favicon(light.get("accent", &lc)?, dark.get("accent", &dc)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEAKS: [u32; 4] = [115, 206, 298, 385];

    #[test]
    fn calendar_round_trips_across_both_century_rules() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        for z in days_from_civil(1900, 1, 1)..days_from_civil(2101, 1, 1) {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z, "round-trip failed at {y}-{m}-{d}");
        }
        // 1900 is divisible by 4 but not a leap year; 2000 is.
        assert_eq!(days_from_civil(1900, 3, 1) - days_from_civil(1900, 2, 28), 1);
        assert_eq!(days_from_civil(2000, 3, 1) - days_from_civil(2000, 2, 28), 2);
    }

    #[test]
    fn peak_day_is_pure_season() {
        assert_eq!(blend_for_day(115, PEAKS), (0, 1, 0.0));
    }

    #[test]
    fn segment_midpoint_is_half_blend() {
        let peaks = [100, 200, 300, 400];
        assert_eq!(blend_for_day(150, peaks), (0, 1, 0.5));
        assert_eq!(blend_for_day(350, peaks), (2, 3, 0.5));
    }

    #[test]
    fn new_year_wraps_into_autumn_to_winter() {
        let (from, to, w) = blend_for_day(1, PEAKS);
        assert_eq!((from, to), (2, 3));
        // Smoothstepped, not linear
        assert!((0.8..0.9).contains(&w), "w = {w}");
        // Leap-year Dec 31 (ordinal 366) folds onto Jan 1.
        assert_eq!(blend_for_day(366, PEAKS), blend_for_day(1, PEAKS));
    }

    #[test]
    fn day_before_first_peak_wraps_to_spring() {
        let (from, to, w) = blend_for_day(114, PEAKS);
        assert_eq!((from, to), (3, 0)); // winter -> spring
        assert!(w > 0.98, "w = {w}"); // one day short of the spring peak
    }

    #[test]
    fn every_day_yields_bounded_weight() {
        for day in 1..=365 {
            let (from, to, w) = blend_for_day(day, PEAKS);
            assert!((0.0..=1.0).contains(&w), "day {day}: w = {w}");
            assert_eq!(to, (from + 1) % 4);
        }
    }

    const TEMPLATE: &str = "const SPEC = __SPEC__;";

    fn shipped(name: &str) -> String {
        fs::read_to_string(name).unwrap_or_else(|e| panic!("{name}: {e}"))
    }

    #[test]
    fn declarations_do_not_depend_on_the_season() {
        let file: ThemeFile = toml::from_str(&shipped("theme.toml")).unwrap();
        let light = Palette::new(&file, light_vars);
        let dark = Palette::new(&file, dark_vars);
        for p in [&light, &dark] {
            assert!(p.seasons.iter().all(|s| s.len() == p.seasons[0].len()));
            assert!(
                p.decls.iter().filter(|d| matches!(d, Decl::Color(..))).count() == p.seasons[0].len()
            );
        }
    }

    #[test]
    fn json_strings_cannot_end_the_script() {
        let s = json_str("a\"b\\c</script><!--\n");
        assert!(!s.contains('<') && !s.contains('\n'), "{s}");
        assert_eq!(s, "\"a\\\"b\\\\c\\u003c/script>\\u003c!--\\u000a\"");
    }

    #[test]
    fn template_needs_exactly_one_marker() {
        let toml_text = shipped("theme.toml");
        assert!(Theme::resolve(&toml_text, "no marker", 1).is_err());
        assert!(Theme::resolve(&toml_text, "__SPEC__ __SPEC__", 1).is_err());
        let ok = Theme::resolve(&toml_text, TEMPLATE, 1).unwrap();
        assert!(ok.script.starts_with("const SPEC = {\"peaks\":[115, 206, 298, 385]"));
    }

    #[test]
    fn browser_script_matches_rust_on_every_day() {
        use std::process::Command;

        const HARNESS: &str = r#"
const fs = require('fs');
const src = fs.readFileSync(process.argv[2], 'utf8');
const RealDate = Date;
let out = '';
for (let day = 1; day <= 366; day++) {
  const node = (o) => ({ ...o, getAttribute(k) { return this[k] ?? null; }, setAttribute(k, v) { this[k] = v; } });
  const style = { textContent: 'stale' };
  const metas = [
    node({ media: '(prefers-color-scheme: light)', content: '' }),
    node({ media: '(prefers-color-scheme: dark)', content: '' }),
  ];
  const icon = node({ href: '' });
  const doc = {
    getElementById: () => style,
    querySelectorAll: () => metas,
    querySelector: () => icon,
  };
  class FakeDate extends RealDate {
    constructor(...a) { if (a.length) super(...a); else super(2024, 0, day); }
  }
  new Function('document', 'Date', src)(doc, FakeDate);
  out += [style.textContent, metas[0].content, metas[1].content, icon.href].join('\n') + '\n\n';
}
process.stdout.write(out);
"#;

        let toml_text = shipped("theme.toml");
        let template = shipped(SCRIPT_TEMPLATE);

        let dir = std::env::temp_dir().join("hazel_theme_parity");
        fs::create_dir_all(&dir).unwrap();
        let script = dir.join("theme.js");
        let harness = dir.join("harness.js");
        let first = Theme::resolve(&toml_text, &template, 1).unwrap();
        fs::write(&script, &first.script).unwrap();
        fs::write(&harness, HARNESS).unwrap();

        let out = match Command::new("node").arg(&harness).arg(&script).output() {
            Ok(o) => o,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                eprintln!("skipping: node is not installed");
                return;
            }
            Err(e) => panic!("failed to run node: {e}"),
        };
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let got = String::from_utf8(out.stdout).unwrap();

        let mut want = String::new();
        for day in 1..=366 {
            let t = Theme::resolve(&toml_text, &template, day).unwrap();
            let _ = write!(want, "{}\n{}\n{}\n{}\n\n", t.css, t.theme_color_light, t.theme_color_dark, t.favicon);
        }
        for (i, (g, w)) in got.split("\n\n").zip(want.split("\n\n")).enumerate() {
            assert_eq!(g, w, "script and Rust disagree on day {}", i + 1);
        }
        assert_eq!(got.len(), want.len());
    }
}