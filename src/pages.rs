use maud::{Markup, html};

use crate::content::{Painting, SiteContent};
use crate::layout::page;

const ICON_PREV: &str = "M15 5l-7 7 7 7";
const ICON_NEXT: &str = "M9 5l7 7-7 7";
const ICON_CLOSE: &str = "M6 6l12 12M18 6L6 18";

fn icon(path: &str) -> Markup {
    html! {
        svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"
        {
            path d=(path) {}
        }
    }
}

fn painting_item(p: &Painting) -> Markup {
    let src = format!("/images/{}", p.file);
    let details = p.details();

    html! {
        li id=(p.slug) style=(format!("--r:{:.4}", p.ratio())) {
            button.thumb type="button"
                data-full=(src) data-title=(p.title) data-alt=(p.alt_text()) data-details=(details)
            {
                @if let Some((w, h)) = p.dims {
                    img src=(src) alt=(p.alt_text()) width=(w) height=(h)
                        loading="lazy" decoding="async";
                } @else {
                    img src=(src) alt=(p.alt_text()) loading="lazy" decoding="async";
                }
            }
            p.caption {
                em { (&p.title) }
                @if !details.is_empty() { span { (details) } }
            }
        }
    }
}

const HOME_COUNT: usize = 8;

pub fn home(content: &SiteContent) -> Markup {
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
                @for p in featured { (painting_item(p)) }
            }
            div.more {
                a href="/paintings/" { "See all " (content.paintings.len()) " paintings" }
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
    page(
        Some("Paintings"),
        "/paintings/",
        content,
        html! {
            h1 { "Paintings" }
            ul.gallery {
                @for p in &content.paintings { (painting_item(p)) }
            }
            (lightbox())
        },
    )
}

pub fn about(content: &SiteContent) -> Markup {
    let alt = content
        .portrait_alt
        .clone()
        .unwrap_or_else(|| format!("Portrait of {}", content.artist_name));

    page(
        Some("About"),
        "/about/",
        content,
        html! {
            h1 { "About" }
            div.about {
                @if let Some(file) = &content.portrait {
                    @let src = format!("/images/{file}");
                    figure.portrait {
                        @if let Some((w, h)) = content.portrait_dims {
                            img src=(src) alt=(alt) width=(w) height=(h) decoding="async";
                        } @else {
                            img src=(src) alt=(alt) decoding="async";
                        }
                    }
                }
                div.prose.bio {
                    p.eyebrow { (content.tagline) }
                    @for (i, paragraph) in content.bio.iter().enumerate() {
                        p.lead[i == 0] { (paragraph) }
                    }
                    div.actions {
                        a href="/contact/" { "Get in touch" }
                        a.ghost href="/paintings/" { "View the paintings" }
                    }
                }
            }
        },
    )
}

pub fn contact(content: &SiteContent) -> Markup {
    page(
        Some("Contact"),
        "/contact/",
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

