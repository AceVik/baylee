# Tower printing languages

`print-languages.json` was generated on 2026-11-16 from the 878 frozen
generation-6 Tower printings and public Scryfall metadata. It contains identifiers
and edition facts only: 557 exact German editions and 321 English fallbacks.
SHA-256: `a9cdb7acd1235643d6e2380c23c05d77ee4c8db71694d30e8d7c1bc08efb3e41`.

Each pair has the same Oracle identity, set and collector number; the German
record must name a real scan. A missing edition or placeholder does not borrow
art from a different printing. Foil treatment stays the run's earned treatment.
The runtime retains only sorted UUID pairs, approximately 18 KiB, and uses the
existing card-image cache and attribution. No card images are bundled here.

Reproduce with the Rust tool described in `docs/dev-tools.md`. Its cache contains
only the public metadata needed for this check. Fetching is explicit, serial,
bounded, rate-limited and fixed to `api.scryfall.com`; offline regeneration is the
default. The output is sorted by source UUID and has no generation timestamp.
