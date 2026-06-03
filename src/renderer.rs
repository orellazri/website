use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use tera::{Context, Tera};

use crate::models::Post;

pub struct Renderer {
    tera: Tera,
    out: PathBuf,
}

impl Renderer {
    pub fn new(templates_dir: &Path, out: &Path) -> Result<Self> {
        let glob = format!("{}/**/*.html", templates_dir.display());
        let tera = Tera::new(&glob)?;
        Ok(Self {
            tera,
            out: out.into(),
        })
    }

    pub fn render_post(&self, post: &Post) -> Result<()> {
        let dir = self.out.join("posts").join(&post.slug);
        fs::create_dir_all(&dir)?;

        let mut ctx = Context::new();
        ctx.insert("post", post);

        let html = self.tera.render("post.html", &ctx)?;
        fs::write(dir.join("index.html"), html)?;

        Ok(())
    }
}
