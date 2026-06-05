use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::Local;
use tera::{Context, Tera};

use crate::models::PageContext;

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

    fn base_context(&self) -> Context {
        let mut ctx = Context::new();
        ctx.insert("generated_at", &Local::now().format("%Y-%m-%d").to_string());
        ctx
    }

    pub fn render(&self, ctx: &PageContext) -> Result<()> {
        let mut tera_ctx = self.base_context();

        match ctx {
            PageContext::Index { posts } => tera_ctx.insert("posts", posts),
            PageContext::PostList { posts } => tera_ctx.insert("posts", posts),
            PageContext::Post { post } => tera_ctx.insert("post", post),
            PageContext::Page { page } => tera_ctx.insert("page", page),
            PageContext::Tag { tag, posts } => {
                tera_ctx.insert("tag", tag);
                tera_ctx.insert("posts", posts);
            }
        }

        let out_path = ctx.out_path(&self.out);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let html = self.tera.render(ctx.template(), &tera_ctx)?;
        fs::write(out_path, html)?;

        Ok(())
    }
}
