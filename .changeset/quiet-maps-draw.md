---
"fiducial-cli": minor
---

Derived diagrams: topology, topics, rules and storage

`fid-topology` can derive `topology.md` — the nodes grouped by system with the
buses between them, and the topics as a sequence between a node and the broker.
`fid-monitoring` can derive `rules.md` — what each rule reads and drives, and
the tables readings and settings are stored in. Both are Mermaid, both are
optional outputs, and both are gated by `fid derive --check` like every other
output, so a changed pin, topic or rule changes the picture in the same derive.
