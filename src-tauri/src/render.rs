use comrak::nodes::NodeValue;
use comrak::{format_html, parse_document, Arena, Options};

/// What comrak leaves behind for each raw-HTML node when `unsafe_` is off.
/// One per dropped node, in document order — which is what lets `render` put
/// anchor targets back in the right places.
const OMITTED: &str = "<!-- raw HTML omitted -->";

fn options() -> Options<'static> {
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

    // Task-list write-back needs the line: this is what puts `data-sourcepos`
    // on the `<li>`, so a clicked checkbox knows which line to flip.
    o.render.sourcepos = true;

    o
}

/// Ids we are willing to reproduce. Deliberately narrow — the value is pasted
/// into an attribute we generate, so anything that could close the quote or the
/// tag is rejected outright rather than escaped.
fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
}

fn attribute<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    let lower = attrs.to_ascii_lowercase();
    let mut from = 0;
    while let Some(hit) = lower[from..].find(key) {
        let start = from + hit;
        // Must be a whole attribute name, not the tail of another one.
        let boundary = start == 0
            || lower[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_whitespace());
        let rest = attrs[start + key.len()..].trim_start();
        if boundary {
            if let Some(value) = rest.strip_prefix('=') {
                let value = value.trim_start();
                for quote in ['"', '\''] {
                    if let Some(v) = value.strip_prefix(quote) {
                        if let Some(end) = v.find(quote) {
                            return Some(&v[..end]);
                        }
                    }
                }
            }
        }
        from = start + key.len();
    }
    None
}

/// The name a bare `<a id="x">` / `<a name="x">` / `<span id="x">` gives to a
/// spot in the document, if that is all the tag is doing.
///
/// Documents in the wild mark link targets this way — GitHub and Azure DevOps
/// both render it — and without this every `[F5](#f5)` in such a file points at
/// nothing. Only the name is taken; the replacement tag is generated from
/// scratch, so no attribute of the original survives.
fn anchor_target(raw: &str) -> Option<String> {
    let mut tag = raw.trim();
    for close in ["</a>", "</span>"] {
        if let Some(head) = tag.strip_suffix(close) {
            tag = head.trim_end();
            break;
        }
    }
    let inner = tag.strip_prefix('<')?.strip_suffix('>')?;
    let inner = inner.strip_suffix('/').unwrap_or(inner);

    let (name, attrs) = match inner.find(char::is_whitespace) {
        Some(i) => (&inner[..i], &inner[i..]),
        None => (inner, ""),
    };
    if !name.eq_ignore_ascii_case("a") && !name.eq_ignore_ascii_case("span") {
        return None;
    }

    let id = attribute(attrs, "id").or_else(|| attribute(attrs, "name"))?;
    is_safe_id(id).then(|| id.to_string())
}

/// Render Markdown to an HTML fragment.
///
/// `render.unsafe_` is deliberately left off: this app opens arbitrary files
/// from disk, so raw HTML in a document is dropped rather than passed through,
/// and comrak's own filtering of dangerous link schemes stays in force.
///
/// The one thing recovered from that dropped HTML is anchor *targets* — see
/// `anchor_target`. They are rebuilt from the parsed name alone, so this adds
/// no path by which document HTML reaches the webview.
pub fn render(md: &str) -> String {
    let o = options();
    let arena = Arena::new();
    let root = parse_document(&arena, md, &o);

    // Raw-HTML nodes in document order, which is the order comrak drops them.
    let targets: Vec<Option<String>> = root
        .descendants()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::HtmlInline(raw) => Some(anchor_target(raw)),
            NodeValue::HtmlBlock(block) => Some(anchor_target(&block.literal)),
            _ => None,
        })
        .collect();

    let mut html = String::new();
    if format_html(root, &o, &mut html).is_err() {
        return String::new();
    }

    if targets.iter().all(Option::is_none) {
        return html;
    }

    let mut out = String::with_capacity(html.len());
    let mut rest: &str = &html;
    for target in &targets {
        let Some(at) = rest.find(OMITTED) else { break };
        out.push_str(&rest[..at]);
        if let Some(id) = target {
            out.push_str("<span id=\"");
            out.push_str(id);
            out.push_str("\"></span>");
        }
        rest = &rest[at + OMITTED.len()..];
    }
    out.push_str(rest);
    out
}

/// The UTF-8 byte-order mark, which Windows editors leave on files this app
/// both reads and — for task lists — writes back.
pub const BOM: &[u8] = b"\xef\xbb\xbf";

/// Best-effort plain-text decode: strips a UTF-8 BOM, replaces invalid sequences.
pub fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(BOM).unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

/// Flip the task box on 1-based `line`, or say why that line has none.
///
/// comrak decides what a task item is, with the same options `render` used,
/// so the line the webview sends back — read off a `data-sourcepos` comrak
/// wrote — is matched against the same parse. A hand-rolled scan would be a
/// second definition of "task item": one that counts lines differently (comrak
/// ends a line on a bare `\r` too) and takes `- [ ]` inside a code block at
/// face value.
pub fn toggle_task(md: &str, line: usize, checked: bool) -> Result<String, String> {
    let arena = Arena::new();
    let root = parse_document(&arena, md, &options());
    let symbol = root
        .descendants()
        .find_map(|node| {
            let data = node.data.borrow();
            match &data.value {
                NodeValue::TaskItem(item) if data.sourcepos.start.line == line => {
                    Some(item.symbol_sourcepos.start)
                }
                _ => None,
            }
        })
        .ok_or_else(|| format!("Line {line} is not a task item"))?;

    // Columns are 1-based bytes; the symbol is one ASCII byte between the brackets.
    let at = line_start(md, symbol.line)
        .map(|start| start + symbol.column - 1)
        .filter(|&at| matches!(md.as_bytes().get(at), Some(b' ' | b'x' | b'X')))
        .ok_or_else(|| format!("Line {line} does not hold the box comrak saw"))?;

    let mut out = String::with_capacity(md.len());
    out.push_str(&md[..at]);
    out.push(if checked { 'x' } else { ' ' });
    out.push_str(&md[at + 1..]);
    Ok(out)
}

/// Byte offset where 1-based `line` starts, counting lines the way comrak
/// does: `\n`, `\r\n`, or a bare `\r` each end one.
fn line_start(md: &str, line: usize) -> Option<usize> {
    let bytes = md.as_bytes();
    let mut i = 0;
    for _ in 1..line {
        let eol = i + bytes[i..].iter().position(|b| matches!(b, b'\n' | b'\r'))?;
        i = eol + 1;
        if bytes[eol] == b'\r' && bytes.get(i) == Some(&b'\n') {
            i += 1;
        }
    }
    Some(i)
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
        // Opening tags only: `sourcepos` gives most elements an attribute.
        assert!(html.contains("<table"), "table extension off: {html}");
        assert!(html.contains("type=\"checkbox\""), "tasklist off: {html}");
        assert!(html.contains("<del"), "strikethrough off: {html}");
        assert!(html.contains("footnote"), "footnotes off: {html}");
        assert!(
            html.contains("class=\"language-rust\""),
            "expected language- class for highlight.js: {html}"
        );
        assert!(html.contains("<h1"), "heading missing: {html}");
        // `- [x] done` is line 7, and the line number on the task `<li>` — not
        // merely the presence of the attribute — is what a clicked checkbox
        // sends back for `toggle_task` to edit.
        assert!(
            html.contains("<li data-sourcepos=\"7:1-"),
            "task item is not carrying its own line: {html}"
        );
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
        assert!(html.contains("Hello"), "surrounding text lost: {html}");
    }

    /// `render` rebuilds anchor targets by substituting comrak's placeholders in
    /// order, so it depends on there being exactly one per dropped node. If a
    /// comrak upgrade changes the wording or the count, fail here rather than
    /// silently scattering anchors through the document.
    #[test]
    fn dropped_html_leaves_one_ordered_placeholder_each() {
        // options() leaves raw HTML disabled, which is what produces placeholders.
        let raw = comrak::markdown_to_html("a <b>c</b> d <i>e</i>\n", &options());
        assert_eq!(
            raw.matches(OMITTED).count(),
            4,
            "expected one placeholder per raw tag: {raw}"
        );
    }

    #[test]
    fn explicit_html_anchors_become_link_targets() {
        // The shape Azure DevOps and GitHub documents use to name a section.
        let html = render("### <a id=\"f5\"></a>F5 — Something\n\nSee [F5](#f5).\n");
        assert!(html.contains("id=\"f5\""), "anchor target missing: {html}");
        assert!(html.contains("href=\"#f5\""), "link mangled: {html}");
        // The heading keeps its own slug as well, so both spellings resolve.
        assert!(html.contains("<h3"), "{html}");
    }

    #[test]
    fn name_attribute_anchors_also_work() {
        let html = render("<a name=\"old-style\"></a>text\n");
        assert!(html.contains("id=\"old-style\""), "{html}");
    }

    /// Only the name is carried over; everything else about the original tag is
    /// discarded, because the replacement is generated rather than passed through.
    #[test]
    fn anchor_recovery_carries_nothing_but_the_name() {
        let html = render("<a id=\"ok\" onclick=\"alert(1)\" href=\"javascript:x\"></a>hi\n");
        assert!(html.contains("id=\"ok\""), "{html}");
        assert!(!html.contains("onclick"), "{html}");
        assert!(!html.contains("javascript"), "{html}");
    }

    /// The other half of leaving comrak's `unsafe` option off: a *Markdown*
    /// link may not carry a scripting URL either. Pinned because the README
    /// states it, and this app opens files it did not write.
    #[test]
    fn markdown_links_cannot_carry_a_scripting_url() {
        for bad in [
            "[click](javascript:alert(1))",
            "[click](JaVaScRiPt:alert(1))",
            "[click](vbscript:msgbox)",
            "[click](data:text/html;base64,PHNjcmlwdD4=)",
            "![img](javascript:alert(1))",
        ] {
            let html = render(&format!("{bad}\n"));
            let lowered = html.to_ascii_lowercase();
            assert!(!lowered.contains("javascript:"), "{bad}: {html}");
            assert!(!lowered.contains("vbscript:"), "{bad}: {html}");
            assert!(!lowered.contains("data:text/html"), "{bad}: {html}");
        }
    }

    #[test]
    fn ids_that_could_break_out_of_the_attribute_are_refused() {
        for bad in [
            "a\"><script>alert(1)</script>",
            "a\" onload=\"x",
            "a'><img src=x onerror=y>",
            "has space",
        ] {
            let html = render(&format!("<a id=\"{bad}\"></a>text\n"));
            assert!(!html.contains("<script"), "{bad}: {html}");
            assert!(!html.contains("onload"), "{bad}: {html}");
            assert!(!html.contains("onerror"), "{bad}: {html}");
        }
    }

    /// A tag that does more than name a spot is still dropped whole.
    #[test]
    fn non_anchor_html_is_still_dropped() {
        let html = render("<div id=\"x\">body</div>\n");
        assert!(!html.contains("<div"), "{html}");
        assert!(!html.contains("id=\"x\""), "{html}");
    }

    /// Substitution walks placeholders in order, so an anchor must not be able
    /// to land on some unrelated tag's position.
    #[test]
    fn anchors_land_on_their_own_position() {
        let html = render("<b>bold</b>\n\n<a id=\"here\"></a>target\n");
        let anchor = html.find("id=\"here\"").expect("anchor missing");
        let target = html.find("target").expect("text missing");
        let bold = html.find("bold").expect("text missing");
        assert!(
            bold < anchor,
            "anchor drifted above unrelated markup: {html}"
        );
        assert!(anchor < target, "anchor landed after its text: {html}");
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
    fn toggle_task_ticks_and_unticks() {
        assert_eq!(toggle_task("- [ ] a\n", 1, true).unwrap(), "- [x] a\n");
        assert_eq!(toggle_task("- [x] a\n", 1, false).unwrap(), "- [ ] a\n");
        // Uppercase is accepted on the way in, and normalised on the way out.
        assert_eq!(toggle_task("- [X] a\n", 1, false).unwrap(), "- [ ] a\n");
    }

    /// The edit is a three-byte swap on one line of the original text, so the
    /// file's line endings — CRLF here — come through untouched.
    #[test]
    fn toggle_task_leaves_the_rest_of_the_file_alone() {
        let md = "# Title\r\n\r\n- [ ] one\r\n- [ ] two\r\n";
        assert_eq!(
            toggle_task(md, 3, true).unwrap(),
            "# Title\r\n\r\n- [x] one\r\n- [ ] two\r\n"
        );
    }

    #[test]
    fn toggle_task_finds_the_marker_in_every_list_shape() {
        assert_eq!(
            toggle_task("- [ ] a\n    - [ ] b\n", 2, true).unwrap(),
            "- [ ] a\n    - [x] b\n"
        );
        assert_eq!(toggle_task("1. [ ] a\n", 1, true).unwrap(), "1. [x] a\n");
        assert_eq!(toggle_task("1) [ ] a\n", 1, true).unwrap(), "1) [x] a\n");
        assert_eq!(toggle_task("+ [ ] a\n", 1, true).unwrap(), "+ [x] a\n");
        assert_eq!(toggle_task("* [ ] a\n", 1, true).unwrap(), "* [x] a\n");
        assert_eq!(toggle_task("> - [ ] a\n", 1, true).unwrap(), "> - [x] a\n");
    }

    /// The line the frontend sends is checked against the file rather than
    /// trusted, so anything that is not a task item is refused outright — a
    /// stale line number must never turn into an edit somewhere else. A box
    /// inside a code block is refused on the same grounds: it is text, not a
    /// task.
    #[test]
    fn toggle_task_refuses_a_line_without_a_box() {
        for line in [
            "hello [ ] there\n",
            "text - [ ] mid\n",
            "- item\n",
            "# [ ] heading\n",
            "\n",
        ] {
            assert!(toggle_task(line, 1, true).is_err(), "{line:?}");
        }
    }

    #[test]
    fn toggle_task_refuses_a_line_past_the_end() {
        assert!(toggle_task("- [ ] a\n", 9, true).is_err());
    }

    /// A bare `\r` ends a line for comrak, so the line it puts on the `<li>`
    /// counts it — and the byte the box is written to has to be found the same way.
    #[test]
    fn toggle_task_counts_lines_like_comrak() {
        let md = "# T\r\n\r- [ ] a\n- [ ] b\n";
        assert_eq!(
            toggle_task(md, 3, true).unwrap(),
            "# T\r\n\r- [x] a\n- [ ] b\n"
        );
        assert_eq!(
            toggle_task(md, 4, true).unwrap(),
            "# T\r\n\r- [ ] a\n- [x] b\n"
        );
    }

    /// Inside a code block `- [ ]` is content on the page, not a checkbox, so
    /// there is nothing there to flip.
    #[test]
    fn toggle_task_ignores_boxes_in_code() {
        assert!(toggle_task("```\n- [ ] x\n```\n", 2, true).is_err());
        assert!(toggle_task("    - [ ] x\n", 1, true).is_err());
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
