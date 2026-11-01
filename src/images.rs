use std::fs;
use std::io;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader};

pub const SOURCE_DIR: &str = "images";
const CACHE_ROOT: &str = ".cache/images";
const WIDTHS: [u32; 4] = [480, 800, 1200, 1800];
const QUALITY: f32 = 82.0;
const FALLBACK_MIN_WIDTH: u32 = 800;

#[derive(Debug, Default, Clone)]
pub struct Image {
    pub stem: String,
    pub widths: Vec<u32>,
    pub dims: Option<(u32, u32)>,
}

impl Image {
    pub fn url(&self, root: &str, width: u32) -> String {
        format!("{root}images/{}", variant_name(&self.stem, width))
    }

    pub fn largest(&self) -> u32 {
        self.widths.last().copied().unwrap_or(0)
    }

    pub fn full_url(&self, root: &str) -> String {
        self.url(root, self.largest())
    }

    pub fn fallback_url(&self, root: &str) -> String {
        let w = self
            .widths
            .iter()
            .copied()
            .find(|&w| w >= FALLBACK_MIN_WIDTH)
            .unwrap_or_else(|| self.largest());
        self.url(root, w)
    }

    pub fn srcset(&self, root: &str) -> String {
        self.widths
            .iter()
            .map(|&w| format!("{} {w}w", self.url(root, w)))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub fn variant_name(stem: &str, width: u32) -> String {
    format!("{stem}-{width}.webp")
}

fn cache_dir() -> PathBuf {
    Path::new(CACHE_ROOT).join(format!("q{}", QUALITY as u32))
}

fn cache_path(stem: &str, width: u32) -> PathBuf {
    cache_dir().join(variant_name(stem, width))
}

fn io_err(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

fn modified(path: &Path) -> io::Result<SystemTime> {
    fs::metadata(path)?.modified()
}

pub fn build(file: &Path) -> io::Result<Image> {
    build_inner(file).map_err(|e| io::Error::other(format!("image '{}': {e}", file.display())))
}

fn build_inner(file: &Path) -> io::Result<Image> {
    let stem = stem_of(file)?;
    let src = Path::new(SOURCE_DIR).join(file);
    let src_time = modified(&src)?;
    fs::create_dir_all(cache_dir())?;

    let mut decoder = ImageReader::open(&src)?
        .with_guessed_format()?
        .into_decoder()
        .map_err(io_err)?;
    let orientation = decoder.orientation().map_err(io_err)?;
    let (mut ow, mut oh) = decoder.dimensions();
    if matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    ) {
        std::mem::swap(&mut ow, &mut oh);
    }
    if ow == 0 || oh == 0 {
        return Err(io_err("image has zero width or height"));
    }

    let max = *WIDTHS.last().unwrap();
    let mut widths: Vec<u32> = WIDTHS.iter().copied().filter(|&w| w < ow).collect();
    widths.push(ow.min(max));
    widths.dedup();

    let stale: Vec<u32> = widths
        .iter()
        .copied()
        .filter(|&w| !modified(&cache_path(&stem, w)).is_ok_and(|t| t >= src_time))
        .collect();

    if !stale.is_empty() {
        let mut img = DynamicImage::from_decoder(decoder).map_err(io_err)?;
        img.apply_orientation(orientation);

        for w in stale {
            let h = (w as f64 * oh as f64 / ow as f64).round().max(1.0) as u32;
            let resized = if w == ow {
                img.clone()
            } else {
                img.resize_exact(w, h, FilterType::Lanczos3)
            };
            // The webp crate only accepts RGB8 / RGBA8.
            let rgb = DynamicImage::ImageRgb8(resized.to_rgb8());
            let bytes = webp::Encoder::from_image(&rgb)
                .map_err(io_err)?
                .encode(QUALITY);
            fs::write(cache_path(&stem, w), &*bytes)?;
        }
    }

    let largest = *widths.last().unwrap();
    let dims = imagesize::size(cache_path(&stem, largest))
        .ok()
        .map(|s| (s.width as u32, s.height as u32));

    Ok(Image { stem, widths, dims })
}

fn stem_of(file: &Path) -> io::Result<String> {
    if file.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(io_err("path must be relative to images/ with no '..' or '.'"));
    }
    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io_err("file name is not valid UTF-8"))?;
    if stem.is_empty()
        || !stem.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(io_err("file stem may only use A-Z, a-z, 0-9, '-' and '_'"));
    }
    Ok(stem.to_owned())
}

pub fn publish(img: &Image, out_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(out_dir)?;
    for &w in &img.widths {
        fs::copy(
            cache_path(&img.stem, w),
            out_dir.join(variant_name(&img.stem, w)),
        )?;
    }
    Ok(())
}
