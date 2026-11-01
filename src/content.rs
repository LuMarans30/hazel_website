use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::images::{self, Image};

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
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub medium: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub alt: Option<String>,
    #[serde(default, alias = "featured")]
    pub is_featured: bool,
    #[serde(skip)]
    pub image: Image,
    #[serde(skip)]
    pub slug: String,
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
        [year.as_deref(), self.medium.as_deref(), self.size.as_deref()]
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
    pub portrait: Option<PathBuf>,
    #[serde(default)]
    pub portrait_alt: Option<String>,
    #[serde(skip)]
    pub portrait_image: Option<Image>,
    pub paintings: Vec<Painting>,
    #[serde(default)]
    pub goatcounter_code: Option<String>,
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
