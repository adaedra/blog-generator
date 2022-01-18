use dolmen::{tag, ElementBox, HtmlDocument, IntoElementBox};
use pastex::output::html;

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
            let (document_block, abstract_block) = html::output(document);

            tag!(box article {
                tag!(h1 { title; });
                abstract_block.map(|block| tag!(div => block));
                tag!(div => document_block);
            })
        })
        .collect())
}

fn main() -> anyhow::Result<()> {
    let options = std::fs::read_to_string("../blog-data/blog.toml")?;
    let options: BlogData = toml::from_str(&options)?;

    let document = HtmlDocument(tag!(html(lang = "en") {
        tag!(head {
            tag!(meta(charset = "utf-8"));
            tag!(title { options.title.clone(); });
        });
        tag!(body {
            tag!(nav {
                tag!(a(href="/") { options.title; });
            });

            tag!(main => articles()?);
        });
    }));

    println!("{}", document);
    Ok(())
}
