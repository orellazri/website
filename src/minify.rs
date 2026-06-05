use anyhow::{Result, anyhow};
use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
};

pub fn minify_css(source: &str) -> Result<String> {
    let mut stylesheet = StyleSheet::parse(source, ParserOptions::default())
        .map_err(|e| anyhow!("failed to parse css: {}", e))?;
    stylesheet.minify(MinifyOptions::default())?;
    let res = stylesheet.to_css(PrinterOptions {
        minify: true,
        ..Default::default()
    })?;

    Ok(res.code)
}
