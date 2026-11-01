# Argon: next release-order card batch after PR #394

Use this as the next task prompt after the reviewed PR has landed.

Continue Baylee card implementation on a fresh branch from current `main`,
starting after Urza’s Destiny batch 1 (`Apprentice Necromancer` through
`Gamekeeper`). Select the next 25 still-absent first-printing identities in ledger order,
starting with `Goblin Festival`; skip already Implemented/Partial cards,
excluded identities and reviewed holds. Do not reopen a held card without
resolving its recorded cause, or reorder sets. Report the exact selected names
before editing. Keep the batch small
and wait for a different model’s review before starting another batch.

Read `AGENTS.md`, all of `docs/cards.md`, `docs/card-agent-brief.md`, the
normative coverage/file-ownership sections of `docs/card-dsl.md`,
`docs/card-identity.md`, `docs/legal.md` and the last three sections of
`docs/llm-learnings.md`. Read the findings in
`docs/history/2026-10-23-argon-review.md` as examples of defects to avoid.
Search existing DSL definitions AND their engine consumers before declaring
any sentence unsupported. Fix the reader first where it can transcode the
rule. Never edit a generated card by hand or copy the external corpus.

For every card, account for every Oracle sentence: controller/owner,
mandatory targets and counts, costs at announcement, symmetric effects,
opponent choices, restrictions, duration boundaries and object incarnations.
`Partial` does not permit a missing mandatory cost, drawback, target or
opponent benefit. If an entire optional positive ability cannot be expressed,
a useful body may remain with an explicit Partial reason. Otherwise restore
a canonical unimplemented stub using `codegen --hold-back` with the concrete
reason. Do not keep empty artifacts/enchantments, useless 0/0 bodies or
approximations that protect the caster from their own printed downside.

Use exact existing primitives. Distinguish damage prevention from assigning
no damage; distinguish controller-qualified last-counter removal from any
remover. A source’s current zone object is not its original battlefield
incarnation. Printed “your next untap step” belongs to the effect controller,
even after control changes. A printed quantity depending on a sacrificed
source must use its last known battlefield characteristics, including buffs
and counters actually remaining after costs. Do not substitute counter names.
Target selection and resolution-time choice are separate doors.

Play each retained card in its own mirrored engine test. Observe its actual
question/offer and resulting action, not only CardDef fields. Include the
negative boundary that would catch a missing own-side drawback, an opponent
object, insufficient mana/counters, an illegal target or a later incarnation.
Use real cast/activation/control-change paths. Offer lists depend on available
objects and floating mana. Do not assume a dynamic granted ability has index
zero. Handle combat damage assignments and optional questions explicitly.
Echo payment must be funded at the correct upkeep priority window, before
the resolution asks whether to pay. Keep assertions faithful to the printed
rule when repairing a test fixture.

Regenerate `codegen --tables` twice after hand-owned card changes. Never run
full codegen without the clean upstream corpus and printing cache. Run the
required card-test guard, validation and rules gate; before any push run all
three local gates (`cargo dev gate`, `cargo dev gate features`,
`cargo dev gate wasm`, with the documented DATABASE_URL where required).
Serialize heavy Cargo work with the machine’s cargo lock. Give every card
commit `Card-Author: <actual model>` and `Review: pending`. Report exact
Implemented/Partial/held counts, genuine DSL gaps and test results. Do not
self-review, merge or begin the next batch before independent review closes.
Hand substantial engine problems to Astra as requested by the owner.

At the reviewed snapshot, the next 25 absent identities are: `Goblin Festival`, `Goblin Marshal`, `Hulking Ogre`, `Hunting Moa`, `Impatience`, `Incendiary`, `Iridescent Drake`, `Ivy Seer`, `Jasmine Seer`, `Junk Diver`, `Keldon Champion`, `Keldon Vandals`, `Landslide`, `Lurking Jackals`, `Magnify`, `Mark of Fury`, `Metalworker`, `Metathran Elite`, `Metathran Soldier`, `Momentum`, `Multani's Decree`, `Nightshade Seer`, `Opalescence`, `Pattern of Rebirth`, `Phyrexian Negator`. Reconfirm this list against current main before editing.
