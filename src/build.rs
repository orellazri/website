use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::{
    feed::write_atom,
    minify::minify_css,
    models::{Page, PageContext, Post},
    parser::parse_file,
    renderer::Renderer,
};

fn copy_static(dist: &Path) -> Result<()> {
    for entry in WalkDir::new("static")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let relative = entry.path().strip_prefix("static")?;
        let dest = dist.join(relative);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        match entry.path().extension().and_then(|e| e.to_str()) {
            Some("css") => {
                let source = fs::read_to_string(entry.path())?;
                let minified = minify_css(&source)?;
                fs::write(&dest, minified)?;
            }
            _ => {
                fs::copy(entry.path(), &dest)?;
            }
        }
    }

    Ok(())
}

pub fn build() -> Result<()> {
    let dist = Path::new("dist");
    if dist.exists() {
        fs::remove_dir_all(dist)?;
    }
    fs::create_dir_all(dist)?;

    copy_static(dist)?;

    let posts = collect_posts(Path::new("content/posts"))?;
    let pages = collect_pages(Path::new("content/pages"))?;
    let renderer = Renderer::new(Path::new("templates"), dist)?;

    renderer.render(&PageContext::Index { posts: &posts })?;
    renderer.render(&PageContext::PostList { posts: &posts })?;

    for post in &posts {
        renderer.render(&PageContext::Post { post })?;
    }

    for page in &pages {
        renderer.render(&PageContext::Page { page })?;
    }

    let mut tag_map: HashMap<&str, Vec<&Post>> = HashMap::new();
    for post in &posts {
        for tag in &post.front.tags {
            tag_map.entry(tag).or_default().push(post);
        }
    }
    for (tag, tagged_posts) in &tag_map {
        renderer.render(&PageContext::Tag {
            tag,
            posts: tagged_posts,
        })?;
    }

    write_atom(
        &posts,
        dist,
        "https://orellazri.com",
        "Orel Lazri",
        "Orel Lazri",
    )?;

    Ok(())
}

fn collect_posts(content_dir: &Path) -> Result<Vec<Post>> {
    let mut posts = vec![];

    for entry in WalkDir::new(content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("md".as_ref()))
    {
        let (front, html) = parse_file(entry.path())?;
        let slug = entry
            .path()
            .file_stem()
            .context(format!("cannot extract file stem from {:?}", entry))?
            .to_string_lossy()
            .splitn(4, '-')
            .skip(3)
            .collect::<Vec<_>>()
            .join("-");

        posts.push(Post { front, slug, html })
    }

    // sort by newest first
    posts.sort_by_key(|b| std::cmp::Reverse(b.front.date));

    Ok(posts)
}

fn collect_pages(content_dir: &Path) -> Result<Vec<Page>> {
    let mut pages = vec![];

    for entry in WalkDir::new(content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("md".as_ref()))
    {
        let (front, html) = parse_file(entry.path())?;
        let slug = entry
            .path()
            .file_stem()
            .context(format!("cannot extract file stem from {:?}", entry))?
            .to_string_lossy()
            .to_string();

        pages.push(Page { front, slug, html })
    }

    Ok(pages)
}
