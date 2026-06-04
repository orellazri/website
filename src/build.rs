use std::{fs, path::Path};

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::{
    models::{Page, Post},
    parser::parse_file,
    renderer::Renderer,
};

pub fn build() -> Result<()> {
    let dist = Path::new("dist");
    if dist.exists() {
        fs::remove_dir_all(dist)?;
    }
    fs::create_dir_all(dist)?;

    fs_extra::dir::copy(
        "static",
        dist,
        &fs_extra::dir::CopyOptions {
            content_only: true,
            ..Default::default()
        },
    )?;

    let pages = collect_pages(Path::new("content/pages"))?;
    let posts = collect_posts(Path::new("content/posts"))?;
    let renderer = Renderer::new(Path::new("templates"), dist)?;

    fs::create_dir_all(dist.join("pages"))?;
    for page in &pages {
        renderer.render_page(page)?;
    }

    fs::create_dir_all(dist.join("posts"))?;
    for post in &posts {
        renderer.render_post(post)?;
    }

    Ok(())
}

fn collect_pages(content_dir: &Path) -> Result<Vec<Page>> {
    let mut pages = vec![];

    for entry in WalkDir::new(content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension() == Some("md".as_ref()))
    {
        let (front, html) = parse_file(entry.path())?;

        pages.push(Page { front, html })
    }

    Ok(pages)
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
