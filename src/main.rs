use dolmen::{tag, ElementBox, Fragment, HtmlDocument, IntoElementBox, Tag};
use once_cell::sync::Lazy;
use pastex::{document::Document, output::html};
use std::{
    fs,
    io::Write,
    iter,
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

fn separator() -> ElementBox {
    tag!(box p(class = "bl-separator", role = "presentation") { "\u{25C7}"; })
}

fn article_preview(article: &Article) -> ElementBox {
    let title = article
        .document
        .metadata
        .title
        .as_ref()
        .unwrap()
        .to_string();
    let (_, summary) = html::output(&article.document);
    let path = article.path.file_stem().unwrap().to_str().unwrap();

    tag!(box article(class = "bl-article-preview") {
        tag!(a(href = format!("/{:04}/{:02}/{}/", article.date.year(), article.date.iso_week(), path)) {
            tag!(p { &article.date.format(&DATE_FORMAT).unwrap(); });
            tag!(h3 { &title; });
        });
        summary.map(|block| tag!(div => block));
    })
}

fn index(blog_data: &BlogData, articles: &[Article]) -> Vec<ElementBox> {
    let tagline = html::output_fragment(&pastex::document::process_fragment(&blog_data.tagline));
    let articles = articles.iter().rev().map(article_preview).collect();

    vec![tag!(box main {
        tag!(box div(class = "bl-main-wrapper") {
            tag!(header(class = "bl-home") {
                tag!(h1 { &blog_data.title; });
                tag!(p => tagline);
            });
        });
        tag!(box div(class="bl-main-wrapper") {
            tag!(header {
                tag!(h2 { "Latest articles"; });
            });
            Fragment::from(articles);
        });
    })]
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

fn article_page(article: &Article) -> Vec<ElementBox> {
    let title = article
        .document
        .metadata
        .title
        .as_ref()
        .unwrap()
        .to_string();
    let (contents, summary) = html::output(&article.document);

    let inner = vec![tag!(box header {
        tag!(p { &article.date.format(&DATE_FORMAT).unwrap(); });
        tag!(h1 { &title; });
    } )]
    .into_iter()
    .chain(
        summary
            .map(|summary| tag!(box div(class = "bl-abstract") => summary.into_iter().chain(iter::once(separator())).collect()))
            .into_iter(),
    )
    .chain(contents)
    .collect();

    vec![tag!(box main(class = "bl-main-wrapper") => inner)]
}

fn layout(blog_data: &BlogData, inner: Fragment) -> Tag<dolmen::html::html> {
    let footer = html::output_fragment(&pastex::document::process_fragment(&blog_data.footer));
    let socials = blog_data
        .socials
        .iter()
        .map(|social| {
            tag!(box a(href = social.url, target = "_blank", title = social.name) {
                tag!(svg(xmlns = "http://www.w3.org/2000/svg", viewbox = "0 0 16 16", alt = social.name) {
                    tag!(r#use(href = format!("/assets/icons.svg#{}", social.icon_name)));
                });
                tag!(span { &social.name; });
            })
        })
        .collect();
    let stylesheets = blog_data
        .stylesheets
        .iter()
        .map(|stylesheet| tag!(box link(rel = "stylesheet", type = "text/css", href = stylesheet)))
        .collect();

    tag!(html(lang = "en") {
        tag!(head {
            tag!(meta(charset = "utf-8"));
            tag!(meta(name = "viewport", content = "width=device-width, initial-scale=1"));

            tag!(title { &blog_data.title; });
            Fragment::from(stylesheets);
        });
        tag!(body {
            tag!(nav {
                tag!(div(class = "bl-wrapper") {
                    tag!(a(href = "/") { &blog_data.title; });
                    tag!(a(href = "/articles/") { "Articles"; });
                    tag!(a(href = "/me/") { "About me"; });
                    tag!(span(class = "bl-separator") { Fragment::empty(); });
                    Fragment::from(socials);
                });
            });

            inner;

            tag!(footer {
                tag!(div(class = "bl-wrapper") => iter::once(separator()).chain(footer).collect());
            });
        });
    })
}

fn article_list(articles: &[Article]) -> Vec<ElementBox> {
    vec![
        tag!(box div(class = "bl-main-wrapper") => articles.iter().rev().map(article_preview).collect()),
    ]
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
        let mut output = fs::File::create(output_dir.join("index.html"))?;
        writeln!(output, "{}", document)?;
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
            let page = tag!(box main(class = "bl-main-wrapper") {
                tag!(header {
                    tag!(h1 { &document.metadata.title.unwrap(); });
                });
                Fragment::from(result);
            });
            let page = layout(&blog_data, Fragment::from(vec![page]));

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
