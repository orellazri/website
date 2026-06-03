use std::path::Path;

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::{models::Post, parser::parse_file};

mod models;
mod parser;

fn collect_posts(content_dir: &Path) -> Result<Vec<Post>> {
    let mut posts = vec![];

    for entry in WalkDir::new(content_dir.join("posts"))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("md".as_ref()))
    {
        let (frontmatter, html) = parse_file(entry.path())?;
        let slug = entry
            .path()
            .file_stem()
            .context(format!("cannot extract file stem from {:?}", entry))?
            .to_string_lossy()
            .splitn(4, '-')
            .skip(3)
            .collect::<Vec<_>>()
            .join("-");

        posts.push(Post {
            frontmatter,
            slug,
            html,
        })
    }

    // sort by newest first
    posts.sort_by_key(|b| std::cmp::Reverse(b.frontmatter.date));

    Ok(posts)
}

fn main() -> Result<()> {
    let posts = collect_posts(Path::new("content"))?;
    dbg!(posts);

    Ok(())
}
