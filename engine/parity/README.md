# Parity

While the engine is ported (PLAN step R), these scripts run the TypeScript engine and write down what it does, into
`engine/target/parity/`. The tests in `mana/tests/parity_*.rs` check that the Rust engine does the same, and skip when
there's nothing written down yet.

```
npm run parity        # writes engine/target/parity/, with the Node in $MANA_NODE (default: node)
npm run engine:test
```

Use an official Node build (nodejs.org) for the traces: one compiled for a newer CPU does its math a little differently
(see mana/tests/fixtures/v8-math.mjs). Everything here goes when the TypeScript engine does (step R7).
