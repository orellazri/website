use std::{fs, path::Path};

use anyhow::{Result, anyhow};

use crate::models::Frontmatter;

/// Parse a markdown file, returing its frontmatter and its content as HTML
pub fn parse_file(path: &Path) -> Result<(Frontmatter, String)> {
    let raw = fs::read_to_string(path)?;

    // split on the +++ delimiters
    let parts: Vec<&str> = raw.splitn(3, "+++").collect();
    if parts.len() < 3 {
        return Err(anyhow!("missing front matter in  {:?}", path));
    }

    let frontmatter: Frontmatter = toml::from_str(parts[1].trim())?;
    let markdown = parts[2].trim();

    // markdown -> HTML
    let mut opts = pulldown_cmark::Options::empty();
    opts.insert(pulldown_cmark::Options::ENABLE_TABLES);
    opts.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    opts.insert(pulldown_cmark::Options::ENABLE_FOOTNOTES);

    let parser = pulldown_cmark::Parser::new_ext(markdown, opts);
    let mut html_out = String::new();
    pulldown_cmark::html::push_html(&mut html_out, parser);

    Ok((frontmatter, html_out))
}
