use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PostFrontmatter {
    pub title: String,
    pub date: NaiveDate,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Post {
    pub front: PostFrontmatter,
    pub slug: String,
    pub html: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageFrontmatter {
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
    pub front: PageFrontmatter,
    pub slug: String,
    pub html: String,
}

pub enum PageContext<'a> {
    Index { posts: &'a [Post] },
    PostList { posts: &'a [Post] },
    Post { post: &'a Post },
    Page { page: &'a Page },
}

impl PageContext<'_> {
    pub fn template(&self) -> &'static str {
        match self {
            PageContext::Index { .. } => "index.html",
            PageContext::PostList { .. } => "posts.html",
            PageContext::Post { .. } => "post.html",
            PageContext::Page { .. } => "page.html",
        }
    }

    pub fn out_path(&self, dist: &Path) -> PathBuf {
        match self {
            PageContext::Index { .. } => dist.join("index.html"),
            PageContext::PostList { .. } => dist.join("posts/index.html"),
            PageContext::Post { post } => dist.join(format!("posts/{}/index.html", post.slug)),
            PageContext::Page { page } => dist.join(format!("{}/index.html", page.slug)),
        }
    }
}
