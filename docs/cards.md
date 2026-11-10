# Cards: codegen, layout, ownership and writing a card

The rules every card change is held to, whoever makes it. An agent on card
work reads `docs/card-agent-brief.md` first (it adds the defects earlier
batches shipped), then this page, then `docs/card-dsl.md` (the vocabulary)
and `docs/card-identity.md` (indices and who may write what).

- Codegen writes stubs, each group crate's `src/lib.rs` and `Cargo.toml` (`crates/cards/sets/<group>/`), the pool's `generated.rs` and `Cargo.toml` (`crates/cards/pool/`), `generated_tokens.rs` (in `crates/cards/src/`), `crates/baylee-core/src/generated/` (`subtypes.rs`; `index/`, per-set files globbed into `index::`; prefix rule and door test: `docs/card-identity.md` §"Who may write what") and four compiled-pool tables: `generated_lines.rs` (`docs/client.md` §"Which ability is on the stack"), `generated_names.rs`, `generated_sides.rs`, `generated_oracle.rs`. Two-phase: run codegen twice; `decks::name_table_tests` fail in between.
- Edit only `coverage`, `keywords`, `abilities`; the rest is generated, except a transforming DFC back face's `keywords`, `color_indicator`, `castable_from_hand`.
- Name the index as a constant (`index = index::MOX_OPAL`); a number does not compile.
- `CardIndex` is the append-only ledger `baylee_cards_index::ROWS` (`crates/baylee-cards-index`) over the whole corpus; decks and replays store it, so it never changes. Only `xtask ledger` writes it; codegen fails on a rowless card. Pool cards the corpus filter drops go in additive `data/corpus-keep.tsv`.
- The engine must not link `baylee-cards-index` (own crate, not a core feature). The pool's name table is in `baylee-cards`. `docs/card-identity.md` is normative.
- `validate` holds the `//!` header against the `CardDef`, and oracle text, `pool::type_line(face)`, printed costs (`Implemented` only) against Scryfall; header types use printed spelling (faces joined `" // "`). Never hand-type header oracle text; run `refresh-oracle`.
- DSL can't say it: `Coverage::Partial("reason")` + `// NOT SUPPORTED:`; prefer extending the DSL. `docs/card-dsl.md` is the contract. Update `docs/llm-learnings.md` after every batch.
- A card batch's author model writes the cards and their tests, and marks every such commit with the trailers `Card-Author: <model>` and `Review: pending`. A different model then reviews those commits against the oracle text and the brief, and fixes what is wrong in commits of its own that carry `Reviewed: <sha>…` for each commit it read (owner, 20.10.2026, replacing the rule of 29.09 that a card's author never writes its test). Unreviewed commits: `Review: pending` without a later `Reviewed:` naming them. The cheap-model card lanes (`scripts/llm/`, `tools/cardbatch/`) are removed. Sets go in release order by first printing, each by `docs/mechanics-roadmap.md` §E8; unplayable cards (ante, dexterity, subgame, un-set) are listed in `data/unplayable.tsv`, never built.

## Layout

- Codegen places files; never move one. `crates/cards/sets/<group>/src/<set>/<slug>.rs`: `<set>` is the first printing the ledger froze (`Row::set`), `<group>` its release year, from 2021 its quarter (`2021-q1`), front face slug (`layout.rs`; `docs/card-dsl.md` §"Where a card's file lives"). The rule reads no count, so adding cards never moves one.
- Release days are `baylee_cards_index::SETS`, written by `cargo xtask sets` (Scryfall's set list, cached). A set missing there stops codegen (`UnknownSet`): run it and commit the table.
- Each group is a crate (`baylee-cards-<group>`); its generated `src/lib.rs` declares every card with `#[path = …]` and re-exports `filters`, `generated_tokens`, `tokens` for `crate::…`. Codegen refuses orphan `.rs` under `crates/cards/sets` before moving anything, writes a new group's crate and its line in the pool's manifest, and removes an empty group.
- List card files with xtask's recursive `card_files`; read fields with `knob(content, field)`, never a literal `"<field> = "` (rustfmt wraps). A textual pool reader asserts a floor and a ceiling on its population (a misread marker inflates the count; a floor alone passes that).

## Readers and ownership

- Before a stub, codegen tries `landgen.rs` (printed land text) and `scriptgen.rs` (only `K:`, `A:`/`T:`, `S:` (partly), linked `SVar:`; names, costs, types, P/T come from Scryfall).
- CR 305.6 mana is an ability: every land with basic land types carries `landgen::intrinsic_mana_ability` (`stubgen::transcode_card` re-adds it after scriptgen); `casting::intrinsic_mana` covers one basic type, `None` for two.
- One unread clause (effect, parameter, computed `SVar`, data keyword) keeps the card `Coverage::Unimplemented`. Fully read is `Coverage::Generated` (playable, untested), or `Implemented` once a card test named after it exists (`card_tests/**/<slug>.rs`, `layout::tested_slugs`; a module root counts only with a `#[test]` of its own) or a test names its oracle id: codegen decides by that, so the next run after a test lands promotes the card (`docs/card-dsl.md` §"Coverage").
- `tokengen.rs` reads token scripts into `TokenDef` (abilities via scriptgen), refusing fields `TokenDef` lacks and token-making tokens. `generated_tokens::ALL` position is the art key; `tokenledger::assign` refuses same-named tokens with different abilities.
- The ceiling is our DSL. Rank corpus-wide to grow it, `--stubs` to finish cards; say which. Scripts count under their first refusal: judge a fix by its cause reaching zero and, separately, cards finished. Patch the top blocker out and re-measure first. A blocker is a sentence the DSL can't say; grep for existing machinery.
- Refusal causes come from `scriptgen::refusal_reason` (xtask `refusal_cause`). Name a refused value by its `SVar` definition, not letter. Only a rule reading a key claims it; never pad `PROSE_KEYS` or keep a per-rule key table.
- DSL facts: `Effect::AddMana` takes `Amount`/`combination` (`mana_dynamic`, `mana_choice_dynamic`, `mana_combination`). `.restricted(filter, rider)` is "Spend this mana only…"; a rider without "only" is `.when_spent(filter, rider)`: ordinary mana (CR 106.6) whose rider fires once per unit spent on a matching spell (106.6a). `DB$ Sacrifice` is `SacrificeSelf`; with `UnlessCost$`, `Effect::PlayerMayPayCostOr` (one `CostPart`; naming none declines). `cost_expr` (scriptgen) turns cost spellings (`Sac`, `tapXType`, …) into `CostPart`s; check it before blaming `cost_wizard`. `ChangeTypeDesc$` is claimed via `Params::claim_label` beside `ChangeType$`. Library search: `Optional$` = up to, `Mandatory$ True` = must, neither = refused.
- In a trigger `Amount::X` is `x.unwrap_or(0)`; `Tx::has_x` is per rules line. `S: Mode$ Continuous` becomes one `static_ability!` per layer.
- `stubgen::STUB_MARKER`/`OWNED_MARKER` files are rewritten every run; files with neither are hand-written, untouched. Fix wrong output in the reader, never the card. Only `stubgen::is_machine_owned` asks; never retype the markers. `validate` prints the hand-owned/machine-owned/stub split; watch it.
- After a reader gains an adopted card's complete rules,
  `cargo run -p xtask -- codegen --reclaim "<name>" --scripts <upstream-cardsfolder> --check`
  previews returning that named card to generator ownership. Repeat `--reclaim`
  for a small explicit selection. Every selected card must already be
  `Implemented`, fully transcode, retain its ledger identity, cached pinned
  printing and canonical path, and have accepted tests at its own path.
  All selected outputs are planned and every source is rechecked before the
  first card write. Omit `--check` to apply the reviewed selection; no downloads,
  other cards, registries or tables are written. A pending preview exits nonzero.
  Regenerate compiled tables separately with `codegen --tables`, then test and
  validate the generated definitions. Never restore ownership markers by hand.
- When a reader fully handles an existing machine-owned stub,
  `cargo run -p xtask -- codegen --complete-stub "<name>" --scripts <upstream-cardsfolder> --check`
  previews just that stub's normal generator output. Repeat `--complete-stub`
  for an explicit selection. Only unfinished `GENERATED STUB` sources qualify;
  hand-owned, Partial and already completed generated cards are refused.
  Coverage is `Generated` without accepted canonical tests and `Implemented`
  with them; an existing test hold prevents promotion. Like `--reclaim`, it
  pins the cached printing, Oracle ID, ledger index and canonical path, plans
  the whole selection and rechecks every original before the first write.
  Omit `--check` only after reviewing the complete preview. This mode never
  downloads, adopts cards, changes other cards, or regenerates tables/registries.
  A completed output is no longer an input to this stub-only mode.
- When a fully read machine-owned card is already `Coverage::Generated` and
  gains its accepted canonical test, `cargo run -p xtask -- codegen
  --promote-generated "<name>" --scripts <upstream-cardsfolder> --check`
  previews only its promotion to `Implemented`. Repeat the flag for an
  explicit selection. This mode requires the same complete script read,
  pinned printing, Oracle ID, ledger index, canonical path and unheld tests
  as reclamation; hand-owned cards, stubs and existing `Implemented` cards
  are refused. The whole selection is planned and every original is checked
  again before the first write. Applying changes only the selected cards,
  never tables, registries or other cards; run `codegen --tables` separately
  after reviewing and applying the preview. It is deliberately not an
  idempotent reclaim alias: a card already promoted is no longer its input.
- A reviewed unsafe hand-owned implementation can be restored only through
  `cargo run -p xtask -- codegen --hold-back "<name>" --hold-back-reason "<finding>"`
  (repeat names or pass `--hold-back-file`). This narrow mode needs cached
  pinned printings, never the corpus; it writes only selected canonical stubs
  with no abilities or coverage claim and records the reason. It refuses
  finished machine-owned cards, whose reader must be fixed instead. Regenerate
  compiled tables separately with `codegen --tables`.
- `cross-read` is a report (disagreement ≠ defect), blind to costs.

## Writing a card

- Open with `use baylee_cards_dsl::prelude::*;`; use the macros in `crates/baylee-cards-dsl/src/build.rs` (`card!`, `face!`, `activated!`, `triggered!`, `static_ability!`, `cost!`, …). Never restate a default. Parentheses and `field = value`, never braces or `field: value`. Fields without a rules default are positional.
- Defaults are rules (CR 117.1b, 113.6, 605.1). Use `mana_ability!`, not a flag; `lints::mana_ability_fault` (`mana_ability_fault_of`, also for `Modifier::GrantActivated`) rejects mana abilities that make no mana or target.
- Pool walks visit both grant doors, `AbilityDef::Static` and `Effect::CreateContinuousEffect`; population floors count doors, not instances.
- `static_ability!(filter, modifier)` has no layer (`Modifier::layer`); raw literals must agree (`lints::every_layer_in_the_pool_is_the_one_its_modifier_derives`). `equip!("{2}")` takes only the cost.
- `condition = Some(…)` gives `AbilityDef::ActivatedConditional`; every match handles both twins.
- `cost!("{1}{G}", TapSelf, SacrificeSelf)` in printed order; `Cost::FREE`, `Cost::TAP`. Readers emit costs only via `body::cost_literal`.
- Generic filters on `Filter` (`CREATURE`, `NONLAND`, …); pool-specific ones in `crates/cards/src/filters.rs` next to `crate::tokens` (Magic knowledge vs. this pool's). No per-card filter statics.
- Some tests exist only to fail on broken convention.
- Every card added, fixed or refactored is played in a file named after it under `crates/cards/testkit/card_tests/` (`<type>/<dir>/<slug>.rs`, filed by subject; the tests did not move with the cards), on the shared `testkit`, via `card_index("<oracle id>")` (where it compiles: §"Where a card test compiles"); `baylee-tools hooks require-card-tests` checks recursively (restored stubs and `Coverage::Generated` cards exempt; a Generated card whose test exists asks for a codegen run). Never in the card file. A fix also needs a test beside the broken rule that fails on the old code. Diff `pool-dump` around no-rules-change refactors.

## Where a card compiles

Since 10.11.2026 the cards compile apart from the libraries that read them, so a changed card rebuilds the cards and relinks the programs, and no library in between:

- `baylee-cards-<group>` (`crates/cards/sets/<group>`, §"Layout") compiles the cards first printed in that year or quarter: 52 groups on 10.11.2026, of 34 to 711 cards. A changed card rebuilds its own group and the pool's tables, and the groups compile in parallel: the pool built in 25 s on the MacBook where the one crate of all 12,545 cards took 54 s.
- `baylee-cards-pool` compiles the tables that name the cards (`generated.rs`) over every group, and exports them under fixed symbol names (`baylee_cards_pool_all`, `_by_index`, `_hash`). Only programs depend on it: a binary, a test binary, a bench, an example. Each takes `use baylee_cards_pool as _;` at its root; a library's unit tests take it under `#[cfg(test)]` with the pool as a dev-dependency.
- `baylee-cards-base` holds what a card file is written with (`tokens`, `generated_tokens`, `filters`) and `generated`, the door to the pool's tables: `unsafe extern "Rust"` statics of exactly the types the pool exports, which the pool holds to each other at compile time. A program that reads a card and does not link the pool fails to link, naming `baylee_cards_pool_all`; that is the whole fix.
- `baylee-cards` is the registry as libraries read it (`by_index`, `by_oracle_id`, decks, formats, lines, Oracle text, …), and re-exports `base`'s modules under their old paths. It no longer holds a card.

Measured back to back on the MacBook on 10.11.2026, before the groups (8b5e494ce) and with them (4fe2be85b): `hyperfine`, three runs after a warm-up, dev profile, a target directory of its own for each side, one appended comment line per run. A changed card rebuilt the client in 21.4 s instead of 25.6 s, its own card-test crate in 4.0 s instead of 10.8 s, and every test in 55.1 s instead of 62.1 s. A changed card test (3.0 s against 2.9 s for its crate) and a changed engine file (53.5 s against 52.4 s for every test) cost what they did.

A game record (replay, log) stores `CardIndex`es, which the ledger only appends to, and a replay on a later build is checked against `seen`, a digest of the public table written in those indices (`baylee-gamehost/src/record/seen.rs`). Neither names a group, a path or `POOL_HASH` (which only keys the client's `/pool` cache), so adding cards, or a whole new group, leaves every stored game readable.

## Where a card test compiles

The files stay where `card_tests/` puts them, but since 07.11.2026 they compile outside the engine: a changed test builds one small crate instead of the engine's test binary of 14,000 tests (42 s became 3 s on the MacBook, a changed card 52 s became 15 s for its crate and 28 s for all of them).

- `baylee-card-testkit` holds what the card tests share: the engine's `testkit` (compiled here and in the engine's own tests), `card_tests/mod.rs` with the kits it declares (`legends_kit`, `aura_kit`, …) and each door's vocabulary, `card_tests/<door>.rs`. Its feature `enabled` is off by default and only the test crates' dev-dependencies turn it on, so no build that ships links it or the engine's `test-support`.
- `baylee-card-tests-<part>` is one crate per `card_tests/shard_<part>.rs`, which is the crate's `lib.rs` (`[lib] path`) and names the children of a door it compiles. A part holds about 55,000 lines: one crate of all the creature tests did not finish a `cargo check` in nine minutes, 53,000 lines check in under one.
- `baylee-card-tests-verification` holds the verification hooks' own tests (`verification_tests.rs`, `card_rider_tests.rs`) and, compiled a second time under shorter paths (`instants::lightning_bolt::…`), the card tests they run as subjects.
- A door's own tests are in `card_tests/<door>/door.rs`, those of the top module in `card_tests/own.rs`.
- A new card test goes in its subject's folder as before (where its card was filed by type line until 10.11.2026), with a `mod` line in its parent. A new child of a door (a new `mv_N`, a new folder) also needs a `mod` line in a shard of that door: the smallest, unless that passes about 55,000 lines, then a new shard and crate.
- The engine internals the tests read are open only to them: fields, modules and re-exports behind the engine's `test-support` feature, in place, and functions and types `#[doc(hidden)] pub`, which reach no state without that feature. The compiler made the list (`docs/proposals/card-tests-split.md`).
