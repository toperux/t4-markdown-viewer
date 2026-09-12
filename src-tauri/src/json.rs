//! JSON shown as source rather than rendered: highlighted, foldable, and — for
//! a file too big to put on screen in one go — handed over a chunk at a time.
//!
//! The highlighting happens here instead of in the webview so that a 30 MB dump
//! never reaches highlight.js: the classes emitted are the ones highlight.js
//! would have produced (`hljs-attr`, `hljs-string`, …), so every theme already
//! styles them and nothing downstream has to know JSON is special.
//!
//! Nothing in here parses JSON. A viewer has to show what is in the file —
//! comments, key order, a trailing comma, a file that is not valid JSON at all —
//! so this is a tokenizer that never fails: it classifies what it recognises and
//! passes everything else through as escaped text.

use std::borrow::Cow;

/// How much source one render covers before the rest is left to `more` buttons.
/// Big enough that an ordinary config file arrives whole, small enough that the
/// first screen of a huge one is there immediately.
// ponytail: one budget for the whole document; add per-container
// collapse-by-size if the first screen of a deeply nested file reads as noise.
const CHUNK_BYTES: usize = 512 * 1024;

/// The most a re-render will bring back in one go, however far the reader had
/// got. A document expanded to its end and then rewritten on disk as something
/// far bigger must not come back whole; the rest stays a click away.
const MAX_EXTENT: usize = 8 * 1024 * 1024;

/// How far past the budget the emitter will look for a better place to cut —
/// a comma between two records rather than inside one. Bounded so a record
/// bigger than this costs one ugly cut rather than a scan of the whole file.
const LOOKAHEAD: usize = 64 * 1024;

/// The fold control. It carries no text of its own — the marker is a CSS
/// `::before` keyed off `aria-expanded`, so the theme decides how it looks and a
/// selection copied out of the document does not pick up an arrow.
///
/// Two flavours: a bracket that opens its line gets the `gutter` class and the
/// marker hangs in the `pre` padding, so the indentation it heads stays
/// aligned; a bracket mid-line — `"a": [` in reflowed output, or `[[` — takes
/// its own width instead, so it never paints over what precedes it.
const FOLD_BUTTON: &str =
    r#"<button class="fold" aria-expanded="true" aria-label="Fold"></button>"#;
const GUTTER_FOLD_BUTTON: &str =
    r#"<button class="fold gutter" aria-expanded="true" aria-label="Fold"></button>"#;

/* ---------------- tokenizer ---------------- */

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A quoted run, key or value — which of the two is decided at emit time.
    Str,
    Num,
    /// `true`, `false` or `null`. Any other bare word is `Other`.
    Literal,
    /// `//` to end of line, or `/* */`. JSONC, and the reason serde is no use
    /// for any of this.
    Comment,
    /// One of `{ } [ ] : ,`.
    Punct,
    Space,
    /// Anything unrecognised, down to a single stray byte. Emitted as text.
    Other,
}

struct Token {
    kind: Kind,
    start: usize,
    end: usize,
}

/// A streaming tokenizer: `(kind, byte range)` and nothing else, so a 32 MB
/// document never becomes a `Vec` of tokens several times its own size.
///
/// Every branch ends at a byte offset the input actually has, and unterminated
/// constructs — a string or a `/*` with no end — run to the end of the input
/// rather than failing, so the iterator always advances and always stops.
struct Tokens<'a> {
    src: &'a str,
    at: usize,
}

impl<'a> Tokens<'a> {
    fn new(src: &'a str, at: usize) -> Self {
        Tokens { src, at }
    }
}

/// How many bytes the character starting with `b` occupies. Used to step over an
/// escaped character inside a string without landing inside it — slicing a
/// `&str` at a non-boundary is a panic, and this module is handed whatever is on
/// disk.
fn char_len(b: u8) -> usize {
    match b {
        0xf0..=0xf7 => 4,
        0xe0..=0xef => 3,
        0xc0..=0xdf => 2,
        _ => 1,
    }
}

impl Iterator for Tokens<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        let bytes = self.src.as_bytes();
        let start = self.at;
        let first = *bytes.get(start)?;

        let (kind, end) = match first {
            b'"' => {
                let mut i = start + 1;
                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' => {
                            i += 1;
                            if i < bytes.len() {
                                i += char_len(bytes[i]);
                            }
                        }
                        b'"' => {
                            i += 1;
                            break;
                        }
                        b => i += char_len(b),
                    }
                }
                (Kind::Str, i.min(bytes.len()))
            }
            // Deliberately loose: this accepts `1e10`, `-1.5e-3` and
            // `12345678901234567890` alike, and colours nonsense like `1.2.3`
            // as a number too. Parsing it would only let us disagree with what
            // the file says.
            b'-' | b'0'..=b'9' => {
                let mut i = start + 1;
                while i < bytes.len()
                    && matches!(bytes[i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
                {
                    i += 1;
                }
                (Kind::Num, i)
            }
            b'{' | b'}' | b'[' | b']' | b':' | b',' => (Kind::Punct, start + 1),
            b'/' if bytes.get(start + 1) == Some(&b'/') => {
                let end = bytes[start..]
                    .iter()
                    .position(|b| *b == b'\n')
                    .map(|at| start + at)
                    .unwrap_or(bytes.len());
                (Kind::Comment, end)
            }
            b'/' if bytes.get(start + 1) == Some(&b'*') => {
                let end = self.src[start + 2..]
                    .find("*/")
                    .map(|at| start + 2 + at + 2)
                    .unwrap_or(bytes.len());
                (Kind::Comment, end)
            }
            b if b.is_ascii_whitespace() => {
                let mut i = start + 1;
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                (Kind::Space, i)
            }
            b if b.is_ascii_alphabetic() => {
                let mut i = start + 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let kind = match &self.src[start..i] {
                    "true" | "false" | "null" => Kind::Literal,
                    _ => Kind::Other,
                };
                (kind, i)
            }
            b => (Kind::Other, start + char_len(b)),
        };

        self.at = end;
        Some(Token { kind, start, end })
    }
}

/* ---------------- source ---------------- */

/// The text the rest of this module renders, and the text `render_slice`'s
/// offsets index into.
///
/// Almost always the file itself: what is on disk is what the reader wants to
/// see, formatting included. The exception is a file written on one line —
/// how a great many `.json` files are written — which would otherwise be a
/// single unreadable line several megabytes wide. Blank lines and `//` lines
/// above it are a header, not layout, so a generated file that announces
/// itself before its one long line counts too. Anything with a second line of
/// its own was formatted by someone, and is left as they left it.
///
/// The reflow goes through the same tokenizer rather than through serde, so
/// `1e10` and `12345678901234567890` come back exactly as written, comments
/// survive, and a file that is not valid JSON is still pretty-printed instead of
/// rejected.
pub fn source(text: &str) -> Cow<'_, str> {
    let mut body = text
        .lines()
        .skip_while(|l| l.trim().is_empty() || l.trim_start().starts_with("//"));
    body.next();
    // Lazy, so a formatted file is recognised at its second line, not its last.
    if body.any(|l| !l.trim().is_empty()) {
        return Cow::Borrowed(text);
    }

    let mut out = String::with_capacity(text.len() + text.len() / 4);
    let mut depth = 0usize;
    for token in Tokens::new(text, 0) {
        let piece = &text[token.start..token.end];
        if token.kind == Kind::Space {
            // Layout is ours from here on, but a space that keeps two tokens
            // apart — `1 2`, `} // c` — is not layout, and dropping it would
            // fuse them into something the file does not say. Leading space
            // keeps nothing apart.
            if !out.is_empty() && !out.ends_with(|c: char| c.is_whitespace()) {
                out.push(' ');
            }
            continue;
        }
        if token.kind != Kind::Punct {
            out.push_str(piece);
            // A line comment owns the rest of its line. Its newline was a
            // Space token the file may not even have had (a header comment
            // above one long line), so it is put back here, or whatever
            // follows would be swallowed into the comment.
            if token.kind == Kind::Comment && piece.starts_with("//") {
                indent(&mut out, depth);
            }
            continue;
        }
        // Whatever space the file put before `:` or `,` is layout, and the
        // layout is ours from here on — unless it is the indent under a line
        // comment, where the separator has to stay on its own line or the
        // comment eats it. A closer trims for itself below.
        if matches!(piece.as_bytes()[0], b':' | b',') {
            let kept = out.trim_end_matches(' ').len();
            if !out[..kept].ends_with('\n') {
                out.truncate(kept);
            }
        }
        match piece.as_bytes()[0] {
            b'{' | b'[' => {
                out.push_str(piece);
                depth += 1;
                indent(&mut out, depth);
            }
            b'}' | b']' => {
                // The opener has already written a newline and an indent for a
                // body that turns out not to exist, so an empty container is
                // put back together here rather than looked ahead for.
                out.truncate(out.trim_end().len());
                depth = depth.saturating_sub(1);
                if !out.ends_with('{') && !out.ends_with('[') {
                    indent(&mut out, depth);
                }
                out.push_str(piece);
            }
            b',' => {
                out.push_str(piece);
                indent(&mut out, depth);
            }
            b':' => {
                out.push_str(piece);
                out.push(' ');
            }
            _ => out.push_str(piece),
        }
    }

    out.truncate(out.trim_end().len());
    out.push('\n');
    Cow::Owned(out)
}

fn indent(out: &mut String, depth: usize) {
    out.push('\n');
    for _ in 0..depth {
        out.push_str("  ");
    }
}

/* ---------------- rendering ---------------- */

/// A whole JSON document as an HTML fragment, ready to drop into the page.
///
/// Only the first `CHUNK_BYTES` of it, though — or `extent`, whichever is more:
/// past that the emitter stops at the next member boundary and leaves a `more`
/// button per open container. The webview asks for those through `json_region`,
/// so opening a 30 MB file costs the same as opening a 500 KB one.
///
/// `extent` is how far the document had already been loaded when it was last on
/// screen, and is what makes a re-render — a tab switched back to, a live
/// reload, F5 — keep the chunks those buttons had fetched instead of dropping
/// back to chunk one. The emitter trips at the first comma at or past its
/// budget, so a budget equal to where a chunk stopped trips at that same comma
/// again and reproduces exactly what was loaded, buttons and all.
// ponytail: fold state is per-render, so a live reload reopens every fold;
// carry the open ranges across if that turns out to be annoying in practice.
pub fn render_to(src: &str, extent: usize) -> String {
    let mut html = String::from("<pre><code>");
    html.push_str(&emit(src, 0, extent.clamp(CHUNK_BYTES, MAX_EXTENT)));
    html.push_str("</code></pre>");
    html
}

/// One chunk of a document, as named by a `more` button's `data-range`.
///
/// The offsets are into `source()`'s output rather than into the file, and the
/// range is always token-aligned — it starts at a comma or just past a closing
/// bracket — so the slice renders on its own with no state carried over from the
/// chunk before it. There is no `<pre><code>` wrapper: the caller splices this
/// in where the button was.
///
/// `get` rather than indexing, because the range may be stale: the file can have
/// been edited between the render that produced the button and the click on it.
/// A stale-but-valid range renders whatever text is now there, which is harmless
/// — the watcher's re-render replaces the whole document moments later anyway.
pub fn render_slice(src: &str, start: usize, end: usize) -> Result<String, String> {
    let slice = src
        .get(start..end)
        .ok_or_else(|| format!("Bytes {start}–{end} are no longer part of this document"))?;
    Ok(emit(slice, start, CHUNK_BYTES))
}

/// Render `src` as HTML, stopping once `budget` source bytes are behind us.
///
/// `base` is where `src` sits in the document `source()` produced, so the ranges
/// on `more` buttons are absolute however deep into a file this slice starts.
///
/// The container stack is explicit rather than a recursive descent: `{"a":{"a":{…`
/// nested ten thousand deep is a file someone can hand this app, and a recursive
/// emitter would meet it with a blown stack.
fn emit(src: &str, base: usize, budget: usize) -> String {
    let mut out = String::with_capacity(src.len().min(budget) * 2);
    // One entry per container we are inside: the byte that closes it, and
    // whether it opened a `.fold-body` span that has to be closed with it.
    let mut stack: Vec<(u8, bool)> = Vec::new();
    // Once past the budget: the comma the chunk ends at.
    let mut cut_at: Option<usize> = None;
    // Whether nothing but indentation precedes the current token on its line,
    // which is what decides where a bracket's fold marker goes.
    let mut at_line_start = true;

    for token in Tokens::new(src, 0) {
        let text = &src[token.start..token.end];
        let opens_line = at_line_start;
        at_line_start = token.kind == Kind::Space && (opens_line || text.contains('\n'));
        match token.kind {
            Kind::Space | Kind::Other => escape(text, &mut out),
            Kind::Num => span(&mut out, "hljs-number", text),
            Kind::Literal => span(&mut out, "hljs-literal", text),
            Kind::Comment => span(&mut out, "hljs-comment", text),
            // A string is a key if the next thing that is not whitespace or a
            // comment is a colon. That is the whole of the distinction: no
            // parse state, so it holds inside a slice that starts mid-object.
            Kind::Str => {
                let class = match after_trivia(src, token.end) {
                    Some(b':') => "hljs-attr",
                    _ => "hljs-string",
                };
                span(&mut out, class, text);
            }
            Kind::Punct => match text.as_bytes()[0] {
                byte @ (b'{' | b'[') => {
                    let closer = if byte == b'{' { b'}' } else { b']' };
                    // Nothing but whitespace inside means nothing to fold, so an
                    // empty container gets its two brackets and no control.
                    let folds = has_body(src, token.start, closer);
                    if folds {
                        out.push_str(if opens_line {
                            GUTTER_FOLD_BUTTON
                        } else {
                            FOLD_BUTTON
                        });
                    }
                    span(&mut out, "hljs-punctuation", text);
                    if folds {
                        out.push_str(r#"<span class="fold-body">"#);
                    }
                    stack.push((closer, folds));
                }
                // A closer that does not match what is open closes nothing: it
                // is a stray bracket in the text, and falls through to plain
                // punctuation below.
                byte @ (b'}' | b']') if stack.last().is_some_and(|(c, _)| *c == byte) => {
                    let (_, folds) = stack.pop().unwrap();
                    if folds {
                        out.push_str("</span>");
                    }
                    span(&mut out, "hljs-punctuation", text);
                }
                // Past the budget, a comma is where this chunk ends — a member
                // boundary, so the remainder can be rendered on its own. Not
                // the first one, though: the budget lands inside a record more
                // often than between two, and cutting there splits the record
                // across a button per open container, "… 0 KB more" included.
                // So the first comma past the budget looks ahead for the
                // shallowest one nearby, and the cut waits for that comma.
                // `start > 0` keeps a slice that opens on a comma from handing
                // back a button for itself.
                b',' if token.start >= budget && token.start > 0 => {
                    let at =
                        *cut_at.get_or_insert_with(|| shallowest_comma(src, token.start, &stack));
                    if token.start >= at {
                        let ends = container_ends(src, token.start, &stack);
                        let inner = ends.first().copied().flatten().unwrap_or(src.len());
                        // A remainder holding nothing but this comma and
                        // whitespace — a JSONC trailing comma, or the exact end
                        // of a container — is not worth a button, so carry on
                        // and trip at the next comma further out instead.
                        if has_content(src, token.start, inner) {
                            finish(&mut out, src, base, token.start, &ends, &stack);
                            return out;
                        }
                    }
                    span(&mut out, "hljs-punctuation", text);
                }
                _ => span(&mut out, "hljs-punctuation", text),
            },
        }
    }

    // An unclosed container's body ends where the input does. No bracket is
    // invented for it — the document does not have one — but the span it opened
    // is closed, so the markup handed to the webview is always balanced.
    for (_, folds) in stack.iter().rev() {
        if *folds {
            out.push_str("</span>");
        }
    }
    out
}

/// Wind up a chunk: a `more` button for the rest of every container we are
/// inside, innermost first, each inside the `.fold-body` it continues, followed
/// by that container's closing bracket. The document itself is the outermost
/// container — an implicit one — so a trailing comment or a file that is one
/// long scalar is covered by the last button, which sits directly in the `code`
/// element.
fn finish(
    out: &mut String,
    src: &str,
    base: usize,
    cut: usize,
    ends: &[Option<usize>],
    stack: &[(u8, bool)],
) {
    let mut start = cut;
    for (level, end) in ends.iter().enumerate() {
        more(out, src, base, start, end.unwrap_or(src.len()));
        let (_, folds) = stack[stack.len() - 1 - level];
        if folds {
            out.push_str("</span>");
        }
        match end {
            Some(at) => {
                span(out, "hljs-punctuation", &src[*at..*at + 1]);
                start = at + 1;
            }
            // This container never closes, so neither it nor anything outside
            // it has a remainder left to fetch.
            None => start = src.len(),
        }
    }
    more(out, src, base, start, src.len());
}

/// The control that fetches `src[start..end]` on click. Offsets are absolute and
/// decimal so the frontend can hand them straight back to `json_region`.
///
/// A remainder that is only a comma and whitespace — a JSONC trailing comma
/// after the container the cut fell in — is not worth a fetch, but it is still
/// part of the document, so it is written out here rather than dropped.
// ponytail: the label is button text, so copying a selection that spans it picks
// up "… 2.4 MB more"; move it to a pseudo-element if that grates.
fn more(out: &mut String, src: &str, base: usize, start: usize, end: usize) {
    if !has_content(src, start, end) {
        let rest = &src[start..end];
        match rest.strip_prefix(',') {
            Some(after) => {
                span(out, "hljs-punctuation", ",");
                escape(after, out);
            }
            None => escape(rest, out),
        }
        return;
    }
    out.push_str(r#"<button class="more" data-range=""#);
    out.push_str(&(base + start).to_string());
    out.push(':');
    out.push_str(&(base + end).to_string());
    out.push_str(r#"">… "#);
    out.push_str(&size(end - start));
    out.push_str(" more</button>");
}

/// How much is behind a `more` button, in the units someone reads a file size in.
fn size(bytes: usize) -> String {
    let (value, unit) = if bytes >= 1024 * 1024 {
        (bytes as f64 / (1024.0 * 1024.0), "MB")
    } else {
        (bytes as f64 / 1024.0, "KB")
    };
    let text = format!("{value:.1}");
    format!("{} {unit}", text.strip_suffix(".0").unwrap_or(&text))
}

/// Where each open container's closing bracket is, innermost first, or `None`
/// for one the document never closes.
///
/// One forward scan from the point the chunk stops at, which is all the emitter
/// needs to know to close its brackets and to work out what each ancestor has
/// left — no pre-pass over the document, and nothing retained from the walk so far.
fn container_ends(src: &str, from: usize, stack: &[(u8, bool)]) -> Vec<Option<usize>> {
    if stack.is_empty() {
        return Vec::new();
    }
    let mut ends: Vec<Option<usize>> = Vec::with_capacity(stack.len());
    let mut depth = 0usize;
    for token in Tokens::new(src, from) {
        if token.kind != Kind::Punct {
            continue;
        }
        match src.as_bytes()[token.start] {
            b'{' | b'[' => depth += 1,
            byte @ (b'}' | b']') => {
                if depth > 0 {
                    depth -= 1;
                } else if byte == stack[stack.len() - 1 - ends.len()].0 {
                    // Same rule as the emitter's: a bracket that does not match
                    // what is open closes nothing.
                    ends.push(Some(token.start));
                    if ends.len() == stack.len() {
                        return ends;
                    }
                }
            }
            _ => {}
        }
    }
    while ends.len() < stack.len() {
        ends.push(None);
    }
    ends
}

/// Whether the container opened at `open` holds anything but whitespace, which
/// is what decides whether it gets a fold control.
fn has_body(src: &str, open: usize, closer: u8) -> bool {
    src.as_bytes()[open + 1..]
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b != closer)
}

/// Where the shallowest comma within `LOOKAHEAD` bytes of `from` is — the
/// first of them, if several tie — given `stack` is what the emitter has open
/// there. Where the chunk should end: between two records rather than inside
/// one, if a record boundary is anywhere close.
///
/// Depth is counted relative to `from` and goes negative through ancestors,
/// by the emitter's own rule: a closer that does not match the container it
/// would close is a stray bracket and closes nothing. Counting it would name
/// a depth the emitter never reaches, and the cut would never come.
fn shallowest_comma(src: &str, from: usize, stack: &[(u8, bool)]) -> usize {
    let mut rel = 0isize;
    let mut best = (isize::MAX, from);
    for token in Tokens::new(src, from) {
        if token.start > from + LOOKAHEAD {
            break;
        }
        if token.kind != Kind::Punct {
            continue;
        }
        match src.as_bytes()[token.start] {
            b'{' | b'[' => rel += 1,
            byte @ (b'}' | b']') => {
                let open = stack.len() as isize + rel;
                if rel > 0 || (open > 0 && stack[open as usize - 1].0 == byte) {
                    rel -= 1;
                }
            }
            b',' if rel < best.0 => {
                best = (rel, token.start);
                // Outside every container: nothing is shallower than this.
                if stack.len() as isize + rel == 0 {
                    break;
                }
            }
            _ => {}
        }
    }
    best.1
}

/// Whether `src[start..end]` is worth a button: a leading comma and whitespace
/// are not content, anything else is.
fn has_content(src: &str, start: usize, end: usize) -> bool {
    let mut body = &src.as_bytes()[start..end];
    if let [b',', rest @ ..] = body {
        body = rest;
    }
    body.iter().any(|b| !b.is_ascii_whitespace())
}

/// The first byte from `from` on that is neither whitespace nor part of a
/// comment — the one thing a string needs to know to tell a key from a value.
fn after_trivia(src: &str, from: usize) -> Option<u8> {
    Tokens::new(src, from)
        .find(|t| !matches!(t.kind, Kind::Space | Kind::Comment))
        .map(|t| src.as_bytes()[t.start])
}

fn span(out: &mut String, class: &str, text: &str) {
    out.push_str("<span class=\"");
    out.push_str(class);
    out.push_str("\">");
    escape(text, out);
    out.push_str("</span>");
}

/// Everything from the document reaches the page through here. `"` is escaped
/// along with the rest even though nothing here puts document text in an
/// attribute, because the cost is nothing and the day someone does is not
/// the day to remember this.
fn escape(text: &str, out: &mut String) {
    let mut rest = text;
    while let Some(at) = rest.find(['&', '<', '>', '"']) {
        out.push_str(&rest[..at]);
        out.push_str(match rest.as_bytes()[at] {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            _ => "&quot;",
        });
        rest = &rest[at + 1..];
    }
    out.push_str(rest);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A document nothing has been expanded in yet — how every render but a
    /// re-render of an expanded one arrives.
    fn render(src: &str) -> String {
        render_to(src, 0)
    }

    /// The text a render puts on screen: tags gone, and `more` buttons gone with
    /// their labels, which are ours rather than the document's. Entities are
    /// left alone so the result can be compared against `escape`d source.
    fn strip(html: &str) -> String {
        let mut rest = html.to_string();
        while let Some(at) = rest.find(r#"<button class="more""#) {
            let end = rest[at..].find("</button>").expect("unclosed more button") + at;
            rest.replace_range(at..end + "</button>".len(), "");
        }
        let mut out = String::new();
        let mut tail = rest.as_str();
        while let Some(at) = tail.find('<') {
            out.push_str(&tail[..at]);
            let end = tail[at..].find('>').expect("unclosed tag") + at;
            tail = &tail[end + 1..];
        }
        out.push_str(tail);
        out
    }

    fn escaped(text: &str) -> String {
        let mut out = String::new();
        escape(text, &mut out);
        out
    }

    fn balanced(html: &str) -> bool {
        html.matches("<span").count() == html.matches("</span>").count()
            && html.matches("<button").count() == html.matches("</button>").count()
    }

    /// The range on the first `more` button, if there is one.
    fn first_range(html: &str) -> Option<(usize, usize)> {
        let at = html.find(r#"<button class="more" data-range=""#)?;
        let rest = &html[at + r#"<button class="more" data-range=""#.len()..];
        let range = &rest[..rest.find('"').unwrap()];
        let (start, end) = range.split_once(':').unwrap();
        Some((start.parse().unwrap(), end.parse().unwrap()))
    }

    /// What the frontend does with a `more` button, in a loop: fetch the range
    /// and splice the result in where the button was, until none is left.
    fn expand(src: &str, budget: usize) -> String {
        let mut html = emit(src, 0, budget);
        for _ in 0..1000 {
            let Some(at) = html.find(r#"<button class="more""#) else {
                return html;
            };
            let end = html[at..].find("</button>").unwrap() + at + "</button>".len();
            let (start, stop) = first_range(&html[at..]).unwrap();
            let chunk = emit(&src[start..stop], start, budget);
            html.replace_range(at..end, &chunk);
        }
        panic!("chunking never reached the end of the document");
    }

    #[test]
    fn every_token_kind_gets_its_class() {
        let html = render("{\"k\": \"v<&\\\"\", \"n\": 1e3, \"b\": true, \"z\": null}");
        assert!(html.starts_with("<pre><code>"), "{html}");
        assert!(html.ends_with("</code></pre>"), "{html}");
        assert!(
            html.contains(r#"<span class="hljs-attr">&quot;k&quot;</span>"#),
            "key not marked as one: {html}"
        );
        assert!(
            html.contains(r#"<span class="hljs-string">&quot;v&lt;&amp;\&quot;&quot;</span>"#),
            "string value or its escaping is wrong: {html}"
        );
        assert!(
            html.contains(r#"<span class="hljs-number">1e3</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="hljs-literal">true</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="hljs-literal">null</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="hljs-punctuation">:</span>"#),
            "{html}"
        );
        // No class on the `code` element: highlight.js keys off `language-`, and
        // this document must not be handed to it a second time.
        assert!(!html.contains("<code class"), "{html}");
    }

    #[test]
    fn comments_survive_and_are_marked() {
        let html = render("{\n  // one\n  \"a\": /* two */ 1\n}");
        assert!(
            html.contains(r#"<span class="hljs-comment">// one</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="hljs-comment">/* two */</span>"#),
            "{html}"
        );
        // The newline ending the line comment is whitespace, not part of it.
        assert!(html.contains("</span>\n  "), "{html}");
    }

    /// A key is a string a colon follows, whatever sits in between.
    #[test]
    fn a_comment_between_key_and_colon_does_not_hide_the_key() {
        let html = render("{\"a\" /* c */ : 1}");
        assert!(html.contains(r#"class="hljs-attr""#), "{html}");
    }

    #[test]
    fn containers_fold_unless_they_are_empty() {
        let html = render("{\"a\":1}");
        assert_eq!(html.matches(r#"<button class="fold"#).count(), 1, "{html}");
        assert_eq!(html.matches(r#"class="fold-body""#).count(), 1, "{html}");
        assert!(balanced(&html), "{html}");

        for empty in ["{}", "[ ]", "[\n\n]"] {
            let html = render(empty);
            assert!(!html.contains(r#"<button class="fold"#), "{empty}: {html}");
            assert_eq!(
                html.matches(r#"class="hljs-punctuation""#).count(),
                2,
                "{empty}: {html}"
            );
        }

        let html = render("{\"a\":[1],\"b\":[]}");
        assert_eq!(html.matches(r#"<button class="fold"#).count(), 2, "{html}");
        assert!(balanced(&html), "{html}");
    }

    /// A bracket that opens its line hangs its marker in the gutter; one
    /// mid-line takes its own width, so `[[` never stacks two arrows.
    #[test]
    fn only_a_bracket_opening_its_line_gets_the_gutter_marker() {
        // The root brace opens its line; `[`, `[[`, `[` and `{` all sit after
        // something on theirs.
        let html = render("{\n  \"a\": [[1], [2]],\n  \"b\": {\n    \"c\": 1\n  }\n}");
        assert_eq!(html.matches(r#"class="fold gutter""#).count(), 1, "{html}");
        assert_eq!(html.matches(r#"class="fold" "#).count(), 4, "{html}");
        // A matrix laid out one row per line: every bracket opens a line.
        let html = render("[\n  [1],\n  [2]\n]");
        assert_eq!(html.matches(r#"class="fold gutter""#).count(), 3, "{html}");
        // Line-start means indentation only: a tab counts, a value does not.
        assert!(render("\t[1]").contains("fold gutter"));
        assert!(!render("1 [1]").contains("fold gutter"));
        // However deep: the decision is made in passing, not by scanning back.
        let deep = "[\n".repeat(300) + "1" + &"\n]".repeat(300);
        assert_eq!(render(&deep).matches("fold gutter").count(), 300);
    }

    /// A bracket that closes nothing is text in the document, not structure.
    #[test]
    fn stray_closers_are_plain_punctuation() {
        let html = render("}");
        assert_eq!(
            html,
            r#"<pre><code><span class="hljs-punctuation">}</span></code></pre>"#
        );
        // The inner `}` must not close the outer object.
        let html = render("[1}2]");
        assert!(balanced(&html), "{html}");
        assert_eq!(html.matches(r#"class="fold-body""#).count(), 1, "{html}");
    }

    /// An unclosed container's body runs to the end of the input, and no bracket
    /// is invented — but the markup still balances.
    #[test]
    fn an_unclosed_container_still_balances() {
        for broken in ["{\"a\":1", "[1,[2", "{"] {
            let html = render(broken);
            assert!(balanced(&html), "{broken}: {html}");
            assert_eq!(strip(&html), escaped(broken), "{broken}: {html}");
        }
    }

    #[test]
    fn chunking_stops_at_a_comma_and_carries_on() {
        let src = "[1,2,3,4,5,6,7,8,9,10]";
        let html = emit(src, 0, 8);
        let (start, end) = first_range(&html).expect("no more button");
        assert_eq!(&src[start..start + 1], ",", "range must open on a comma");
        assert_eq!(end, src.len() - 1, "range must stop at the closing bracket");
        assert!(balanced(&html), "{html}");
        // The bracket is closed after the button, inside nothing but the code
        // element, so the first chunk is a complete document on its own.
        assert!(
            html.ends_with(r#"</span><span class="hljs-punctuation">]</span>"#),
            "{html}"
        );

        let chunk = render_slice(src, start, end).unwrap();
        assert!(
            !chunk.contains("<pre>"),
            "a slice carries no wrapper: {chunk}"
        );
        assert!(balanced(&chunk), "{chunk}");

        // Following every button to the end reproduces the document exactly.
        assert_eq!(strip(&expand(src, 8)), escaped(src));
    }

    /// The budget lands inside a record; the cut still falls between two.
    #[test]
    fn a_cut_prefers_the_shallowest_comma_nearby() {
        let records: Vec<String> = (0..20)
            .map(|i| format!("{{\"id\":{i},\"tags\":[\"a\",\"b\"],\"ok\":true}}"))
            .collect();
        let src = format!("[{}]", records.join(","));
        let html = emit(&src, 0, 100);
        assert_eq!(html.matches(r#"<button class="more""#).count(), 1, "{html}");
        let (start, _) = first_range(&html).unwrap();
        assert_eq!(&src[start..start + 2], ",{", "cut inside a record: {html}");
        assert_eq!(strip(&expand(&src, 100)), escaped(&src));
    }

    /// A stray closer in the lookahead must not name a depth the emitter never
    /// reaches — that would mean no cut at all, and the whole file in one go.
    #[test]
    fn a_stray_closer_does_not_defeat_the_cut() {
        let records: Vec<String> = (0..200).map(|i| format!("{{\"a\":{i}}}")).collect();
        let src = format!("[{},}},{}]", records[..3].join(","), records[3..].join(","));
        let html = emit(&src, 0, 10);
        assert!(html.contains(r#"class="more""#), "no cut at all: {html}");
        assert_eq!(strip(&expand(&src, 10)), escaped(&src));
    }

    /// With no shallower comma in reach, the cut is where the budget fell, and
    /// every open container gets a button for its remainder.
    #[test]
    fn a_nested_trip_leaves_a_button_on_every_open_container() {
        let pad = "x".repeat(LOOKAHEAD + 1);
        let src = format!("{{\"a\":[1,2,3,\"{pad}\",5],\"b\":2}}\n// tail\n");
        let src = src.as_str();
        let html = emit(src, 0, 10);
        assert_eq!(
            html.matches(r#"<button class="more""#).count(),
            3,
            "expected a button for the array, the object and the document: {html}"
        );
        assert!(balanced(&html), "{html}");
        assert!(
            html.contains(r#"<span class="hljs-punctuation">]</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<span class="hljs-punctuation">}</span>"#),
            "{html}"
        );
        // The document's own button covers what follows the root object.
        let last = html.rfind(r#"<button class="more" data-range=""#).unwrap();
        let (start, end) = first_range(&html[last..]).unwrap();
        assert_eq!(&src[start..end], "\n// tail\n");
        assert_eq!(strip(&expand(src, 10)), escaped(src));
    }

    /// A remainder that is only a trailing comma has nothing to fetch, so the
    /// chunk runs on to the end rather than leaving a button on an empty range.
    #[test]
    fn an_empty_remainder_gets_no_button() {
        let src = "[1,2,]";
        let html = emit(src, 0, 4);
        assert!(!html.contains(r#"class="more""#), "{html}");
        assert_eq!(strip(&html), escaped(src), "{html}");
    }

    /// A trailing comma left over on an *ancestor* after the cut is written out
    /// rather than fetched — and rather than dropped.
    #[test]
    fn a_trailing_comma_after_the_cut_is_still_shown() {
        let pad = "x".repeat(LOOKAHEAD + 1);
        let src = format!("{{\"a\":[1,2,3,\"{pad}\",5],\n}}");
        let src = src.as_str();
        let html = emit(src, 0, 8);
        assert_eq!(html.matches(r#"class="more""#).count(), 1, "{html}");
        assert_eq!(strip(&expand(src, 8)), escaped(src));
    }

    /// A slice's buttons have to name offsets in the whole document, not in the
    /// slice, or the second click would fetch the wrong bytes.
    #[test]
    fn slice_offsets_are_absolute() {
        let src = "[1,2,3,4,5,6,7,8,9,10,11,12]";
        let (start, end) = first_range(&emit(src, 0, 8)).unwrap();
        // What `render_slice` does, with a budget small enough to trip again.
        let chunk = emit(&src[start..end], start, 8);
        let (next, _) = first_range(&chunk).expect("the slice should chunk again");
        assert!(next > start, "offset is relative to the slice: {chunk}");
        assert_eq!(&src[next..next + 1], ",");
    }

    /// What `extent` is for: re-rendering with the cut a chunk stopped at as
    /// the budget brings back what had been fetched, rather than chunk one.
    #[test]
    fn a_budget_equal_to_a_cut_reproduces_the_loaded_chunks() {
        let src = "[1,2,3,4,5,6,7,8,9,10]";
        // The page after one click: the first chunk, with the slice its button
        // fetched spliced in where the button was.
        let first = emit(src, 0, 4);
        let (start, end) = first_range(&first).expect("no more button");
        let second = emit(&src[start..end], start, 4);
        let (next, _) = first_range(&second).expect("the slice should chunk again");
        let at = first.find(r#"<button class="more""#).unwrap();
        let stop = first[at..].find("</button>").unwrap() + at + "</button>".len();
        let mut page = first.clone();
        page.replace_range(at..stop, &second);

        // The innermost button left is at `next`, which is what the frontend
        // remembers — and as a budget it trips at that same comma again.
        assert_eq!(strip(&emit(src, 0, next)), strip(&page));
        let html = render_to(src, next);
        assert!(html.starts_with("<pre><code>"), "{html}");
        assert!(strip(&html).starts_with(&escaped(&src[..next])), "{html}");
    }

    #[test]
    fn a_stale_range_is_an_error_rather_than_a_panic() {
        let src = "[\"é\"]";
        assert!(render_slice(src, 0, 99).is_err(), "past the end");
        assert!(render_slice(src, 3, 1).is_err(), "backwards");
        // Byte 3 is the middle of `é`: indexing there would panic.
        assert!(render_slice(src, 0, 3).is_err(), "mid-character");
        assert!(render_slice(src, 0, src.len()).is_ok());
    }

    #[test]
    fn garbage_never_panics() {
        for bad in [
            "\0",
            "\u{0}\u{1}\u{2}",
            "\"unterminated",
            "\"escaped at the end\\",
            "/* unterminated",
            "// unterminated",
            "{[{[{[",
            "]}]}",
            "🙂",
            "\"🙂\\🙂\"",
            "",
            "-",
            "1.2.3e",
            "undefined",
        ] {
            let html = render(bad);
            assert!(balanced(&html), "{bad:?}: {html}");
            assert_eq!(strip(&html), escaped(bad), "{bad:?}: {html}");
        }
    }

    #[test]
    fn a_one_line_document_is_reflowed() {
        let out = source("{\"a\":1,\"b\":[1e10,12345678901234567890]}");
        assert_eq!(
            out,
            "{\n  \"a\": 1,\n  \"b\": [\n    1e10,\n    12345678901234567890\n  ]\n}\n"
        );
    }

    #[test]
    fn reflow_keeps_comments_and_empty_containers() {
        let out = source("{/* c */\"a\":{},\"b\":[]}");
        assert!(out.contains("/* c */"), "{out}");
        assert!(out.contains("\"a\": {}"), "{out}");
        assert!(out.contains("\"b\": []"), "{out}");
    }

    /// Spaces the file uses to keep tokens apart are kept apart in the reflow.
    #[test]
    fn reflow_does_not_fuse_space_separated_tokens() {
        assert_eq!(source("[1 2]"), "[\n  1 2\n]\n");
        assert_eq!(source("{\"a\":1} // c"), "{\n  \"a\": 1\n} // c\n");
        assert_eq!(source("/* c */ 1"), "/* c */ 1\n");
        // Layout spaces are still ours: none survive around the brackets.
        assert_eq!(
            source("{ \"a\" : [ 1 , 2 ] }"),
            "{\n  \"a\": [\n    1,\n    2\n  ]\n}\n"
        );
    }

    /// Leading space is not a separator, and a formatted file with one big
    /// value is still a formatted file.
    #[test]
    fn reflow_decides_by_lines_not_by_length() {
        assert_eq!(source("\n\n{\"a\":1}\n"), "{\n  \"a\": 1\n}\n");
        assert_eq!(source("  [1]"), "[\n  1\n]\n");
        let cert = "x".repeat(30 * 1024);
        let formatted = format!("{{\n  \"name\": 1,\n  \"cert\": \"{cert}\"\n}}\n");
        assert!(matches!(source(&formatted), Cow::Borrowed(_)));
    }

    /// A re-render never brings back more than `MAX_EXTENT` at once.
    #[test]
    fn extent_is_clamped() {
        assert!(render_to("[1]", usize::MAX).contains(r#"class="hljs-number">1<"#));
    }

    /// A header comment above one enormous line is still a one-line file.
    #[test]
    fn a_long_line_behind_a_header_is_reflowed() {
        let body: String = (0..500).map(|i| format!("\"k{i}\":{i},")).collect();
        let src = format!("// generated\n{{{body}\"end\":0}}\n");
        let out = source(&src);
        assert!(matches!(out, Cow::Owned(_)), "not reflowed");
        assert!(out.starts_with("// generated\n{\n  \"k0\": 0,\n"), "{out}");
    }

    /// A file that already has newlines is shown exactly as written — no copy is
    /// even made of it.
    #[test]
    fn a_multi_line_document_is_left_alone() {
        let src = "{\n\t\"a\":1,\n\n\n  \"b\" : 2\n}\n";
        assert!(matches!(source(src), Cow::Borrowed(_)));
        assert_eq!(source(src), src);
    }

    #[test]
    fn reflowing_garbage_does_not_panic() {
        for bad in ["{\0[\"unterminated", "]}]}", "", "   ", "/* c", "🙂🙂"] {
            let out = source(bad);
            assert!(out.ends_with('\n') || out.is_empty(), "{bad:?}: {out:?}");
        }
    }
}
