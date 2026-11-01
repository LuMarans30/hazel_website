use maud::{DOCTYPE, Markup, html};

const NAV: [(&str, &str); 3] = [
    ("/paintings/", "Paintings"),
    ("/about/", "About"),
    ("/contact/", "Contact"),
];

/// A small green tile with a wave, so the browser tab isn't blank.
const FAVICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Crect width='32' height='32' rx='9' fill='%231f6b45'/%3E%3Cpath d='M0 20c5-6 8-6 13 0s8 6 13 0 4-3 6-2v14H0z' fill='%237fdc9f'/%3E%3C/svg%3E";

pub fn page(
    title: Option<&str>,
    current: &str,
    content: &crate::content::SiteContent,
    body: Markup,
) -> Markup {
    let full_title = match title {
        Some(t) => format!("{t} - {}", content.artist_name),
        None => content.artist_name.clone(),
    };

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="description" content=(content.tagline);
                meta name="theme-color" content="#f4f9f2" media="(prefers-color-scheme: light)";
                meta name="theme-color" content="#0a130f" media="(prefers-color-scheme: dark)";
                title { (full_title) }
                link rel="icon" href=(FAVICON);
                link rel="stylesheet" href="/style.css";
                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
                script src="/gallery.js" defer {}
            }
            body {
                a.skip href="#main" { "Skip to content" }
                header.site-header {
                    a.site-name href="/" { (content.artist_name) }
                    nav aria-label="Main" {
                        @for (href, label) in NAV {
                            a href=(href) aria-current=[(href == current).then_some("page")] { (label) }
                        }
                    }
                }
                main #main { (body) }
                footer.site-footer {
                    "© " (content.artist_name)
                }
            }
        }
    }
}
