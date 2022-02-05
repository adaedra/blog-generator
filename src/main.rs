use clap::Parser;
use dolmen::{tag, ElementBox, Fragment, HtmlDocument, IntoElementBox, Tag};
use pastex::{document::Document, output::html};
use std::{
    collections::HashMap,
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

#[derive(Parser)]
struct CliOptions {
    #[clap(long)]
    production: bool,
    #[clap(long)]
    manifest: Option<PathBuf>,
}

trait AssetManifest {
    fn asset(&self, name: &str) -> String;
}

struct NoopAssetManifest;

impl AssetManifest for NoopAssetManifest {
    fn asset(&self, name: &str) -> String {
        format!("/assets/{}", name)
    }
}

struct WebpackManifest(HashMap<String, String>);

impl WebpackManifest {
    fn from(path: &Path) -> anyhow::Result<WebpackManifest> {
        let reader = fs::File::open(path)?;
        let map = serde_json::from_reader(reader)?;

        Ok(WebpackManifest(map))
    }
}

impl AssetManifest for WebpackManifest {
    fn asset(&self, name: &str) -> String {
        self.0
            .get(name)
            .map(Clone::clone)
            // TODO: Warn of missing asset
            .unwrap_or_else(|| format!("/assets/{}", name))
    }
}

struct Article {
    document: Document,
    path: PathBuf,
}

fn index(blog_data: &BlogData, articles: &[Article]) -> Vec<ElementBox> {
    let tagline = html::output_fragment(&pastex::document::process_fragment(&blog_data.tagline));
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
                    tag!(a(href = format!("/{}/", path)) { &title; });
                });
                summary.map(|block| tag!(div => block));
            })
        })
        .collect();

    vec![tag!(box main {
        tag!(box div(class = "main-wrapper") {
            tag!(header(class = "home") {
                tag!(h1 { &blog_data.title; });
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

fn layout(
    blog_data: &BlogData,
    options: &CliOptions,
    manifest: &dyn AssetManifest,
    inner: Fragment,
) -> Tag<dolmen::html::html> {
    let footer = html::output_fragment(&pastex::document::process_fragment(&blog_data.footer));
    let socials = blog_data
        .socials
        .iter()
        .map(|social| {
            tag!(box a(href = social.url, target = "_blank", title = social.name) {
                tag!(svg(xmlns = "http://www.w3.org/2000/svg", viewbox = "0 0 30 30", alt = social.name) {
                    tag!(r#use(href = format!("{}#{}", manifest.asset("icons.svg"), social.icon_name)));
                });
                tag!(span { &social.name; });
            })
        })
        .collect();

    tag!(html(lang = "en") {
        tag!(head {
            tag!(meta(charset = "utf-8"));
            tag!(meta(name = "viewport", content = "width=device-width, initial-scale=1"));

            tag!(title { &blog_data.title; });
            // Keep only for production. In development, we use webpack.
            options.production.then(|| tag!(link(rel = "stylesheet", type = "text/css", href = manifest.asset("main.css"))));
        });
        tag!(body {
            tag!(nav {
                tag!(div(class = "wrapper") {
                    tag!(a(href = "/") { &blog_data.title; });
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

            (!options.production).then(|| tag!(script(type = "text/javascript", src = manifest.asset("main.js")) { Fragment::empty(); }));
        });
    })
}

fn main() -> anyhow::Result<()> {
    let blog_data = fs::read_to_string("../blog-data/blog.toml")?;
    let blog_data: BlogData = toml::from_str(&blog_data)?;
    let options = CliOptions::parse();
    let manifest = options
        .manifest
        .as_ref()
        .map(|path| WebpackManifest::from(&path))
        .transpose()?
        .map(|manifest| Box::new(manifest) as Box<dyn AssetManifest>)
        .unwrap_or_else(|| Box::new(NoopAssetManifest));

    let output_dir = Path::new("output");
    if !output_dir.is_dir() {
        fs::create_dir(output_dir)?;
    }

    let articles = articles()?;

    {
        let document = HtmlDocument(layout(
            &blog_data,
            &options,
            manifest.as_ref(),
            Fragment::from(index(&blog_data, &articles)),
        ));
        let mut output = fs::File::create(output_dir.join("index.html"))?;
        writeln!(output, "{}", document)?;
    }

    for article in articles {
        let document = HtmlDocument(layout(
            &blog_data,
            &options,
            manifest.as_ref(),
            Fragment::from(article_page(&article)),
        ));
        let path = output_dir.join(article.path.file_stem().unwrap());
        if !path.is_dir() {
            fs::create_dir(&path)?;
        }

        let mut output = fs::File::create(path.join("index.html"))?;
        writeln!(output, "{}", document)?;
    }

    Ok(())
}
