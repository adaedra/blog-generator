use dolmen::{prelude::*, Fragment};
use dolmen_dsl::element as tag;
use once_cell::sync::Lazy;
use pastex::{document::Document, output::html};
use std::{
    fmt, fs,
    iter::once,
    path::{Path, PathBuf},
};
use time::{format_description::FormatItem, Date};

static DATE_FORMAT: Lazy<Vec<FormatItem<'_>>> =
    Lazy::new(|| time::format_description::parse("[year]-[month]-[day]").unwrap());

#[derive(serde::Deserialize)]
struct Social {
    name: String,
    icon_name: String,
    url: String,
}

#[derive(serde::Deserialize)]
struct BlogData {
    title: String,
    tagline: String,
    footer: String,
    socials: Vec<Social>,
    #[serde(default)]
    stylesheets: Vec<String>,
}

struct Article {
    document: Document,
    path: PathBuf,
    date: Date,
}

fn separator() -> dolmen::Element {
    tag!(p[class: "bl-separator", role: "presentation"] {{ "\u{25C7}" }})
}

fn article_preview(article: &Article) -> Box<dyn Node> {
    let title = article
        .document
        .metadata
        .title
        .as_ref()
        .unwrap()
        .to_string();
    let (_, summary) = html::output(&article.document);
    let path = article.path.file_stem().unwrap().to_str().unwrap();

    tag!(article[class: "bl-article-preview"] {
        a[href: { format!("/{:04}/{:02}/{}/", article.date.year(), article.date.iso_week(), path) }] {
            p {{ article.date.format(&DATE_FORMAT).unwrap() }};
            h3 {{ title }};
        };
        { summary.map(|block| tag!(div {{ block }}).into_node()).unwrap_or_else(|| Fragment::empty().into_node()) };
    }).into_node()
}

fn index(blog_data: &BlogData, articles: &[Article]) -> Fragment {
    let tagline = html::output_fragment(&pastex::document::process_fragment(&blog_data.tagline));
    let articles = Fragment::new(articles.iter().rev().map(article_preview));

    Fragment::new([
        tag!(main {
            div[class: "bl-main-wrapper"] {
                header[class: "bl-home"] {
                    h1 {{ &blog_data.title }};
                    p {{ tagline }};
                }
            }
        })
        .into_node(),
        tag!(div[class: "bl-main-wrapper"] {
            header {
                h2 {{ "Latest articles" }};
            }
            {{ articles }}
        })
        .into_node(),
    ])
}

fn articles() -> anyhow::Result<Vec<Article>> {
    let articles = glob::glob("../blog-data/articles/**/*.px")?
        .map(|path| path.map_err(Into::into))
        .map(|path| {
            path.and_then(|path| {
                let document = pastex::document::process(&path)?;

                Ok((document, path))
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut articles: Vec<Article> = articles
        .into_iter()
        .filter(|(document, _)| !document.metadata.draft && document.metadata.date.is_some())
        .map(|(document, path)| Article {
            path,
            date: Date::parse(document.metadata.date.as_ref().unwrap(), &DATE_FORMAT).unwrap(),
            document,
        })
        .collect();

    articles.sort_unstable_by_key(|item| item.date);
    Ok(articles)
}

fn article_page(article: &Article) -> Fragment {
    let title = article
        .document
        .metadata
        .title
        .as_ref()
        .unwrap()
        .to_string();
    let (contents, summary) = html::output(&article.document);

    let tag = tag!(main[class: "bl-main-wrapper"] {
        header {
            p {{ article.date.format(&DATE_FORMAT).unwrap() }};
            h1 {{ title }};
        }
        { summary.map(|summary| {
            tag!(div[class: "bl-abstract"] {
                { summary };
                { separator() };
            }).into_node()
        }).unwrap_or_else(|| Fragment::empty().into_node()) };
        { contents };
    });

    Fragment::new(once(tag.into_node()))
}

fn layout(blog_data: &BlogData, inner: Fragment) -> Fragment {
    let footer = html::output_fragment(&pastex::document::process_fragment(&blog_data.footer));
    let socials = Fragment::new(blog_data.socials.iter().map(|social| {
        tag!(a[href: {social.url.clone()}, target: "_blank", title: {social.name.clone()}] {
            svg[xmlns: "http://www.w3.org/2000/svg", viewbox: "0 0 16 16", alt: {social.name.clone()}] {
                use[href: {format!("/assets/icons.svg#{}", social.icon_name)}]
            };
            span {{ &social.name }};
        })
        .into_node()
    }));
    let stylesheets = Fragment::new(blog_data.stylesheets.iter().map(|stylesheet| {
        tag!(link[rel: "stylesheet", type: "text/css", href: {stylesheet.clone()}]).into_node()
    }));

    let html = tag!(html[lang: "en"] {
        head {
            meta[charset: "utf-8"];
            meta[name: "viewport", content: "width=device-width, initial-scale=1"];
            title {{ &blog_data.title }};
            { stylesheets };
        }
        body {
            nav {
                div[class: "bl-wrapper"] {
                    a[href: "/"] {{ &blog_data.title }};
                    a[href: "/articles/"] {{ "Articles" }};
                    a[href: "/me/"] {{ "About me" }};
                    span[class: "bl-separator"] {{ Fragment::empty() }};
                    { socials };
                }
            }
            { inner };
            footer {
                div[class: "bl-wrapper"] {
                    { separator() };
                    { footer };
                }
            };
        }
    });
    Fragment::new(once(html.into_node()))
}

fn article_list(articles: &[Article]) -> Fragment {
    let articles = Fragment::new(articles.iter().rev().map(article_preview));
    Fragment::new(once(
        tag!(div[class: "bl-main-wrapper"] {{ articles }}).into_node(),
    ))
}

struct HtmlDocument(Fragment);

impl fmt::Display for HtmlDocument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "<!DOCTYPE html>")?;
        self.0.fmt(f)
    }
}

fn main() -> anyhow::Result<()> {
    let blog_data = fs::read_to_string("../blog-data/blog.toml")?;
    let blog_data: BlogData = toml::from_str(&blog_data)?;

    let output_dir = Path::new("output");
    if !output_dir.is_dir() {
        fs::create_dir(output_dir)?;
    }

    let articles = articles()?;

    {
        let document = HtmlDocument(layout(
            &blog_data,
            Fragment::from(index(&blog_data, &articles)),
        ));
        fs::write(output_dir.join("index.html"), document.to_string())?;
    }

    {
        let article_list = article_list(&articles);
        let article_list = layout(&blog_data, Fragment::from(article_list));

        let path = output_dir.join("articles");
        if !path.is_dir() {
            fs::create_dir(&path)?;
        }

        fs::write(path.join("index.html"), article_list.to_string())?;
    }

    for article in articles {
        let document = HtmlDocument(layout(&blog_data, Fragment::from(article_page(&article))));
        let path = output_dir
            .join(format!(
                "{:04}/{:02}",
                article.date.year(),
                article.date.iso_week()
            ))
            .join(article.path.file_stem().unwrap());
        if !path.is_dir() {
            fs::create_dir_all(&path)?;
        }

        fs::write(path.join("index.html"), document.to_string())?;
    }

    let pages = glob::glob("../blog-data/pages/**/*.px")?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|path| {
            let document = pastex::document::process(&path).unwrap();
            let (result, _) = html::output(&document);
            let page = tag!(main[class: "bl-main-wrapper"] {
                header {
                    h1 {{ document.metadata.title.unwrap() }};
                }
                { result }
            });
            let page = layout(&blog_data, Fragment::new(once(page.into_node())));

            (path, page)
        });
    for (src_path, page) in pages {
        let path = output_dir.join(src_path.file_stem().unwrap());
        if !path.is_dir() {
            fs::create_dir_all(&path)?;
        }

        fs::write(path.join("index.html"), page.to_string())?;
    }

    Ok(())
}
