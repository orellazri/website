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
    pub template: String,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
    pub front: PageFrontmatter,
    pub html: String,
}
