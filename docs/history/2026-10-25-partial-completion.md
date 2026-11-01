# Partial completion, first batch — 25 October 2026

The first eight-card batch passes focused card/rule tests, reader and reclaim
checks, and the local rules and full workspace gates. Whole-pool coverage is
now 10,066 Implemented, 286 Partial and 341 Unimplemented definitions. The
remaining 286 Partial cards are still open; this report covers this batch only.

## Eight completed definitions

| Card | Completed behavior | Canonical card tests |
| --- | --- | ---: |
| Arcum's Weathervane | Add or remove snow through layer-four supertype changes, preserving card types and other supertypes. | 3 |
| Bone Shaman | A paid activation grants a live prohibition against regenerating exact creatures it damaged this turn; ability loss, source departure and cleanup end the relevant effect. | 4 |
| Energy Storm | Prevent damage from instant/sorcery spells; damage from nonspell sources, including their abilities and combat, remains unaffected. Flying creatures skip untapping, with cumulative upkeep paid or declined normally. | 7 |
| Fire Covenant | Pay chosen life-X and divide that damage among legal creature targets, preserving each share through prevention, redirection and target loss. | 7 |
| Incinerate | Only creatures actually damaged by this resolution's source cannot regenerate this turn, including redirected recipients. | 5 |
| Lhurgoyf | Count creature cards in all graveyards, including itself when appropriate, with characteristic-defining P/T in every zone. | 3 |
| Lost Order of Jarkeld | Choose the opponent through the real entry decision and update its count from that opponent's creatures. | 3 |
| Shyft | Choose one or more colors for its exact source incarnation, independently of unrelated targets. | 3 |

The definitions use shared rule vocabulary. Added primitives are
`DealtDamageBySourceThisTurn`, `CantBeRegenerated`,
`DamagedThisWayCantBeRegenerated`, `AddSupertype`, `RemoveSupertype`,
`ColorSubject::Source`, `PtCount::InAllGraveyards` and
`PtCount::ChosenPlayerControls`. The vocabulary and its limits are in
`docs/card-dsl.md`. Continuous World-supertype tenure is still unsupported;
adding or removing a supertype alone does not complete the world rule.

## Coverage and ownership

The baseline whole pool contains 10,693 definitions: 10,058 Implemented,
294 Partial and 341 Unimplemented stubs. This batch changes eight hand-owned
definitions from Partial to Implemented. Fresh post-reclaim coverage is
**10,066 Implemented, 286 Partial and 341 stubs**, with the same total of
10,693. The source walk classified every definition, with no unclassified files.

Before reclaim, `xtask validate` measured **2,582 hand-owned, 7,770
machine-owned and 341 stubs**, with no untested finished definitions. Four
generic reader forms now fully read Arcum's Weathervane, Bone Shaman, Energy
Storm and Incinerate from their real upstream scripts. Selective reclaim
commit `96a52d420` returns exactly those four completed definitions to
generator ownership while preserving identity, pinned printing, path and
Implemented coverage. Fresh source measurement and native `xtask validate`
confirm **2,578 hand-owned, 7,774 machine-owned and 341 stubs**: 10,352 finished
definitions, none untested, and 341 stubs.

A separate read-only join of Ice Age's 346 first-printing Oracle identities
to the current source definitions and exclusions measures **265 Implemented,
49 Partial, 31 Unimplemented, 0 Absent and 1 Excluded** (Amulet of Quoz).
This is a current measurement, not a sum of historical authoring claims.

These are four reader-driven ownership transfers of already implemented
cards, with **zero additional coverage promotions through the reader**. The
eight Partial-to-Implemented changes above are the full coverage delta; the
newly accepted input forms do not complete any other corpus card by claim.

The reader refuses World-supertype changes while continuous World tenure is
unsupported. Source-static regeneration prevention accepts the exact
`Creature.DamagedBy` predicate. Damage-memory regeneration prevention accepts
only an initial damage instruction: the runtime's recipient facts cover the
resolution, so accepting a later memory reset would attribute earlier damage
incorrectly. These are acceptance boundaries, not keys to ignore.

## Validation

The parent serialized builds and reported **48/48 focused tests passing again
after reclaim**:
35 canonical card tests across the eight cards and 13 independent engine
tests: seven regeneration tests, four characteristic tests, one source-color
test and one chosen-opponent entry test.

Nine targeted mutations were detected by the corresponding regressions:

1. Remove the pre-event regeneration-prohibition snapshot.
2. Remove projection invalidation after choosing an opponent.
3. Disable source-color subject selection (`ColorSubject::Source => None`).
4. Remove supertype-filter dependency ordering.
5. Break the all-graveyards card count.
6. Ignore resolved regeneration prohibitions.
7. Ignore the live static regeneration prohibition.
8. Break the chosen-player permanent count.
9. Make supertype application a no-op.

The ninth mutation initially survived because the rule test used
`SupertypeSet::contains(BASIC.union(SNOW))`, which accepts any matching bit.
Replacing that assertion with exact set equality made the mutation fail with
Basic alone (`0x01`) instead of Basic plus Snow (`0x05`). Sources were restored
after the mutation runs. This was a defect in the test's assertion, not evidence
that the no-op implementation was correct.

The exact AI activation census and its eight-card attribution regression pass
at **2,782 readable / 3,492 total**. The implemented-card game sweep passed
for all **10,066 Implemented definitions**. The new historical public-filter
client regression passed (one test), headlessly without a client window.

The table pipeline then ran `codegen --tables` twice and
`codegen --tables --check`. `xtask validate` accepted all 10,693 definitions,
the registry-parallelism regression passed, and `require-card-tests` cleared
eight registered Write events. Generated commit `33fcecd22` updates ability
rows and both derived precon status files. Incinerate is the only batch card
in the 30-card Fifth Edition Starter Deck A (`5ED/deck-a`); its completion makes
that deck playable. No generated file was hand-edited.

The post-reclaim pipeline also passed both table-generation passes,
`codegen --tables --check`, native `xtask validate` and `require-card-tests`;
the regenerated files stayed unchanged.

Reader integration passed **191 scriptgen tests** and **382 total codegen
tests**, plus Clippy. Twenty-one new boundary tests and an updated existing
regression cover the accepted forms and strict refusals; the four real upstream
scripts were each fully read.
The move-only reader extraction is commit `4acc81d63`, separate from its
functional changes.

Selective reclaim's production, tests, documentation and complete four-card
preview were independently reviewed by Astra. Its **15 new offline boundary
tests**, **98 total xtask tests** and Clippy passed. Preview `--check` returned
the expected nonzero status because it proposed changes; all **15 SHA-256
file hashes** remained unchanged during preview. The bounded path checks the
current identity, pin and canonical path before writing and does not broaden
the requested four-card scope.

Both final gate commands exited 0 with the configured database:

| Command | Final result |
| --- | --- |
| `cargo dev gate rules` | Formatting, lint, tests and validate pass; 20,872 tests passed (7 slow), 17 skipped; test-summary time 529.531 seconds. |
| `cargo dev gate` | Formatting, Clippy, tests, Scryfall cache, validate, API generation and API tests all exit 0; 23,109 workspace tests passed (8 slow), 25 skipped; test-summary time 624.008 seconds. Four additional API tests passed. |

The corrected effects-test population also passed all **13 effects tests**.
The full gate again validates all 10,693 definitions with 2,578 hand-owned,
7,774 machine-owned, zero untested finished definitions and 341 stubs. Both
test summaries retain their skipped populations above.

`cargo dev gate features` was not run. It remains mandatory before any later
push or land under `AGENTS.md`; neither push nor land was performed here.

## Independent review findings

Integration checks caught stale census and generated ability-row expectations,
plus two extraction-related lint errors; all were corrected. The complete
modifier test census also needed the three added variants: two supertype
changes lock their set, while regeneration prohibition is rules-only. Its
exact populations are now 160 modifiers, 49 locking and 111 rules-only;
all 13 effects tests and the final rules gate verify the corrected list. The activation
delta from 2,783/3,490 is exact: Arcum adds two unread supertype activations,
while Bone Shaman's formerly empty readable activation becomes a contextual
static grant the AI cannot price. The other six cards add no activation or
loyalty rows. The regression pins all eight members, the empty-effect baseline,
and a readable Jayemdae Tome control without inventing production AI values.

Sol's engine review found that processing simultaneous destruction one object
at a time could remove the no-regeneration source before checking a later
victim. The fix captures the prohibition on the pre-event board and tests both
source/victim orders for destroy-all and lethal-damage state-based actions.
A dedicated effect-table hint skips the added battlefield scan in ordinary
games without regeneration prohibitions. The review also identified the
World-supertype timing boundary, now documented explicitly.

The real entry-choice test exposed a stale characteristic projection after
choosing an opponent. The action now invalidates projections before the
chosen-player count is read. Layer/count extraction remains separate from
the rule changes so the behavioral diff is reviewable.

## Rules and legal basis

Rules are checked against the published
[25 September 2026 Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt).
Existing identifying headers and notices remain in place. This batch adds
original rule implementation and tests without copied corpus code, artwork,
fonts, audio or altered card images. The existing decisions in `docs/legal.md`
remain applicable. The validation setup agent read the official
[Wizards Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
on 25 October: “Tell the Community it’s unofficial.” Existing unofficial
notices remain in place, and the recorded owner decision of 2 October in
`docs/legal.md` remains the basis for independent rule implementation.

The [Scryfall API/image terms](https://scryfall.com/docs/api) returned HTTP 403
through the web reader but HTTP 200 twice through direct HTTPS. The retrieved
page states: “Your software must create additional value for end-users.”
Existing image-integrity and artist/copyright notices remain unchanged.
The external reference corpus was clean and used its upstream origin;
the owner's fork was not used. Its attribution remains in `NOTICE`.
