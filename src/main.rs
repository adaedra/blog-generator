use dolmen::{tag, ElementBox, Fragment, HtmlDocument, IntoElementBox, Tag};
use pastex::output::html;
use std::{fs, io::Write, path::Path};

#[derive(serde::Deserialize)]
struct BlogData {
    title: String,
}

fn articles() -> anyhow::Result<Vec<ElementBox>> {
    Ok(glob::glob("../blog-data/articles/**/*.px")?
        .map(|article| {
            let article = article.unwrap();
            let document = pastex::document::process(&article).unwrap();

            let title = document.metadata.title.as_ref().unwrap().to_string();
            let (document_block, abstract_block) = html::output(&document);

            tag!(box article {
                tag!(h1 { &title; });
                abstract_block.map(|block| tag!(div => block));
                tag!(div => document_block);
            })
        })
        .collect())
}

fn layout(options: &BlogData, inner: Fragment) -> Tag<dolmen::html::html> {
    tag!(html(lang = "en") {
        tag!(head {
            tag!(meta(charset = "utf-8"));
            tag!(title { &options.title; });
        });
        tag!(body {
            tag!(nav {
                tag!(a(href = "/") { &options.title; });
            });

            tag!(main { inner; });
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

    let document = HtmlDocument(layout(&options, Fragment::from(articles()?)));
    let mut output = fs::File::create(output_dir.join("index.html"))?;
    writeln!(output, "{}", document)?;

    Ok(())
}
