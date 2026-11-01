use maud::{DOCTYPE, Markup, html};

const NAV: [(&str, &str); 3] = [
    ("paintings/", "Paintings"),
    ("about/", "About"),
    ("contact/", "Contact"),
];

const FAVICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Crect width='32' height='32' rx='9' fill='%231f6b45'/%3E%3Cpath d='M0 20c5-6 8-6 13 0s8 6 13 0 4-3 6-2v14H0z' fill='%237fdc9f'/%3E%3C/svg%3E";

pub fn root_of(page_path: &str) -> String {
    "../".repeat(page_path.split('/').filter(|s| !s.is_empty()).count())
}

pub fn url(root: &str, path: &str) -> String {
    let full = format!("{root}{path}");
    if full.is_empty() {
        "./".to_string()
    } else {
        full
    }
}

pub fn page(
    title: Option<&str>,
    current: &str,
    content: &crate::content::SiteContent,
    body: Markup,
) -> Markup {
    let root = root_of(current);
    const HERO_FONT: &str = "fonts/FleurDeLeah-Regular.woff2";

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="description" content=(content.tagline);
                meta name="theme-color" content="#f4f9f2" media="(prefers-color-scheme: light)";
                meta name="theme-color" content="#0a130f" media="(prefers-color-scheme: dark)";
                title {
                    @if let Some(t) = title {
                        (t) " - " (content.artist_name)
                    } @else {
                        (content.artist_name)
                    }
                }
                link rel="icon" href=(FAVICON);
                @if current.is_empty() {
                    link rel="preload" href=(format!("{root}{HERO_FONT}"))
                        as="font" type="font/woff2" crossorigin;
                }
                link rel="stylesheet" href=(format!("{root}style.css"));
                script src=(format!("{root}gallery.js")) defer {}

                @if let Some(code) = &content.goatcounter_code {
                    script
                        data-goatcounter=(format!("https://{code}.goatcounter.com/count"))
                        async
                        src="//gc.zgo.at/count.js" {}
                }
            }
            body {
                a.skip href="#main" { "Skip to content" }
                header.site-header {
                    a.site-name href=(url(&root, "")) { (content.artist_name) }
                    nav aria-label="Main" {
                        @for (href, label) in NAV {
                            a href=(url(&root, href)) aria-current=[(href == current).then_some("page")] { (label) }
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