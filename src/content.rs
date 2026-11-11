use rayon::prelude::*;
use serde::Deserialize;
use serde::Deserializer;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::images::{self, Image};
use crate::theme::Theme;

const FALLBACK_RATIO: f32 = 1.25;

const RESERVED_IDS: [&str; 6] = [
    "main",
    "lightbox",
    "lightbox-img",
    "lightbox-title",
    "lightbox-details",
    "lightbox-count",
];

#[derive(Debug, Deserialize)]
pub struct Painting {
    pub file: PathBuf,
    pub title: String,
    #[serde(default, deserialize_with = "lenient_year")]
    pub year: Option<u16>,
    #[serde(default)]
    pub medium: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub alt: Option<String>,
    #[serde(default)]
    pub is_featured: bool,
    #[serde(skip)]
    pub image: Image,
    #[serde(skip)]
    pub slug: String,
}

fn lenient_year<'de, D>(d: D) -> Result<Option<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Num {
        Int(i64),
        Float(f64),
    }

    match Option::<Num>::deserialize(d)? {
        None => Ok(None),
        Some(Num::Int(n)) => u16::try_from(n)
            .map(Some)
            .map_err(|_| serde::de::Error::custom(format!("year out of range: {n}"))),
        Some(Num::Float(f)) if f.fract() == 0.0 && (0.0..=65535.0).contains(&f) => {
            Ok(Some(f as u16))
        }
        Some(Num::Float(f)) => Err(serde::de::Error::custom(format!("invalid year: {f}"))),
    }
}

impl Painting {
    pub fn alt_text(&self) -> &str {
        self.alt.as_deref().unwrap_or(&self.title)
    }

    pub fn ratio(&self) -> f32 {
        match self.image.dims {
            Some((w, h)) if h > 0 => w as f32 / h as f32,
            _ => FALLBACK_RATIO,
        }
    }

    pub fn details(&self) -> String {
        let year = self.year.map(|y| y.to_string());
        [
            year.as_deref(),
            self.medium.as_deref(),
            self.size.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ")
    }
}

#[derive(Debug, Deserialize)]
pub struct SiteContent {
    pub artist_name: String,
    pub tagline: String,
    pub bio: Vec<String>,
    pub email: String,
    #[serde(default)]
    pub site_url: Option<String>,
    #[serde(default)]
    pub portrait: Option<PathBuf>,
    #[serde(default)]
    pub portrait_alt: Option<String>,
    #[serde(skip)]
    pub portrait_image: Option<Image>,
    pub paintings: Vec<Painting>,
    #[serde(default)]
    pub goatcounter_code: Option<String>,
    #[serde(default)]
    pub social_image: Option<PathBuf>,
    #[serde(skip)]
    pub theme: Theme,
}

impl SiteContent {
    pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let content_str = fs::read_to_string(&path).map_err(|e| {
            io::Error::other(format!(
                "Failed to read content file at '{}': {e}",
                path.as_ref().display()
            ))
        })?;

        let mut content: Self = toml::from_str(&content_str)
            .map_err(|e| io::Error::other(format!("Failed to parse TOML: {e}")))?;

        content.prepare()?;
        Ok(content)
    }

    fn prepare(&mut self) -> io::Result<()> {
        if let Some(file) = &self.social_image {
            let known = self.portrait.as_ref() == Some(file)
                || self.paintings.iter().any(|p| &p.file == file);
            if !known {
                return Err(io::Error::other(format!(
                    "social_image '{}' must match an image file",
                    file.display()
                )));
            }
        }

        if let Some(file) = &self.portrait {
            self.portrait_image = Some(images::build(file)?);
        }

        self.paintings
            .par_iter_mut()
            .try_for_each(|p| -> io::Result<()> {
                p.image = images::build(&p.file)?;
                Ok(())
            })?;

        let mut seen: HashSet<String> = RESERVED_IDS.iter().map(|s| s.to_string()).collect();

        for (i, p) in self.paintings.iter_mut().enumerate() {
            let mut base = slugify(&p.title);
            if base.is_empty() {
                base = format!("painting-{}", i + 1);
            }

            let mut slug = base.clone();
            let mut n = 2;
            while !seen.insert(slug.clone()) {
                slug = format!("{base}-{n}");
                n += 1;
            }
            p.slug = slug;
        }

        Ok(())
    }

    pub fn publish_images(&self, out_dir: &Path) -> io::Result<()> {
        if let Some(img) = &self.portrait_image {
            images::publish(img, out_dir)?;
        }
        for p in &self.paintings {
            images::publish(&p.image, out_dir)?;
        }
        Ok(())
    }

    pub fn canonical_url(&self, path: &str) -> Option<String> {
        let base = self.site_url.as_deref()?.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            Some(format!("{base}/"))
        } else {
            Some(format!("{base}/{path}"))
        }
    }

    pub fn social_card(&self) -> Option<(&Image, &str)> {
        if let Some(file) = &self.social_image {
            if let Some(p) = self.paintings.iter().find(|p| &p.file == file) {
                return Some((&p.image, p.alt_text()));
            }
            if self.portrait.as_ref() == Some(file) {
                let img = self.portrait_image.as_ref()?;
                let alt = self.portrait_alt.as_deref().unwrap_or(&self.artist_name);
                return Some((img, alt));
            }
        }
        self.paintings
            .iter()
            .find(|p| p.is_featured)
            .or_else(|| self.paintings.first())
            .map(|p| (&p.image, p.alt_text()))
    }

    pub fn social_url(&self) -> Option<String> {
        self.canonical_url(images::SOCIAL_NAME)
    }

    pub fn publish_social(&self, out_dir: &Path) -> io::Result<()> {
        match self.social_card() {
            Some((img, _)) => images::publish_social(img, out_dir),
            None => Ok(()),
        }
    }
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().flat_map(char::to_lowercase) {
        let folded = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
            'ç' => "c",
            'è' | 'é' | 'ê' | 'ë' => "e",
            'ì' | 'í' | 'î' | 'ï' => "i",
            'ñ' => "n",
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => "o",
            'ù' | 'ú' | 'û' | 'ü' => "u",
            'ý' | 'ÿ' => "y",
            'ß' => "ss",
            'æ' => "ae",
            'œ' => "oe",
            _ => "",
        };

        if !folded.is_empty() {
            out.push_str(folded);
        } else if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugify_folds_accents_and_trims_junk() {
        assert_eq!(slugify("Untitled 14"), "untitled-14");
        assert_eq!(slugify("Été à Paris"), "ete-a-paris");
        assert_eq!(slugify("Hello!"), "hello");
        assert_eq!(slugify("!!!"), ""); // caller falls back to painting-N
    }
}
