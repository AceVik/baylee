# Bounded Ice Age completion — 25–26 October 2026

Against `origin/main`, 18 card definitions changed from Partial to Implemented:

- Initial eight: Arcum's Weathervane, Bone Shaman, Energy Storm, Fire Covenant,
  Incinerate, Lhurgoyf, Lost Order of Jarkeld and Shyft.
- Two further cards: Cloak of Confusion and Staff of the Ages.
- Final eight: Dance of the Dead, Magus of the Unseen, Ray of Command, Lava Burst,
  Meteor Shower, Fiery Justice, Soul Burn and Fumarole.

The restored focused run passed 274 tests, including the final eight cards'
28 canonical tests. The bounded stub-completion tool passed 37 preflight tests.
Post-fix focused checks passed 18 engine tests and both gateway seat-rate tests.
The final rules gate passed: 21,050 tests, 17 skipped, exit 0.
Full desktop gate: **23,301 passed, 25 skipped; four API tests passed; exit 0**.
Formatting, Clippy, tests, cache, validation and API checks all passed.
Feature gate: **Passed, exit 0**. Client: 2,290 passed, including 39 devctl
checks. Gateway devtable Clippy/tests, dev-dylink, test-support, train without
play, fuzz, mutate, verification-hooks-shipped and crypto-bigint-scope all passed.

Verification retained the failures as well as their corrections. An earlier
rules run exposed three engine history/fixture failures; these were corrected
and the 18 focused engine checks passed. The flood fixture was changed from
authenticated game envelopes to transport Pong frames, preventing per-frame
database authorization from pacing away the flood; its assertions were unchanged.
Both focused gateway checks and the subsequent complete rules gate passed.

The previous full desktop run had 23,299 passing tests, one failure and 25
skipped. Its sole failure was an idle disconnected lobby writing UI nodes.
Fix `63a1306b3`, reviewed in `81569e897`, then passed both focused client checks.
Those focused passes do not count as a passed full desktop gate; its final
rerun result is recorded above only after completion.

Eight deliberate, disjoint rule mutations each failed their designated test
(8/8 detected). Every mutated source was restored byte-for-byte, and the
restored combined run passed 274/274. These intended RED failures are
regression-sensitivity evidence, separate from the real gate failures above.

Ice Age has 383 printing rows and 373 distinct Oracle IDs: 302 Implemented,
39 Partial, 31 Unimplemented and 1 Excluded. Its 346 first-printing Oracle IDs
are 275 Implemented, 39 Partial, 31 Unimplemented and 1 Excluded. All 27 reprint
Oracle IDs are implemented; ten additional printing rows are basic-land art
variants. The remaining 70 rule cards are unfinished; the full set is not complete.

No stub adoption, Melting transfer or corpus-wide regeneration was performed.
