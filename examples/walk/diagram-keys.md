# Diagram keyboard walk

A [plain link](https://example.com/before) before the diagrams.

## Flowchart with two links

```mermaid
flowchart LR
  A[First link] --> B[Second link] --> C[No link]
  click A "https://example.com/a"
  click B "https://example.com/b"
```

A [plain link](https://example.com/between) between the diagrams.

## Sequence diagram with actor menus

```mermaid
sequenceDiagram
  participant Alice
  participant Bob
  link Alice: Home @ https://example.com/alice
  link Alice: Docs @ https://example.com/alice/docs
  link Bob: Home @ https://example.com/bob
  Alice->>Bob: Hello
  Bob-->>Alice: Hi
```

## Taller than the window

```mermaid
sequenceDiagram
  participant Alice
  participant Bob
  link Alice: Home @ https://example.com/alice
  link Bob: Home @ https://example.com/bob
  Alice->>Bob: 1
  Bob-->>Alice: 2
  Alice->>Bob: 3
  Bob-->>Alice: 4
  Alice->>Bob: 5
  Bob-->>Alice: 6
  Alice->>Bob: 7
  Bob-->>Alice: 8
  Alice->>Bob: 9
  Bob-->>Alice: 10
  Alice->>Bob: 11
  Bob-->>Alice: 12
  Alice->>Bob: 13
  Bob-->>Alice: 14
  Alice->>Bob: 15
  Bob-->>Alice: 16
  Alice->>Bob: 17
  Bob-->>Alice: 18
  Alice->>Bob: 19
  Bob-->>Alice: 20
  Alice->>Bob: 21
  Bob-->>Alice: 22
```

A [plain link](https://example.com/after) after the diagrams.
