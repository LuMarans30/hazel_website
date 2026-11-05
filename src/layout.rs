use maud::{DOCTYPE, Markup, PreEscaped, html};

const NAV: [(&str, &str); 3] = [
    ("paintings/", "Paintings"),
    ("about/", "About"),
    ("contact/", "Contact"),
];

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

    let full_title = match title {
        Some(t) => format!("{t} - {}", content.artist_name),
        None => content.artist_name.clone(),
    };
    let canonical = content.canonical_url(current);
    let social = content.social_url().zip(content.social_card());
    let (card_w, card_h) = crate::images::SOCIAL_SIZE;

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="description" content=(content.tagline);
                meta name="theme-color" content=(content.theme.theme_color_light) media="(prefers-color-scheme: light)";
                meta name="theme-color" content=(content.theme.theme_color_dark) media="(prefers-color-scheme: dark)";
                style { (PreEscaped(&content.theme.css)) }
                title { (full_title) }

                @if let Some(url) = &canonical {
                    link rel="canonical" href=(url);
                    meta property="og:url" content=(url);
                }
                meta property="og:type" content="website";
                meta property="og:site_name" content=(content.artist_name);
                meta property="og:title" content=(full_title);
                meta property="og:description" content=(content.tagline);
                @if let Some((img_url, (_, alt))) = &social {
                    meta property="og:image" content=(img_url);
                    meta property="og:image:type" content="image/jpeg";
                    meta property="og:image:width" content=(card_w);
                    meta property="og:image:height" content=(card_h);
                    meta property="og:image:alt" content=(alt);
                    meta name="twitter:card" content="summary_large_image";
                } @else {
                    meta name="twitter:card" content="summary";
                }

                link rel="icon" href=(content.theme.favicon);
                @if current.is_empty() {
                    link rel="preload" href=(format!("{root}{HERO_FONT}"))
                        as="font" type="font/woff2" crossorigin;
                }
                link rel="stylesheet" href=(format!("{root}style.css"));
                script src=(format!("{root}gallery.js")) defer {}
                script src=(format!("{root}leaves.js")) defer {}

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
