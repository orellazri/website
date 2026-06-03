use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    pub date: NaiveDate,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Post {
    pub front: Frontmatter,
    pub slug: String,
    pub html: String,
}
