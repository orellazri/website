use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use tera::{Context, Tera};

use crate::models::{Page, Post};

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

    pub fn render_page(&self, page: &Page) -> Result<()> {
        let mut ctx = Context::new();
        ctx.insert("page", page);

        let out_path = if page.front.slug.is_empty() {
            // slug "" → dist/index.html
            self.out.join("index.html")
        } else {
            // slug "about" → dist/about/index.html
            let dir = self.out.join(&page.front.slug);
            std::fs::create_dir_all(&dir)?;
            dir.join("index.html")
        };

        let html = self.tera.render(&page.front.template, &ctx)?;
        std::fs::write(out_path, html)?;

        Ok(())
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
