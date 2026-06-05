use std::{fs, path::Path};

use anyhow::{Result, anyhow};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};
use serde::de::DeserializeOwned;

use crate::highlight::Highlighter;

/// Parse a markdown file, returing its frontmatter and its content as HTML
pub fn parse_file<F>(path: &Path) -> Result<(F, String)>
where
    F: DeserializeOwned,
{
    let raw = fs::read_to_string(path)?;

    // split on the +++ delimiters
    let parts: Vec<&str> = raw.splitn(3, "+++").collect();
    if parts.len() < 3 {
        return Err(anyhow!("missing front matter in  {:?}", path));
    }

    let front: F = toml::from_str(parts[1].trim())?;
    let markdown = parts[2].trim();

    // markdown -> HTML
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, opts);

    let highlighter = Highlighter::new();

    let mut events = vec![];
    let mut in_code_block = false;
    let mut lang = String::new();
    let mut code_buf = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref l))) => {
                in_code_block = true;
                lang = l.to_string();
            }
            Event::Text(ref t) if in_code_block => {
                code_buf.push_str(t);
            }
            Event::End(TagEnd::CodeBlock) if in_code_block => {
                let html = if lang == "mermaid" {
                    mermaid_rs_renderer::render(&code_buf)?
                } else {
                    highlighter.highlight(&code_buf, &lang)?
                };
                events.push(Event::Html(html.into()));
                in_code_block = false;
                lang.clear();
                code_buf.clear();
            }
            _ => events.push(event),
        }
    }

    let mut html_out = String::new();
    html::push_html(&mut html_out, events.into_iter());

    Ok((front, html_out))
}
