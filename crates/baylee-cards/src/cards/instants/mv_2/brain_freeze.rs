//! Brain Freeze — {1}{U} — Instant
//! Oracle: Target player mills three cards.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: VMA #57 — Vintage Masters | Scryfall ID: 3a2d7cf9-dddb-4de3-b4f2-c52e3ec8fb4b | Oracle ID: 464c0150-3dbc-403b-9ada-fef25ab1f29d
// PARTIAL — a two-mana blue instant that has a player it targets mill three
// cards; the printed storm never copies it, so the spell resolves once.
// NOT SUPPORTED: `Storm` — "copy it for each spell cast before it this turn".
// Three separate things would have to exist and none does. Storm is not on
// `keyword_tests::ENFORCED` and the `keywords!` table in the DSL has no
// `STORM` bit, so it is no `KeywordSet` bit. No `Amount` reads the per-turn
// spell count, though the engine already keeps one (`per_turn.spells_cast`):
// the closest is `Amount::XPlusCommanderCasts`, a per-*game* commander count,
// and `Amount::CountOf` has no `ZoneSel` for the stack. And no `Effect` copies
// the *source* spell — `Effect::CopyTargetSpell` copies the first target, once,
// and a spell has no way to target itself. The card plays exactly as though
// the line were not printed: a {1}{U} instant that mills three, once.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRAIN_FREEZE,
    oracle_id = "464c0150-3dbc-403b-9ada-fef25ab1f29d",
    scryfall_id = "3a2d7cf9-dddb-4de3-b4f2-c52e3ec8fb4b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Brain Freeze",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial("storm is not written: the spell resolves once"),
    abilities = &[spell!(
        &[Effect::Mill {
            amount: Amount::Fixed(3),
            target: PlayerRel::Chosen,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);

// Behaviour belongs in `baylee-engine`'s `engine::card_tests`, and this card
// brings no new shape to it. Both halves are already played there:
// `a_land_that_mills_target_player_can_be_activated` and
// `halimar_excavator_mills_the_player_it_targeted` run `Effect::Mill` with
// `PlayerRel::Chosen` off a chosen player, and
// `s7_tests::a_printed_x_is_asked_for_and_is_the_number_that_resolves` is the
// spell half: it casts Commander's Insight, answers the `Pending::ChoosePlayer`
// a `spell!` with `TargetSpec::AnyPlayer` asks, and resolves for that player.
