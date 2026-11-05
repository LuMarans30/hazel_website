use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::{Display, Write as _};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const NAMES: [&str; 4] = ["spring", "summer", "autumn", "winter"];

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

    fn rgba(self, alpha: f64) -> String {
        format!("rgb({} / {alpha})", self.triplet())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hills {
    top: [Rgb; 4],
    bot: [Rgb; 4],
}

impl Hills {
    fn lerp(&self, o: &Hills, w: f64) -> Hills {
        Hills {
            top: std::array::from_fn(|i| self.top[i].lerp(o.top[i], w)),
            bot: std::array::from_fn(|i| self.bot[i].lerp(o.bot[i], w)),
        }
    }
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
    hills: Hills,
}

impl Scheme {
    fn lerp(&self, o: &Scheme, w: f64) -> Scheme {
        let l = |a: Rgb, b: Rgb| a.lerp(b, w);
        Scheme {
            bg: l(self.bg, o.bg),
            text: l(self.text, o.text),
            muted: l(self.muted, o.muted),
            accent: l(self.accent, o.accent),
            btn: l(self.btn, o.btn),
            btn_fg: l(self.btn_fg, o.btn_fg),
            h1_a: l(self.h1_a, o.h1_a),
            h1_b: l(self.h1_b, o.h1_b),
            ink: l(self.ink, o.ink),
            placeholder: l(self.placeholder, o.placeholder),
            blobs: std::array::from_fn(|i| l(self.blobs[i], o.blobs[i])),
            hills: self.hills.lerp(&o.hills, w),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Lightbox {
    glow_1: Rgb,
    glow_2: Rgb,
    base: Rgb,
}

impl Lightbox {
    fn lerp(&self, o: &Lightbox, w: f64) -> Lightbox {
        Lightbox {
            glow_1: self.glow_1.lerp(o.glow_1, w),
            glow_2: self.glow_2.lerp(o.glow_2, w),
            base: self.base.lerp(o.base, w),
        }
    }
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

#[derive(Debug, Clone, PartialEq)]
enum Pick {
    Day(i64),
    Season(usize),
}

impl Pick {
    fn from_env() -> io::Result<Pick> {
        if let Ok(s) = std::env::var("SITE_SEASON") {
            let s = s.trim().to_lowercase();
            return NAMES
                .iter()
                .position(|n| *n == s)
                .map(Pick::Season)
                .ok_or_else(|| err(format!("SITE_SEASON must be one of {NAMES:?}, got '{s}'")));
        }
        if let Ok(s) = std::env::var("SITE_DATE") {
            let (y, m, d) = parse_date(s.trim())?;
            return Ok(Pick::Day(ordinal(y, m, d)));
        }
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_secs();
        let (y, m, d) = civil_from_days((secs / 86_400) as i64);
        Ok(Pick::Day(ordinal(y, m, d)))
    }
}

fn parse_date(s: &str) -> io::Result<(i64, u32, u32)> {
    let bad = || err(format!("SITE_DATE must be YYYY-MM-DD, got '{s}'"));
    let mut it = s.split('-');
    let y: i64 = it.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
    let m: u32 = it.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
    let d: u32 = it.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(bad());
    }
    if civil_from_days(days_from_civil(y, m, d)) != (y, m, d) {
        return Err(bad());
    }
    Ok((y, m, d))
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

fn ordinal(y: i64, m: u32, d: u32) -> i64 {
    days_from_civil(y, m, d) - days_from_civil(y, 1, 1) + 1
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
    pub theme_color_light: String,
    pub theme_color_dark: String,
    pub favicon: String,
    pub summary: String,
}

fn decl(out: &mut String, name: &str, value: impl Display) {
    let _ = write!(out, "--{name}:{value};");
}

fn scheme_vars(out: &mut String, s: &Scheme, alpha: &Alpha, fixed: &BTreeMap<String, String>) {
    for (name, value) in fixed {
        decl(out, name, value);
    }
    decl(out, "bg", s.bg.hex());
    decl(out, "text", s.text.hex());
    decl(out, "muted", s.muted.hex());
    decl(out, "accent", s.accent.hex());
    decl(out, "btn-bg", s.btn.hex());
    decl(out, "btn-fg", s.btn_fg.hex());
    decl(out, "h1-a", s.h1_a.hex());
    decl(out, "h1-b", s.h1_b.hex());
    decl(out, "ink", s.ink.triplet());
    decl(out, "pill", s.accent.rgba(alpha.pill));
    decl(out, "placeholder", s.placeholder.rgba(alpha.placeholder));
    for (i, (c, a)) in s.blobs.iter().zip(alpha.blobs).enumerate() {
        decl(out, &format!("blob-{}", i + 1), c.rgba(a));
    }
    for (i, (top, bot)) in s.hills.top.iter().zip(&s.hills.bot).enumerate() {
        decl(out, &format!("hill-top-{}", i + 1), top.hex());
        decl(out, &format!("hill-bot-{}", i + 1), bot.hex());
    }
}

fn favicon(light_accent: Rgb, dark_accent: Rgb) -> String {
    format!(
        "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E\
         %3Crect width='32' height='32' rx='9' fill='%23{}'/%3E\
         %3Cpath d='M0 20c5-6 8-6 13 0s8 6 13 0 4-3 6-2v14H0z' fill='%23{}'/%3E%3C/svg%3E",
        light_accent.digits(),
        dark_accent.digits(),
    )
}

impl Theme {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|e| err(format!("Failed to read theme at '{}': {e}", path.display())))?;
        Self::resolve(&text, Pick::from_env()?)
            .map_err(|e| err(format!("{} (in '{}')", e, path.display())))
    }

    fn resolve(toml_text: &str, pick: Pick) -> io::Result<Self> {
        let file: ThemeFile =
            toml::from_str(toml_text).map_err(|e| err(format!("Failed to parse theme: {e}")))?;

        let day = match pick {
            Pick::Day(n) => n,
            Pick::Season(i) => i64::from(file.peaks[i]),
        };
        let (from, to, w) = blend_for_day(day, file.peaks);
        let (a, b) = (file.season(from), file.season(to));

        let light = a.light.lerp(&b.light, w);
        let dark = a.dark.lerp(&b.dark, w);
        let lb = a.lightbox.lerp(&b.lightbox, w);

        let mut css = String::from(":root{");
        scheme_vars(&mut css, &light, &file.alpha.light, &file.fixed.light);
        decl(&mut css, "lb-1", lb.glow_1.triplet());
        decl(&mut css, "lb-2", lb.glow_2.triplet());
        decl(&mut css, "lb-base", lb.base.triplet());
        css.push_str("}@media (prefers-color-scheme:dark){:root{");
        scheme_vars(&mut css, &dark, &file.alpha.dark, &file.fixed.dark);
        css.push_str("}}");

        Ok(Theme {
            css,
            theme_color_light: light.bg.hex(),
            theme_color_dark: dark.bg.hex(),
            favicon: favicon(light.accent, dark.accent),
            summary: format!(
                "{} -> {} at {:.0}% (day {day})",
                NAMES[from],
                NAMES[to],
                w * 100.0
            ),
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
    fn parse_date_rejects_impossible_dates() {
        assert!(matches!(parse_date("2024-02-29"), Ok((2024, 2, 29))));
        assert!(parse_date("2023-02-29").is_err()); // not a leap year
        assert!(parse_date("2024-02-30").is_err());
        assert!(parse_date("2024-13-01").is_err());
        assert!(parse_date("2024-02").is_err());
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
}