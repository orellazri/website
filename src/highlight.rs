use anyhow::Result;
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    html::{IncludeBackground, styled_line_to_highlighted_html},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

pub struct Highlighter {
    ss: SyntaxSet,
    ts: ThemeSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            ss: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight(&self, code: &str, lang: &str) -> Result<String> {
        let syntax = self
            .ss
            .find_syntax_by_extension(lang)
            .unwrap_or_else(|| self.ss.find_syntax_plain_text());

        let theme = &self.ts.themes["base16-eighties.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        let mut output = String::from("<pre><code>");
        for line in LinesWithEndings::from(code) {
            let ranges = h.highlight_line(line, &self.ss)?;
            let html = styled_line_to_highlighted_html(&ranges, IncludeBackground::No)?;
            output.push_str(&html);
        }
        output.push_str("</code></pre>");

        Ok(output)
    }
}
