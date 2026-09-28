# Sequence links

Some document text to click.

```mermaid
sequenceDiagram
  participant Alice
  participant Bob
  participant Dave
  participant Db@{ "type": "database" }
  link Alice: Repo @ https://example.com
  link Alice: Notes @ notes.md
  links Bob: {"Site": "https://example.org"}
  link Db: Docs @ https://example.net
  Alice->>Bob: Hello
  Bob->>Dave: Pass it on
  Dave->>Db: Store
  Db-->>Alice: Done
```

Text between the copies.

```mermaid
sequenceDiagram
  participant Alice
  participant Bob
  participant Dave
  participant Db@{ "type": "database" }
  link Alice: Repo @ https://example.com
  link Alice: Notes @ notes.md
  links Bob: {"Site": "https://example.org"}
  link Db: Docs @ https://example.net
  Alice->>Bob: Hello
  Bob->>Dave: Pass it on
  Dave->>Db: Store
  Db-->>Alice: Done
```

## Stick figure

```mermaid
sequenceDiagram
  actor Carol
  participant Eve
  link Carol: Home @ https://example.com
  Carol->>Eve: Hi
```
