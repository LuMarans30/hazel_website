mod content;
mod images;
mod layout;
mod pages;

use std::fs;
use std::io;
use std::path::Path;

use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use lightningcss::targets::{Browsers, Targets};
use maud::Markup;

const OUT: &str = "dist";
const STATIC: &str = "static";
const STYLES: &str = "styles/style.css";
const CONTENT: &str = "paintings.toml";

fn main() -> io::Result<()> {
    let out = Path::new(OUT);
    let static_dir = Path::new(STATIC);
    let content = content::SiteContent::load(CONTENT)?;

    if out.exists() {
        fs::remove_dir_all(out)?;
    }

    if static_dir.exists() {
        copy_dir(static_dir, out)?;
    }

    content.publish_images(&out.join("images"))?;

    content.publish_social(out)?;
    if content.site_url.is_none() {
        eprintln!("warning: site_url not set; canonical and social-card tags are omitted");
    }

    let css_source = fs::read_to_string(STYLES)
        .map_err(|e| io::Error::other(format!("Failed to read CSS file at '{STYLES}': {e}")))?;

    let css = build_css(&css_source)?;
    fs::write(out.join("style.css"), css)?;

    write_page(out, "index.html", pages::home(&content))?;
    write_page(out, "paintings/index.html", pages::gallery(&content))?;
    write_page(out, "about/index.html", pages::about(&content))?;
    write_page(out, "contact/index.html", pages::contact(&content))?;

    println!("Site successfully generated in ./{OUT}");
    Ok(())
}

fn targets() -> Targets {
    Browsers {
        chrome: Some(105 << 16),
        edge: Some(105 << 16),
        firefox: Some(104 << 16),
        safari: Some(16 << 16),
        ..Browsers::default()
    }
    .into()
}

fn build_css(source: &str) -> io::Result<String> {
    let to_io = |e: &dyn std::fmt::Display| io::Error::other(format!("CSS error: {e}"));

    let mut sheet = StyleSheet::parse(source, ParserOptions::default()).map_err(|e| to_io(&e))?;
    sheet
        .minify(MinifyOptions {
            targets: targets(),
            ..MinifyOptions::default()
        })
        .map_err(|e| to_io(&e))?;
    let output = sheet
        .to_css(PrinterOptions {
            minify: true,
            targets: targets(),
            ..PrinterOptions::default()
        })
        .map_err(|e| to_io(&e))?;
    Ok(output.code)
}

fn write_page(out: &Path, rel_path: &str, markup: Markup) -> io::Result<()> {
    let path = out.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, markup.into_string())
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    if !from.exists() {
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}
