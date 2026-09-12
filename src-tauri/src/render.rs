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
    // YAML frontmatter is metadata for other tools, not content: without this
    // its `---` fences render as a rule and a setext heading.
    o.extension.front_matter_delimiter = Some("---".into());

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
                // HTML lets the value go unquoted, in which case it runs to the
                // first whitespace or to the tag itself. `is_safe_id` still has
                // the last word on what such a value may contain.
                let end = value
                    .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .unwrap_or(value.len());
                if end > 0 {
                    return Some(&value[..end]);
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
        .filter_map(|node| {
            let target = match &node.data.borrow().value {
                NodeValue::HtmlInline(raw) => anchor_target(raw),
                NodeValue::HtmlBlock(block) => anchor_target(&block.literal),
                _ => return None,
            };
            // An image's children are its alt text: comrak renders them rather
            // than dropping them, so this tag leaves no placeholder. Counting
            // it would push every later anchor onto the wrong one.
            node.ancestors()
                .all(|a| !matches!(a.data.borrow().value, NodeValue::Image(_)))
                .then_some(target)
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

/// Where the task box on 1-based `line` sits in `md`, or why that line has
/// none. The byte at the offset is the box character itself, so the caller can
/// write the new one over it without rebuilding the document.
///
/// comrak decides what a task item is, with the same options `render` used,
/// so the line the webview sends back — read off a `data-sourcepos` comrak
/// wrote — is matched against the same parse. A hand-rolled scan would be a
/// second definition of "task item": one that counts lines differently (comrak
/// ends a line on a bare `\r` too) and takes `- [ ]` inside a code block at
/// face value.
pub fn toggle_task(md: &str, line: usize) -> Result<usize, String> {
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
    line_start(md, symbol.line)
        .map(|start| start + symbol.column - 1)
        .filter(|&at| matches!(md.as_bytes().get(at), Some(b' ' | b'x' | b'X')))
        .ok_or_else(|| format!("Line {line} does not hold the box comrak saw"))
}

/// The Markdown behind the heading on 1-based `line`: the heading itself and
/// everything under it, down to the next heading of the same level or higher.
///
/// Same reasoning as `toggle_task` — comrak decides what a heading is, with the
/// same options `render` used, so the line the webview sends back is matched
/// against the parse that put the copy button there. A `#` inside a fenced
/// block is text, not a section.
///
/// A heading inside a quote or a list item has no sibling to close it, so it
/// owns the rest of that quote or item rather than the rest of the file.
pub fn section(md: &str, line: usize) -> Result<String, String> {
    let arena = Arena::new();
    let root = parse_document(&arena, md, &options());
    let (heading, level) = root
        .descendants()
        .find_map(|node| {
            let data = node.data.borrow();
            match &data.value {
                NodeValue::Heading(h) if data.sourcepos.start.line == line => Some((node, h.level)),
                _ => None,
            }
        })
        .ok_or_else(|| format!("Line {line} is not a heading"))?;

    // Deeper headings belong to this section; the first one at or above our
    // level ends it. `skip(1)` because the iterator starts at the heading itself.
    let next = heading.following_siblings().skip(1).find_map(|node| {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Heading(h) if h.level <= level => Some(data.sourcepos.start.line),
            _ => None,
        }
    });

    let start = line_start(md, line).ok_or_else(|| format!("Line {line} is past the end"))?;
    let end = match next {
        Some(at) => line_start(md, at).ok_or_else(|| format!("Line {at} is past the end"))?,
        // Nothing closes the section, so it ends where its container does: the
        // file for a top-level heading, the quote or item for a nested one. A
        // heading always has a parent, so the `else` is unreachable in practice.
        None => match heading.parent() {
            Some(parent) if !matches!(parent.data.borrow().value, NodeValue::Document) => {
                let last = parent.data.borrow().sourcepos.end.line;
                line_start(md, last + 1).unwrap_or(md.len())
            }
            _ => md.len(),
        },
    };
    // Whatever the file's line endings are, the copy keeps them.
    let eol = if md.contains("\r\n") { "\r\n" } else { "\n" };
    Ok(md[start..end].trim_end_matches(['\n', '\r']).to_string() + eol)
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
///
/// Same reasoning as `toggle_task` and `section`: comrak decides what a
/// heading is, with the same options `render` used, so the title is a heading
/// the reader can actually see. Frontmatter is metadata rather than a setext
/// underline, `#hashtag` is a word, an indented `#` is code, and the text
/// comes out of the heading's own nodes rather than off the raw line.
pub fn first_heading(md: &str) -> Option<String> {
    let arena = Arena::new();
    let root = parse_document(&arena, md, &options());
    root.descendants().find_map(|node| {
        if !matches!(node.data.borrow().value, NodeValue::Heading(_)) {
            return None;
        }
        // Inline markup contributes what it says, not how it is spelled. The
        // walk goes through an image too, so a heading holding nothing but a
        // picture is titled by its alt text — the one piece of that image the
        // author already wrote for somewhere it cannot be shown.
        let mut text = String::new();
        for inner in node.descendants() {
            match &inner.data.borrow().value {
                NodeValue::Text(t) => text.push_str(t),
                NodeValue::Code(c) => text.push_str(&c.literal),
                _ => {}
            }
        }
        let text = text.trim();
        (!text.is_empty()).then(|| text.to_string())
    })
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

    /// HTML lets an attribute value go unquoted, and documents in the wild
    /// write anchors that way; the value ends at whitespace or the tag.
    #[test]
    fn unquoted_attribute_anchors_work() {
        let html = render("<a id=f5></a>text\n");
        assert!(html.contains("<span id=\"f5\"></span>"), "{html}");
        // The unquoted value is still held to `is_safe_id`.
        let html = render("<a id=a+b></a>text\n");
        assert!(!html.contains("<span"), "unsafe id was reproduced: {html}");
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

    /// An image's alt children are rendered as plain text rather than dropped,
    /// so raw HTML in there leaves no placeholder. Counting it as a target
    /// would shift every later anchor onto the wrong one and lose the last.
    #[test]
    fn raw_html_in_image_alt_does_not_shift_later_anchors() {
        let html =
            render("![a <a id=\"ghost\"></a> b](i.png)\n\n<b>x</b>\n\n<a id=\"real\"></a>target\n");
        assert!(
            !html.contains("id=\"ghost\""),
            "alt-text tag became an anchor: {html}"
        );
        let anchor = html.find("id=\"real\"").expect("anchor missing");
        let target = html.find("target").expect("text missing");
        assert!(anchor < target, "anchor landed after its text: {html}");
    }

    /// The same risk from the other direction: a footnote nothing refers to is
    /// not rendered, so raw HTML inside it would leave no placeholder either.
    /// It does not desync anything today because comrak detaches the unused
    /// definition from the tree, so the walk never counts the tag in the first
    /// place — pinned here in case a comrak upgrade starts leaving it in.
    #[test]
    fn raw_html_in_an_unreferenced_footnote_does_not_shift_later_anchors() {
        let html = render("[^unused]: note <a id=\"ghost\"></a>\n\n<a id=\"real\"></a>target\n");
        assert!(
            !html.contains("note"),
            "the unused footnote was rendered: {html}"
        );
        assert!(
            !html.contains("id=\"ghost\""),
            "footnote tag became an anchor: {html}"
        );
        let anchor = html.find("id=\"real\"").expect("anchor missing");
        let target = html.find("target").expect("text missing");
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

    /// What the `toggle_task` command does with the offset: write one byte over
    /// the box. Asserting on the text it produces pins the offset down to the
    /// byte, which is all the command trusts it for.
    fn flip(md: &str, line: usize, checked: bool) -> Result<String, String> {
        let at = toggle_task(md, line)?;
        assert!(
            matches!(md.as_bytes()[at], b' ' | b'x' | b'X'),
            "offset {at} is not a box in {md:?}"
        );
        let mut out = md.to_string();
        out.replace_range(at..at + 1, if checked { "x" } else { " " });
        Ok(out)
    }

    #[test]
    fn toggle_task_ticks_and_unticks() {
        assert_eq!(toggle_task("- [ ] a\n", 1).unwrap(), 3);
        assert_eq!(flip("- [ ] a\n", 1, true).unwrap(), "- [x] a\n");
        assert_eq!(flip("- [x] a\n", 1, false).unwrap(), "- [ ] a\n");
        // Uppercase is accepted on the way in, and normalised on the way out.
        assert_eq!(flip("- [X] a\n", 1, false).unwrap(), "- [ ] a\n");
    }

    /// The edit is one byte written over the box in the original file, so the
    /// file's line endings — CRLF here — come through untouched.
    #[test]
    fn toggle_task_leaves_the_rest_of_the_file_alone() {
        let md = "# Title\r\n\r\n- [ ] one\r\n- [ ] two\r\n";
        assert_eq!(
            flip(md, 3, true).unwrap(),
            "# Title\r\n\r\n- [x] one\r\n- [ ] two\r\n"
        );
    }

    #[test]
    fn toggle_task_finds_the_marker_in_every_list_shape() {
        assert_eq!(
            flip("- [ ] a\n    - [ ] b\n", 2, true).unwrap(),
            "- [ ] a\n    - [x] b\n"
        );
        assert_eq!(flip("1. [ ] a\n", 1, true).unwrap(), "1. [x] a\n");
        assert_eq!(flip("1) [ ] a\n", 1, true).unwrap(), "1) [x] a\n");
        assert_eq!(flip("+ [ ] a\n", 1, true).unwrap(), "+ [x] a\n");
        assert_eq!(flip("* [ ] a\n", 1, true).unwrap(), "* [x] a\n");
        assert_eq!(flip("> - [ ] a\n", 1, true).unwrap(), "> - [x] a\n");
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
            assert!(toggle_task(line, 1).is_err(), "{line:?}");
        }
    }

    #[test]
    fn toggle_task_refuses_a_line_past_the_end() {
        assert!(toggle_task("- [ ] a\n", 9).is_err());
    }

    /// A bare `\r` ends a line for comrak, so the line it puts on the `<li>`
    /// counts it — and the byte the box is written to has to be found the same way.
    #[test]
    fn toggle_task_counts_lines_like_comrak() {
        let md = "# T\r\n\r- [ ] a\n- [ ] b\n";
        assert_eq!(flip(md, 3, true).unwrap(), "# T\r\n\r- [x] a\n- [ ] b\n");
        assert_eq!(flip(md, 4, true).unwrap(), "# T\r\n\r- [ ] a\n- [x] b\n");
    }

    /// Inside a code block `- [ ]` is content on the page, not a checkbox, so
    /// there is nothing there to flip.
    #[test]
    fn toggle_task_ignores_boxes_in_code() {
        assert!(toggle_task("```\n- [ ] x\n```\n", 2).is_err());
        assert!(toggle_task("    - [ ] x\n", 1).is_err());
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

    /// Without the frontmatter extension the closing `---` underlines the
    /// last key, so the title would be `title: Foo`.
    #[test]
    fn first_heading_skips_frontmatter() {
        let md = "---\ntitle: Foo\ntags: x\n---\n\n# Real\n";
        assert_eq!(first_heading(md), Some("Real".into()));
        assert_eq!(first_heading("---\ntitle: Foo\n---\n\ntext\n"), None);
    }

    /// comrak decides what a heading is, so the window title agrees with the
    /// page: a `#` with no space is a word, an indented `#` is code, and a
    /// `---` under a list is a rule rather than an underline.
    #[test]
    fn first_heading_ignores_what_comrak_does_not_call_a_heading() {
        assert_eq!(first_heading("#hashtag not a heading\n"), None);
        assert_eq!(first_heading("#hashtag\n\n# Real\n"), Some("Real".into()));
        assert_eq!(first_heading("    # indented code\n"), None);
        assert_eq!(first_heading("- item\n---\n"), None);
    }

    /// The text comes off the parsed heading, so inline markup inside it
    /// contributes what it says rather than how it is spelled.
    #[test]
    fn first_heading_reads_through_inline_markup() {
        assert_eq!(
            first_heading("# `code` in heading\n"),
            Some("code in heading".into())
        );
        assert_eq!(
            first_heading("# **bold** title\n"),
            Some("bold title".into())
        );
        assert_eq!(first_heading("# ![Logo](logo.png)\n"), Some("Logo".into()));
    }

    /// Frontmatter is metadata, not content: the opening fence is not a rule
    /// and the closing one does not make the last key a setext heading.
    #[test]
    fn frontmatter_is_hidden() {
        let html = render("---\ntitle: Foo\ntags: x\n---\n\n# Real\n");
        assert!(!html.contains("<hr"), "opening fence became a rule: {html}");
        assert!(!html.contains("title: Foo"), "frontmatter shown: {html}");
        assert!(html.contains("<h1"), "heading missing: {html}");
    }

    /// comrak skips the frontmatter but keeps counting its lines, so the line
    /// on a `data-sourcepos` is still the file's line — which is what
    /// `toggle_task` and `section` index into.
    #[test]
    fn frontmatter_keeps_file_line_numbers() {
        for eol in ["\n", "\r\n"] {
            let md = ["---", "title: Foo", "---", "", "# A", "- [ ] t", ""].join(eol);
            let html = render(&md);
            assert!(
                html.contains("<li data-sourcepos=\"6:1-"),
                "task item is not carrying its file line: {html}"
            );
            assert_eq!(
                flip(&md, 6, true).unwrap(),
                md.replace("- [ ] t", "- [x] t")
            );
            assert_eq!(section(&md, 5).unwrap(), format!("# A{eol}- [ ] t{eol}"));
        }
    }

    #[test]
    fn section_stops_at_a_sibling_heading() {
        assert_eq!(section("# A\ntext\n# B\nmore\n", 1).unwrap(), "# A\ntext\n");
    }

    /// A subsection is part of its parent, so the copy takes the lot.
    #[test]
    fn section_keeps_deeper_headings() {
        let md = "# A\n## A1\nx\n### A1a\ny\n# B\n";
        assert_eq!(section(md, 1).unwrap(), "# A\n## A1\nx\n### A1a\ny\n");
        assert_eq!(section(md, 2).unwrap(), "## A1\nx\n### A1a\ny\n");
    }

    /// Nothing follows the last heading, so it runs to the end of the file —
    /// and however many blank lines trail it, the copy ends in exactly one.
    #[test]
    fn section_runs_to_the_end_of_file() {
        assert_eq!(
            section("# A\n\n## B\ntail\n\n\n", 3).unwrap(),
            "## B\ntail\n"
        );
    }

    /// comrak puts a setext heading's line at its *text*, not its underline —
    /// which is the line `data-sourcepos` carries, so it is the line that comes
    /// back — and the underline is part of the section either way.
    #[test]
    fn section_keeps_a_setext_underline() {
        let md = "Title\n=====\ntext\n\nNext\n=====\n";
        assert_eq!(section(md, 1).unwrap(), "Title\n=====\ntext\n");
    }

    /// Inside a code block a `#` is content on the page, not a heading: line 1
    /// copies the fence whole, and line 3 names no section at all.
    #[test]
    fn section_ignores_headings_in_fences() {
        let md = "# A\n```\n# not a heading\n```\n# B\n";
        assert_eq!(section(md, 1).unwrap(), "# A\n```\n# not a heading\n```\n");
        assert!(section(md, 3).is_err());
    }

    #[test]
    fn section_refuses_a_line_without_a_heading() {
        assert!(section("# A\njust a paragraph\n", 2).is_err());
        assert!(section("# A\n", 9).is_err());
    }

    /// The bytes are sliced out of the original text, so a CRLF file keeps its
    /// line endings — the trailing run included, which is normalised to one.
    #[test]
    fn section_counts_lines_like_comrak() {
        assert_eq!(
            section("# A\r\ntext\r\n# B\r\n", 1).unwrap(),
            "# A\r\ntext\r\n"
        );
    }

    /// A heading in a quote has no sibling to stop at, so the quote stops it —
    /// and `text` is a lazy continuation of the quoted paragraph, so it is in.
    #[test]
    fn section_inside_a_quote_ends_with_the_quote() {
        let md = "# A\n> ## Nested\n> more\ntext\n# B\n";
        assert_eq!(section(md, 2).unwrap(), "> ## Nested\n> more\ntext\n");
    }

    /// Same for a list item: the section ends with the item, not the list.
    #[test]
    fn section_inside_a_list_item_ends_with_the_item() {
        let md = "# X\n- item\n  ## Y\n  more\n- next\n# Z\n";
        assert_eq!(section(md, 3).unwrap(), "  ## Y\n  more\n");
    }
}
