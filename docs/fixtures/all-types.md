# All Mermaid types

## flowchart

```mermaid
graph TD
subgraph icons [Sub One]
  A[Start] --> B{Is it?}
end
subgraph "tree x"
  C
end
B -->|Yes| C
B -.-> D((round))
click A href "#heading-b"
click C href "https://example.com" _blank
style D fill:#f9f
classDef foo fill:#0f0
class C foo
```

## sequence

```mermaid
sequenceDiagram
box Aqua Group One
participant Alice
actor Bob
end
Alice->>+Bob: Hello
Note over Alice,Bob: a note
loop Every
  Bob-->>-Alice: Hi
end
link Alice: Dash @ https://example.com
autonumber
```

## class

```mermaid
classDiagram
namespace ns {
  class Animal { +name }
}
Animal <|-- Dog
Dog *-- Tail
note for Dog "a note"
class Dog:::cls
click Dog href "#heading-b"
```

## state

```mermaid
stateDiagram-v2
[*] --> icons
state icons {
  a --> b
  state "with space" as ws
  b --> ws
}
state tree {
  c --> d
  --
  e --> f
}
icons --> tree
state fork_state <<fork>>
tree --> fork_state
note right of tree: note
tree --> [*]
```

## er

```mermaid
erDiagram
CUSTOMER ||--o{ ORDER : places
ORDER ||--|{ LINE-ITEM : contains
CUSTOMER { string name }
```

## gantt

```mermaid
gantt
dateFormat YYYY-MM-DD
section S1
Task one :t1, 2024-01-01, 3d
Task two :after t1, 2d
click t1 href "#heading-b"
```

## pie

```mermaid
pie title Pets
"Dogs" : 386
"Cats" : 85
```

## journey

```mermaid
journey
title T
section S
Task: 5: Me, You
```

## gitGraph

```mermaid
gitGraph
commit id: "a b"
branch dev
commit
checkout main
merge dev
commit tag: "v1"
```

## mindmap

```mermaid
mindmap
root((mindmap))
  Origins
    Long history
  Research
```

## timeline

```mermaid
timeline
title History
2002 : LinkedIn
2004 : Facebook : Google
```

## quadrant

```mermaid
quadrantChart
title Reach
x-axis Low --> High
y-axis Low --> High
quadrant-1 Expand
Campaign A: [0.3, 0.6]
```

## requirement

```mermaid
requirementDiagram
requirement test_req {
id: 1
text: the test text.
risk: high
verifymethod: test
}
element test_entity {
type: simulation
}
test_entity - satisfies -> test_req
```

## sankey

```mermaid
sankey-beta
A,B,10
B,C,5
```

## xychart

```mermaid
xychart-beta
title "Sales"
x-axis [jan, feb]
y-axis "Rev" 0 --> 100
bar [50, 60]
line [50, 60]
```

## block

```mermaid
block-beta
columns 2
a["A"] b["B"]
a --> b
```

## packet

```mermaid
packet-beta
0-15: "Source Port"
16-31: "Dest Port"
```

## kanban

```mermaid
kanban
todo[Todo]
  t1[Write]
done[Done]
  t2[Ship]
```

## architecture

```mermaid
architecture-beta
group api(cloud)[API]
service db(database)[Database] in api
service server(server)[Server] in api
db:L -- R:server
```

## c4

```mermaid
C4Context
title System
Person(customerA, "Customer A", "desc")
System(SystemAA, "System AA", "desc")
Rel(customerA, SystemAA, "Uses")
```

## radar

```mermaid
radar-beta
axis A, B, C
curve c1{1,2,3}
```

## treemap

```mermaid
treemap-beta
"Section 1"
  "Leaf 1.1": 12
"Section 2"
  "Leaf 2.1": 4
```

## venn

```mermaid
venn-beta
set A
set B
union A,B
```

## eventmodeling

```mermaid
eventmodeling
tf 01 ui ExampleScreen
tf 02 cmd DoThing
tf 03 evt ThingDone
```

## accessible

```mermaid
graph TD
accTitle: The title
accDescr: The description
A-->B
```

## ishikawa

```mermaid
ishikawa
Problem
  Cause A
    Sub
```

## oddFlow

```mermaid
graph LR
a.b --> c:d
subgraph s.1 [x]
  e_f
end
click a.b href "#x"
```

## oddState

```mermaid
stateDiagram-v2
state "Hello World" as h.w
[*] --> h.w
state co.mp {
  p.q --> r
}
h.w --> co.mp
```

## oddClass

```mermaid
classDiagram
class List~T~
class `Weird Name.x`
List~T~ <|-- `Weird Name.x`
```

## oddEr

```mermaid
erDiagram
"CUSTOMER X" ||--o{ ORDER.Y : places
```

## handDrawn

```mermaid
---
config:
  look: handDrawn
---
graph TD
A-->B
```

## neo

```mermaid
---
config:
  look: neo
---
graph TD
A-->B
```

## swimlaneBeta

```mermaid
swimlane-beta
subgraph icons [Lane A]
  a --> b
end
subgraph tree
  c
end
subgraph "x y"
  d
end
b --> c
```

## stateSwimlaneLayout

```mermaid
---
config:
  layout: swimlane
---
stateDiagram-v2
  state icons {
    a --> b
  }
  state tree {
    c --> d
  }
  icons --> tree
```

## flowSwimlaneLayout

```mermaid
---
config:
  layout: swimlane
---
flowchart LR
  subgraph icons
    a --> b
  end
  subgraph tree
    c
  end
  b --> c
```

## Heading B

This is heading B, the target of the click links above.

<!-- skipped: plan-themeCSS, balanced-themeCSS, quote-fontSize+themeCSS, quote-lineColor, quote-errorTextColor, frontmatter-quote, LOCKED quote-fontSize+themeCSS, LOCKED nested flowchart themeVariables -->
