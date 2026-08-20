# Kitchen Sink

A fixture exercising every construct the viewer renders. Cycle themes with
**F8** and check that nothing here goes unstyled or unreadable.

## Headings

# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6

## Inline text

Regular text with **bold**, *italic*, ***both***, ~~strikethrough~~,
`inline code`, and a [link to example.com](https://example.com). Bare URLs
autolink too: https://github.com

A footnote reference sits here[^note], and another one here[^second].

[^note]: Footnotes render in a block at the bottom of the document.
[^second]: With a back-reference link to the citation.

## Lists

- First item
- Second item
  - Nested item
  - Another nested item
    - Third level
- Third item

1. Ordered first
2. Ordered second
   1. Nested ordered
   2. Sibling
3. Ordered third

### Task list

- [x] Parse GFM extensions
- [x] Highlight fenced code
- [ ] Calibrate against a live wiki page
- [ ] Ship the installer

## Blockquote

> Blockquotes carry a left rule and muted text.
>
> They can span multiple paragraphs, and contain `code`, **bold**, and
> [links](https://example.com).
>
> > Nested quotes indent again.

## Table

| Language | Extension | Highlighted | Notes |
| --- | --- | :-: | --- |
| Rust | `.rs` | yes | The app shell |
| JavaScript | `.js` | yes | The webview side |
| CSS | `.css` | yes | Themes live here |
| Markdown | `.md` | no | What you are reading |

### A wide table (should scroll inside its own box, not the page)

| A | B | C | D | E | F | G | H | I | J | K | L |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| aaaaaaaaaa | bbbbbbbbbb | cccccccccc | dddddddddd | eeeeeeeeee | ffffffffff | gggggggggg | hhhhhhhhhh | iiiiiiiiii | jjjjjjjjjj | kkkkkkkkkk | llllllllll |

## Code

Rust:

```rust
/// Render Markdown to an HTML fragment.
pub fn render(md: &str) -> String {
    let mut o = Options::default();
    o.extension.table = true;
    o.extension.footnotes = true;
    markdown_to_html(md, &o)
}
```

JavaScript:

```js
const { invoke } = window.__TAURI__.core;

async function loadPath(path) {
  const doc = await invoke("load_file", { path });
  document.getElementById("content").innerHTML = doc.html;
}
```

CSS:

```css
.markdown-body pre {
  padding: 1em;
  border-radius: 4px;
  background: #f4f4f4;
}
```

Shell:

```sh
cargo test
cargo tauri build   # NSIS installer
```

A block with no language declared — highlighting is skipped, but the background
still comes from the theme:

```
$ plain preformatted text
  no tokens are coloured here
```

## Image

A relative image, resolved against this file's directory:

![The app icon](img/icon.png)

## Definition list

Term
: The definition of that term.

Another term
: Its definition, which can run long enough to wrap onto a second line and show
  how the indent behaves.

## Horizontal rule

---

## Long paragraph

Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor
incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis
nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.
Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu
fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in
culpa qui officia deserunt mollit anim id est laborum.

## Relative links

[Second document](second.md) — a link to a sibling `.md` loads in place and
pushes a history entry. **Back** should return here, at this exact scroll
position.

## Escaping

Raw HTML is stripped rather than rendered, because this app opens untrusted
files: <script>alert(1)</script> and <iframe src="https://example.com"></iframe>
both vanish, leaving the surrounding text intact. The same applies to benign
tags — `<kbd>`, `<sub>`, `<br>` and inline `style` attributes do not survive.
