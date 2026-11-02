use std::fs;
use std::io;
use std::io::BufWriter;
use std::path::Component;
use std::path::{Path, PathBuf};

use image::RgbImage;
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader, codecs::jpeg::JpegEncoder};
use moxcms::CmsError;
use moxcms::ColorProfile;
use moxcms::Layout;
use moxcms::TransformOptions;

pub const SOURCE_DIR: &str = "images";
const CACHE_ROOT: &str = ".cache/images";
const WIDTHS: [u32; 4] = [480, 800, 1200, 1800];
const QUALITY: f32 = 82.0;
const FALLBACK_MIN_WIDTH: u32 = 800;

pub const SOCIAL_NAME: &str = "social.jpg";
pub const SOCIAL_SIZE: (u32, u32) = (1200, 630);
const SOCIAL_PAD: u32 = 40;
const SOCIAL_QUALITY: u8 = 85;

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

fn to_srgb(rgb: &RgbImage, icc: &[u8]) -> Result<RgbImage, CmsError> {
    let src = ColorProfile::new_from_slice(icc)?;
    let dst = ColorProfile::new_srgb();
    let transform =
        src.create_transform_8bit(Layout::Rgb, &dst, Layout::Rgb, TransformOptions::default())?;

    let (w, h) = rgb.dimensions();
    let mut out = vec![0u8; rgb.as_raw().len()];
    transform.transform(rgb.as_raw(), &mut out)?;
    Ok(RgbImage::from_raw(w, h, out).expect("buffer length unchanged"))
}

pub fn variant_name(stem: &str, width: u32) -> String {
    format!("{stem}-{width}.webp")
}

fn cache_dir() -> PathBuf {
    Path::new(CACHE_ROOT).join(format!("v2-q{}", QUALITY as u32))
}

fn cache_path(stem: &str, width: u32) -> PathBuf {
    cache_dir().join(variant_name(stem, width))
}

fn io_err(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

pub fn build(file: &Path) -> io::Result<Image> {
    build_inner(file).map_err(|e| io::Error::other(format!("image '{}': {e}", file.display())))
}

fn build_inner(file: &Path) -> io::Result<Image> {
    let stem = stem_of(file)?;
    let src = Path::new(SOURCE_DIR).join(file);
    let src_hash = fnv1a(&fs::read(&src)?);
    fs::create_dir_all(cache_dir())?;

    let mut decoder = ImageReader::open(&src)?
        .with_guessed_format()?
        .into_decoder()
        .map_err(io_err)?;
    let orientation = decoder.orientation().map_err(io_err)?;
    let icc = decoder.icc_profile().map_err(io_err)?;
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

    let hash_path = cache_dir().join(format!("{stem}.hash"));
    let src_hash = src_hash.to_string();
    let hash_matches = fs::read_to_string(&hash_path).is_ok_and(|cached| cached == src_hash);

    let stale: Vec<u32> = if hash_matches {
        widths
            .iter()
            .copied()
            .filter(|&w| !cache_path(&stem, w).exists())
            .collect()
    } else {
        widths.clone()
    };

    if !stale.is_empty() {
        let mut img = DynamicImage::from_decoder(decoder).map_err(io_err)?;
        img.apply_orientation(orientation);

        let mut rgb = img.into_rgb8();
        if let Some(icc) = &icc {
            match to_srgb(&rgb, icc) {
                Ok(converted) => rgb = converted,
                Err(e) => eprintln!(
                    "warning: image '{}': ICC conversion skipped: {e:?}",
                    file.display()
                ),
            }
        }
        let img = DynamicImage::ImageRgb8(rgb);

        for w in stale {
            let h = (w as f64 * oh as f64 / ow as f64).round().max(1.0) as u32;
            let resized = if w == ow {
                img.clone()
            } else {
                img.resize_exact(w, h, FilterType::Lanczos3)
            };
            let bytes = webp::Encoder::from_image(&resized)
                .map_err(io_err)?
                .encode(QUALITY);
            fs::write(cache_path(&stem, w), &*bytes)?;
        }

        if !hash_matches {
            fs::write(&hash_path, &src_hash)?;
        }
    }

    let largest = *widths.last().unwrap();
    let dims = imagesize::size(cache_path(&stem, largest))
        .ok()
        .map(|s| (s.width as u32, s.height as u32));

    Ok(Image { stem, widths, dims })
}

fn stem_of(file: &Path) -> io::Result<String> {
    if file
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(io_err(
            "path must be relative to images/ with no '..' or '.'",
        ));
    }
    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io_err("file name is not valid UTF-8"))?;
    if stem.is_empty()
        || !stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
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

pub fn publish_social(img: &Image, out_dir: &Path) -> io::Result<()> {
    let (w, h) = SOCIAL_SIZE;
    let src = image::open(cache_path(&img.stem, img.largest())).map_err(io_err)?;
    let inner = src
        .resize(w - 2 * SOCIAL_PAD, h - 2 * SOCIAL_PAD, FilterType::Lanczos3)
        .into_rgb8();

    let mut canvas = src
        .resize_to_fill(w, h, FilterType::Triangle)
        .blur(28.0)
        .into_rgb8();

    let x = (w - inner.width()) / 2;
    let y = (h - inner.height()) / 2;
    image::imageops::overlay(&mut canvas, &inner, x as i64, y as i64);

    fs::create_dir_all(out_dir)?;
    let file = fs::File::create(out_dir.join(SOCIAL_NAME))?;
    let mut enc = JpegEncoder::new_with_quality(BufWriter::new(file), SOCIAL_QUALITY);
    enc.encode_image(&canvas).map_err(io_err)
}

#[cfg(test)]
mod tests {
    use super::fnv1a;

    #[test]
    fn fnv1a_matches_reference_vectors() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }
}
