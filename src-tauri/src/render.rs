use comrak::{markdown_to_html, Options};

/// Render Markdown to an HTML fragment.
///
/// `render.unsafe_` is deliberately left off: this app opens arbitrary files
/// from disk, so raw HTML in a document is escaped rather than passed through.
pub fn render(md: &str) -> String {
    let mut o = Options::default();

    o.extension.table = true;
    o.extension.strikethrough = true;
    o.extension.tasklist = true;
    o.extension.autolink = true;
    o.extension.footnotes = true;
    o.extension.description_lists = true;
    o.extension.tagfilter = true;
    // Empty prefix: heading anchors are plain slugs, so `#some-section` links work.
    o.extension.header_id_prefix = Some(String::new());

    // We want `<code class="language-rust">`, which is what highlight.js keys on.
    // `github_pre_lang` would emit `<pre lang="rust">` instead.
    o.render.github_pre_lang = false;

    markdown_to_html(md, &o)
}

/// Best-effort plain-text decode: strips a UTF-8 BOM, replaces invalid sequences.
pub fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

/// First ATX/setext heading in the document, used as the window title.
pub fn first_heading(md: &str) -> Option<String> {
    let mut in_fence = false;
    let mut prev: Option<&str> = None;
    for line in md.lines() {
        let t = line.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            prev = None;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(rest) = t.strip_prefix('#') {
            let text = rest.trim_start_matches('#').trim();
            if !text.is_empty() {
                return Some(text.trim_end_matches('#').trim().to_string());
            }
        }
        // setext: a line of === or --- underlining the previous non-empty line
        if let Some(p) = prev {
            if !p.is_empty()
                && t.len() >= 2
                && (t.chars().all(|c| c == '=') || t.chars().all(|c| c == '-'))
            {
                return Some(p.to_string());
            }
        }
        prev = Some(t);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const KITCHEN_SINK: &str = r#"# Title

| a | b |
|---|---|
| 1 | 2 |

- [x] done
- [ ] todo

~~struck~~ and a footnote[^1].

[^1]: the note.

```rust
fn main() {}
```
"#;

    #[test]
    fn gfm_constructs_render() {
        let html = render(KITCHEN_SINK);
        assert!(html.contains("<table>"), "table extension off: {html}");
        assert!(html.contains("type=\"checkbox\""), "tasklist off: {html}");
        assert!(html.contains("<del>"), "strikethrough off: {html}");
        assert!(html.contains("footnote"), "footnotes off: {html}");
        assert!(
            html.contains("class=\"language-rust\""),
            "expected language- class for highlight.js: {html}"
        );
        assert!(html.contains("<h1"), "heading missing: {html}");
    }

    #[test]
    fn heading_ids_are_emitted() {
        let html = render("## Some Section\n");
        assert!(html.contains("id=\"some-section\""), "{html}");
    }

    /// With `render.unsafe_` off, comrak drops raw HTML entirely rather than
    /// escaping it — the tag never reaches the webview in any form.
    #[test]
    fn raw_html_is_stripped_not_executed() {
        let html = render("Hello <script>alert(1)</script> world\n");
        assert!(
            !html.contains("<script"),
            "raw script tag survived rendering: {html}"
        );
        assert!(html.contains("raw HTML omitted"), "{html}");
    }

    #[test]
    fn inline_event_handlers_do_not_survive() {
        let html = render("<img src=x onerror=alert(1)>\n");
        assert!(!html.contains("onerror"), "{html}");
    }

    /// The tagfilter extension neutralises the tags GFM singles out even when
    /// raw HTML is otherwise permitted; assert the end state directly.
    #[test]
    fn iframes_do_not_survive() {
        let html = render("<iframe src=\"https://example.com\"></iframe>\n");
        assert!(!html.contains("<iframe"), "{html}");
    }

    #[test]
    fn autolinks_work() {
        let html = render("see https://example.com now\n");
        assert!(html.contains("href=\"https://example.com\""), "{html}");
    }

    #[test]
    fn decode_strips_bom() {
        assert_eq!(decode(b"\xef\xbb\xbf# Hi"), "# Hi");
        assert_eq!(decode(b"# Hi"), "# Hi");
    }

    #[test]
    fn first_heading_finds_atx() {
        assert_eq!(first_heading("# Hello\n\ntext"), Some("Hello".into()));
        assert_eq!(first_heading("text\n\n## Deeper\n"), Some("Deeper".into()));
    }

    #[test]
    fn first_heading_finds_setext() {
        assert_eq!(first_heading("Hello\n=====\n"), Some("Hello".into()));
    }

    #[test]
    fn first_heading_ignores_fenced_comments() {
        let md = "```sh\n# not a heading\n```\n\n# Real\n";
        assert_eq!(first_heading(md), Some("Real".into()));
    }

    #[test]
    fn first_heading_absent() {
        assert_eq!(first_heading("just a paragraph\n"), None);
    }
}
