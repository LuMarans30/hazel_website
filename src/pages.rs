use maud::{Markup, html};

use crate::content::{Painting, SiteContent};
use crate::layout::{page, root_of, url};

const ICON_PREV: &str = "M15 5l-7 7 7 7";
const ICON_NEXT: &str = "M9 5l7 7-7 7";
const ICON_CLOSE: &str = "M6 6l12 12M18 6L6 18";

const EAGER_COUNT: usize = 3;
const HOME_COUNT: usize = 8;

#[derive(Clone, Copy)]
enum Place {
    Grid,
    Carousel,
}

fn icon(path: &str) -> Markup {
    html! {
        svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"
        {
            path d=(path) {}
        }
    }
}

fn sizes_for(place: Place, ratio: f32) -> String {
    match place {
        Place::Grid => format!("min(100vw, calc({ratio:.3} * clamp(9.5rem, 21vw, 17rem)))"),
        Place::Carousel => {
            format!("calc({ratio:.3} * clamp(13rem, min(42vh, 40vw), 27rem))")
        }
    }
}

fn painting_item(p: &Painting, root: &str, place: Place, index: usize) -> Markup {
    let img = &p.image;
    let details = p.details();
    let dims = img.dims;
    let eager = index < EAGER_COUNT;

    html! {
        li id=(p.slug) style=(format!("--r:{:.4}", p.ratio())) {
            button.thumb type="button"
                data-full=(img.full_url(root)) data-title=(p.title)
                data-alt=(p.alt_text()) data-details=(details)
            {
                img src=(img.fallback_url(root))
                    srcset=(img.srcset(root))
                    sizes=(sizes_for(place, p.ratio()))
                    alt=(p.alt_text())
                    width=[dims.map(|d| d.0)]
                    height=[dims.map(|d| d.1)]
                    loading=(if eager { "eager" } else { "lazy" })
                    fetchpriority=[(index == 0).then_some("high")]
                    decoding="async";
            }
            p.caption {
                em { (&p.title) }
                @if !details.is_empty() { span { (details) } }
            }
        }
    }
}

pub fn home(content: &SiteContent) -> Markup {
    let root = root_of("");

    let mut featured: Vec<&Painting> = content.paintings.iter().filter(|p| p.is_featured).collect();
    if featured.is_empty() {
        featured = content.paintings.iter().take(HOME_COUNT).collect();
    }

    page(
        None,
        "",
        content,
        html! {
            section.intro {
                h1.artist-name { (content.artist_name) }
                p { (content.tagline) }
            }
            ul.gallery.carousel aria-label="Selected paintings" {
                @for (i, p) in featured.iter().enumerate() {
                    (painting_item(p, &root, Place::Carousel, i))
                }
            }
            div.more {
                a href=(url(&root, "paintings/")) { "See all " (content.paintings.len()) " paintings" }
                div.carousel-nav hidden {
                    button.carousel-btn type="button" data-carousel="prev"
                        aria-label="Scroll to earlier paintings" { (icon(ICON_PREV)) }
                    button.carousel-btn type="button" data-carousel="next"
                        aria-label="Scroll to more paintings" { (icon(ICON_NEXT)) }
                }
            }
            (lightbox())
        },
    )
}

pub fn gallery(content: &SiteContent) -> Markup {
    let root = root_of("paintings/");

    page(
        Some("Paintings"),
        "paintings/",
        content,
        html! {
            h1 { "Paintings" }
            ul.gallery {
                @for (i, p) in content.paintings.iter().enumerate() {
                    (painting_item(p, &root, Place::Grid, i))
                }
            }
            (lightbox())
        },
    )
}

pub fn about(content: &SiteContent) -> Markup {
    let root = root_of("about/");
    let alt = content
        .portrait_alt
        .clone()
        .unwrap_or_else(|| format!("Portrait of {}", content.artist_name));

    page(
        Some("About"),
        "about/",
        content,
        html! {
            h1 { "About" }
            div.about {
                @if let Some(img) = &content.portrait_image {
                    figure.portrait {
                        img src=(img.fallback_url(&root))
                            srcset=(img.srcset(&root))
                            sizes="(min-width: 60rem) 60vw, 100vw"
                            alt=(alt)
                            width=[img.dims.map(|d| d.0)]
                            height=[img.dims.map(|d| d.1)]
                            fetchpriority="high"
                            decoding="async";
                    }
                }
                div.prose.bio {
                    p.eyebrow { (content.tagline) }
                    @for (i, paragraph) in content.bio.iter().enumerate() {
                        p.lead[i == 0] { (paragraph) }
                    }
                    div.actions {
                        a href=(url(&root, "contact/")) { "Get in touch" }
                        a.ghost href=(url(&root, "paintings/")) { "View the paintings" }
                    }
                }
            }
        },
    )
}

pub fn contact(content: &SiteContent) -> Markup {
    page(
        Some("Contact"),
        "contact/",
        content,
        html! {
            h1 { "Contact" }
            div.prose {
                p { "For commissions, sales and exhibitions, write to:" }
                p.email {
                    a href=(format!("mailto:{}", content.email)) { (&content.email) }
                    button.copy type="button" data-copy=(content.email) { "Copy address" }
                }
            }
        },
    )
}

fn lightbox() -> Markup {
    html! {
        dialog #lightbox aria-label="Painting viewer" {
            figure {
                img #lightbox-img alt="";
                figcaption {
                    em #lightbox-title {}
                    span #lightbox-details {}
                }
            }
            p.count #lightbox-count aria-live="polite" {}
            button.close type="button" data-action="close" aria-label="Close" { (icon(ICON_CLOSE)) }
            button.nav.prev type="button" data-action="prev" aria-label="Previous painting" { (icon(ICON_PREV)) }
            button.nav.next type="button" data-action="next" aria-label="Next painting" { (icon(ICON_NEXT)) }
        }
    }
}
