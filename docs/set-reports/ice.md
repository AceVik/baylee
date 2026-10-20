# Set Report — *Ice Age* (`ice`, Batches 1–4)

## 1. Cards per state

Out of 346 first-printing cards in *Ice Age* (`ice`):

- **120** previously `Coverage::Implemented` before this run.
- **1** skipped as unplayable (`data/unplayable.tsv`):
  - `Amulet of Quoz` (`9d152de1-8fb8-4012-8c9d-4c8d3072aa17`) — ante card (CR 407).
- **100** newly added and tested in Batches 1–4 (`Abyssal Specter` through `Icy Prison`):
  - **75** `Coverage::Implemented` (23 machine-transcoded by `scriptgen`, 52 hand-authored).
  - **25** `Coverage::Partial`, each with its reason and `// NOT SUPPORTED:` comment:
    1. `AdarkarUnicorn` — `"cumulative upkeep-only mana restriction on the second mode is not enforced"`
    2. `AegisOfTheMeek` — `"does not untap targeted 1/1 creature"`
    3. `AltarOfBone` — `"sacrificed creature is chosen by an effect during resolution rather than as an additional cost"`
    4. `ArcumSWeathervane` — `"changing a basic land's land subtype until end of turn is not supported"`
    5. `BatonOfMorale` — `"banding keyword is not modeled"`
    6. `BindingGrasp` — `"continuous gain control of enchanted creature is not modeled on Aura static"`
    7. `Blessing` — `"white-only mana activation restriction not enforced"`
    8. `BrandOfIllOmen` — `"controller-of-enchanted-creature creature spell restriction and cumulative upkeep not modeled"`
    9. `CallToArms` — `"sacrifice-when-most-white-permanents state trigger not supported"`
    10. `Chromatograph` — `"charge-counter-based cast permission not supported"`
    11. `ColdSnap` — `"snow-land-count damage to upkeep player not supported"`
    12. `CurseOf MaritLage` — `"tapping all Islands on enter not supported"`
    13. `DreamsOfTheDead` — `"granted cumulative upkeep and exile-if-leaves replacement not supported"`
    14. `Earthlink` — `"triggered mandatory land sacrifice when any creature dies not supported"`
    15. `Earthlore` — `"tapping an untapped land you control as an Aura activation cost not supported"`
    16. `ElementalAugury` — `"reordering top three cards of target player's library not supported"`
    17. `Formation` — `"banding keyword not supported"`
    18. `Foxfire` — `"preventing combat damage to and by attacking creature not supported on spell"`
    19. `GameOfChaos` — `"iterative coin-flip doubling life-total wager subgame not supported"`
    20. `GazeOfPain` — `"delayed unblocked-attacker combat damage replacement trigger not supported"`
    21. `GeneralJarkeld` — `"exchanging blockers between two blocked attacking creatures not supported"`
    22. `GhostlyFlame` — `"making black and/or red sources colorless sources of damage not supported"`
    23. `Hipparion` — `"conditional blocking mana tax not supported"`
    24. `IceCauldron` — `"exiling card from hand to cast later and noting/producing spent mana not supported"`
    25. `IcyPrison` — `"upkeep mana payment offered to controller rather than any player"`
- **125** remaining first-printing *Ice Age* cards (Batches 5–9, `Illusionary Forces` through `Zur's Weirding`) left outside `data/card-pool.txt` for subsequent batches after this PR lands.

Every finished pool card in `validate` is tested (`9343 cards conform (9298 finished — 1560 hand-owned, 7738 machine-owned, 0 of them untested — and 45 stubs)`).

## 2. DSL and Engine Additions

Committed in `10ed5d681` (`feat(engine): AtNextTurnsUpkeep, ManaUse::CumulativeUpkeep, Filter::HasCumulativeUpkeep`):

1. **`Effect::AtNextTurnsUpkeep { effects }`** (`DelayedWhen::NextTurnsUpkeep`):
   - Schedules a delayed triggered ability that fires at the beginning of the next turn's upkeep regardless of whose turn it is (CR 603.7b; *Ice Age* "slowtrip"rider: *"Draw a card at the beginning of the next turn's upkeep."*).
   - **Rule test**: `crates/baylee-engine/src/engine/mechanics_tests/wave3b_rules.rs::at_next_turns_upkeep_fires_on_the_very_next_turns_upkeep_even_if_opponents`.
   - **Proof of failure without change**: Before `DelayedWhen::NextTurnsUpkeep`, `DelayedWhen::NextUpkeep` checked `active == delayed.controller`, skipping the opponent's upkeep and delaying the draw until the caster's own next upkeep.
2. **`ManaUse::CumulativeUpkeep`** and **`Filter::HasCumulativeUpkeep`**:
   - Restricts mana produced with `ManaRestriction::OnlyFor(ManaUse::CumulativeUpkeep)` to paying cumulative upkeep costs (CR 702.24a), and matches permanents with a cumulative upkeep ability (`Filter::HasCumulativeUpkeep`).
   - **Rule test**: `crates/baylee-engine/src/engine/mechanics_tests/wave3b_rules.rs::cumulative_upkeep_mana_pays_cumulative_upkeep_and_has_cumulative_upkeep_filter_matches`.
   - **Proof of failure without change**: Without `ManaUse::CumulativeUpkeep` and `Filter::HasCumulativeUpkeep`, cumulative-upkeep-restricted mana could not be distinguished from unrestricted mana and `Filter::HasCumulativeUpkeep` did not exist.

## 3. Reader (`scriptgen`) Additions

Committed in `0271525bb` (`feat(codegen): read slowtrips, cumulative upkeep, PumpAll, snow landwalk`):

1. **Slowtrip delayed trigger (`DelTrig$` at next turn's upkeep)**:
   - Taught `scriptgen` to recognize `DB$ DelayedTrigger` with `Phase$ Upkeep`, `ValidPlayer$ Player`, `Execute$ TrigDraw` and emit `Effect::AtNextTurnsUpkeep { effects: &[Effect::draw(1)] }`.
2. **`K:Cumulative upkeep:<cost>`**:
   - Taught `Tx::keyword` in `crates/baylee-cards-codegen/src/scriptgen/keywords.rs` to transcode `K:Cumulative upkeep` for mana and life costs into the CR 702.24a upkeep trigger (`AddCounter(counters::AGE, 1)` + `PlayerMayPayManaEachThenElse` / `PlayerMayPayLifeEachThenElse` sacrificing self when unpaid).
3. **`SP$ PumpAll` / `AB$ PumpAll`**:
   - Taught `scriptgen` to transcode `PumpAll` with P/T deltas and keyword grants into `Effect::PumpFilter` / `Effect::ModifyMatching`.
4. **Snow-covered landwalk (`K:Landwalk: Mountain.Snow`, etc.)**:
   - Taught `Tx::keyword` to emit `static_ability!(Filter::This, Modifier::CantBeBlockedBy(&Filter::DefendingPlayerControls(&SNOW_<LAND>)))`.

Across Batches 1–4, these reader improvements moved **23** *Ice Age* cards into `Coverage::Generated` (all 23 subsequently tested and promoted to `Coverage::Implemented`).

## 4. Gate Commands and Results

- `cargo fmt --all -- --check`: green.
- `cargo run -p xtask -- validate`: green (`validate: 9343 cards conform (9298 finished — 1560 hand-owned, 7738 machine-owned, 0 of them untested — and 45 stubs)`).
- `echo '{}' | ./scripts/baylee-tools hooks require-card-tests`: green.
- `DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee ~/.baylee-locks/with-cargo-lock.sh ./scripts/gate.sh`: green.
