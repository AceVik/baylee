//! The One Ring — {4} — Legendary Artifact
//! Oracle: Indestructible
//! Oracle: When The One Ring enters, if you cast it, you gain protection from everything until your next turn.
//! Oracle: At the beginning of your upkeep, you lose 1 life for each burden counter on The One Ring.
//! Oracle: {T}: Put a burden counter on The One Ring, then draw a card for each burden counter on The One Ring.
//! Set: LTR #246 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: d5806e68-1054-458e-866d-1f2470f682b2 | Oracle ID: 3aa83ed2-f48b-4ce6-a614-2c54ddf50538
// IMPLEMENTED — Indestructible, and nothing else: the three printed clauses
// below it are NOT SUPPORTED, each named with its reason at the foot of this
// file.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_ONE_RING,
    oracle_id = "3aa83ed2-f48b-4ce6-a614-2c54ddf50538",
    scryfall_id = "d5806e68-1054-458e-866d-1f2470f682b2",
    faces = &[face!(
        name = "The One Ring",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    keywords = KeywordSet::INDESTRUCTIBLE,
    coverage = Coverage::Partial(
        "the burden counter can be neither named (`counters::BURDEN` does not \
         exist) nor counted (no `Amount` reads a counter's count); `Condition` \
         cannot say \"if you cast it\"; and no `Modifier` grants a player \
         protection from everything",
    ),
);

// NOT SUPPORTED: "When The One Ring enters, if you cast it, you gain
// protection from everything until your next turn." — the trigger is
// `Trigger::ETB`, which the DSL has; both remaining halves are not. `Condition`
// carries the printed intervening `if` (CR 603.4) and has no clause for "you
// cast it", and the grant is player-scoped where `Modifier::ProtectionFrom` is
// object-scoped — the nearest variant, `Modifier::PlayerHexproof`, is a
// strictly smaller sentence that stops being targeted and not being dealt
// damage (CR 702.16). `Duration::UntilYourNextTurn` is the one piece of the
// clause that is sayable as it stands.
//
// NOT SUPPORTED: "At the beginning of your upkeep, you lose 1 life for each
// burden counter on The One Ring." — `Trigger::StepBegin { step:
// StepKind::Upkeep, whose: PlayerRel::You }` is the trigger, and `LoseLife`
// wants an `Amount` that nothing can compute: `Amount::CountOf` counts objects
// in a zone and no `Amount` counts counters. The nearest reader,
// `Condition::CountersOnSelf`, answers a yes/no threshold and hands back no
// number to lose life for.
//
// NOT SUPPORTED: "{T}: Put a burden counter on The One Ring, then draw a card
// for each burden counter on The One Ring." — `cost!(TapSelf)` and
// `Effect::AddCounter { amount: Amount::Fixed(1), .. }` (no target, so the
// counter would land on the source) would have built the first half; `kind` is
// the wall. Burden is a word the rules have never heard of, so it belongs in
// `baylee_cards_dsl::counters` as an id of its own and this pool has none. The
// second half draws for that same count, which nothing can read.
