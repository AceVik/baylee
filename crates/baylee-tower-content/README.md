# Tower content adapter

`baylee-tower-content` prepares generation-four collections from the compiled,
tested card registry. The Tower authority and store remain independent of the
engine and card pool. Native admission independently rechecks Commander identity,
39 library cards plus one commander, a sideboard of at most five cards, singleton
nonbasics across all zones, and reference-print rules. Ownership belongs to the
authority's frozen collection and package commitment.

Five starters contain 22 nonbasic spells, 17 basics and one commander: Emmara
(green/white tokens), Elas il-Kor (white/black life and sacrifice), Talrand (blue
spells and tempo), Slimefoot (black/green Saprolings) and Pia and Kiran Nalaar
(red artifacts). All selected cards have `Coverage::Implemented`; this is a
support check, not evidence of measured balance or a completed playtest.

The optional `cli` feature links the complete registry for offline preparation:

```text
cargo run --locked -p baylee-tower-content --features cli -- --check
cargo run --locked -p baylee-tower-content --features cli -- --catalog
cargo run --locked -p baylee-tower-content --features cli -- --publication
```

`--publication` reads an exact executor `Identity` JSON from standard input,
bounded to 32 KiB. It refuses a mismatched rules or house-policy generation and
emits the cohort/catalog/starters payload. It neither starts services nor
publishes anything; the authenticated operator publication route retains its
existing authorization and immutable cohort checks. Never place credentials in
this input or output. `--check` explicitly reports `balance_measured: false`.
