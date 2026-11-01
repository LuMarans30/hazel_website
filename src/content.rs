use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;

const IMAGE_DIR: &str = "static/images";
const FALLBACK_RATIO: f32 = 1.25;

#[derive(Debug, Deserialize)]
pub struct Painting {
    pub file: String,
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
    pub dims: Option<(u32, u32)>,
    #[serde(skip)]
    pub slug: String,
}

impl Painting {
    pub fn alt_text(&self) -> &str {
        self.alt.as_deref().unwrap_or(&self.title)
    }

    pub fn ratio(&self) -> f32 {
        match self.dims {
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
    pub portrait: Option<String>,
    #[serde(default)]
    pub portrait_alt: Option<String>,
    #[serde(skip)]
    pub portrait_dims: Option<(u32, u32)>,
    pub paintings: Vec<Painting>,
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

        content.portrait_dims = content.portrait.as_deref().and_then(measure);
        content.prepare_paintings();
        Ok(content)
    }

    fn prepare_paintings(&mut self) {
        let mut seen = HashSet::new();

        for (i, p) in self.paintings.iter_mut().enumerate() {
            p.dims = measure(&p.file);

            let mut slug = slugify(&p.title);
            if slug.is_empty() || !seen.insert(slug.clone()) {
                slug = format!("painting-{}", i + 1);
                seen.insert(slug.clone());
            }
            p.slug = slug;
        }
    }
}

fn measure(file: &str) -> Option<(u32, u32)> {
    let path = Path::new(IMAGE_DIR).join(file);
    match imagesize::size(&path) {
        Ok(s) => Some((s.width as u32, s.height as u32)),
        Err(e) => {
            eprintln!("warning: can't read size of '{}': {e}", path.display());
            None
        }
    }
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}
