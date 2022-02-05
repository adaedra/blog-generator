use dolmen::{tag, ElementBox, Fragment, HtmlDocument, IntoElementBox, Tag};
use pastex::{document::Document, output::html};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

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
}

struct Article {
    document: Document,
    path: PathBuf,
}

fn index(options: &BlogData, articles: &[Article]) -> Vec<ElementBox> {
    let tagline = html::output_fragment(&pastex::document::process_fragment(&options.tagline));
    let articles = articles
        .iter()
        .map(|article| {
            let title = article
                .document
                .metadata
                .title
                .as_ref()
                .unwrap()
                .to_string();
            let (_, summary) = html::output(&article.document);
            let path = article.path.file_stem().unwrap().to_str().unwrap();

            tag!(box article {
                tag!(h3 {
                    tag!(a(href = format!("{}/", path)) { &title; });
                });
                summary.map(|block| tag!(div => block));
            })
        })
        .collect();

    vec![tag!(box main {
        tag!(box div(class = "main-wrapper") {
            tag!(header(class = "home") {
                tag!(h1 { &options.title; });
                tag!(p => tagline);
            });
        });
        tag!(box div(class="main-wrapper") {
            tag!(header {
                tag!(h2 { "Latest articles"; });
            });
            Fragment::from(articles);
        });
    })]
}

fn articles() -> anyhow::Result<Vec<Article>> {
    glob::glob("../blog-data/articles/**/*.px")?
        .map(|path| path.map_err(Into::into))
        .map(|path| {
            path.and_then(|path| {
                let document = pastex::document::process(&path)?;

                Ok(Article { document, path })
            })
        })
        .collect()
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

    let inner = vec![tag!(box header { tag!(h1 { &title; }); } )]
        .into_iter()
        .chain(
            summary
                .map(|summary| tag!(box div(class = "abstract") => summary))
                .into_iter(),
        )
        .chain(contents)
        .collect();

    vec![tag!(box main(class = "main-wrapper") => inner)]
}

fn layout(options: &BlogData, inner: Fragment) -> Tag<dolmen::html::html> {
    let footer = html::output_fragment(&pastex::document::process_fragment(&options.footer));
    let socials = options
        .socials
        .iter()
        .map(|social| {
            tag!(box a(href = social.url, target = "_blank", title = social.name) {
                tag!(svg(xmlns = "http://www.w3.org/2000/svg", viewbox = "0 0 30 30", alt = social.name) {
                    tag!(r#use(href = format!("/assets/icons.svg#{}", social.icon_name)));
                });
                tag!(span { &social.name; });
            })
        })
        .collect();

    tag!(html(lang = "en") {
        tag!(head {
            tag!(meta(charset = "utf-8"));
            tag!(meta(name = "viewport", content = "width=device-width, initial-scale=1"));

            tag!(title { &options.title; });
            // Keep only for production. In development, we use webpack.
            // TODO: Use a manifest system to have versioned file name.
            // tag!(link(rel = "stylesheet", type = "text/css", href = "/assets/main.css"));
        });
        tag!(body {
            tag!(nav {
                tag!(div(class = "wrapper") {
                    tag!(a(href = "/") { &options.title; });
                    tag!(a(href = "/articles/") { "Articles"; });
                    tag!(a(href = "/about/") { "About me"; });
                    tag!(span(class = "separator") { Fragment::empty(); });
                    Fragment::from(socials);
                });
            });
            tag!(div(class = "header-picture") {
                Fragment::empty();
            });

            inner;

            tag!(footer => footer);

            // TODO: Remove for production.
            tag!(script(type = "text/javascript", src="/assets/main.js") { Fragment::empty(); });
        });
    })
}

fn main() -> anyhow::Result<()> {
    let options = fs::read_to_string("../blog-data/blog.toml")?;
    let options: BlogData = toml::from_str(&options)?;

    let output_dir = Path::new("output");
    if !output_dir.is_dir() {
        fs::create_dir(output_dir)?;
    }

    let articles = articles()?;

    {
        let document = HtmlDocument(layout(&options, Fragment::from(index(&options, &articles))));
        let mut output = fs::File::create(output_dir.join("index.html"))?;
        writeln!(output, "{}", document)?;
    }

    for article in articles {
        let document = HtmlDocument(layout(&options, Fragment::from(article_page(&article))));
        let path = output_dir.join(article.path.file_stem().unwrap());
        if !path.is_dir() {
            fs::create_dir(&path)?;
        }

        let mut output = fs::File::create(path.join("index.html"))?;
        writeln!(output, "{}", document)?;
    }

    Ok(())
}
