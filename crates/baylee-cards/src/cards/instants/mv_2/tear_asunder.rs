//! Tear Asunder — {1}{G} — Instant
//! Oracle: Kicker {1}{B} (You may pay an additional {1}{B} as you cast this spell.)
//! Oracle: Exile target artifact or enchantment. If this spell was kicked, exile target nonland permanent instead.
//! Set: EOC #109 — Edge of Eternities Commander | Scryfall ID: e408c673-4a1f-45db-827a-75c501e1b3d6 | Oracle ID: 610af0f7-b5e3-43fb-9d02-7c59bd99034c
// PARTIAL — "Exile target artifact or enchantment" is built; the kicker and its
// "exile target nonland permanent instead" are left off, see NOT SUPPORTED.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEAR_ASUNDER,
    oracle_id = "610af0f7-b5e3-43fb-9d02-7c59bd99034c",
    scryfall_id = "e408c673-4a1f-45db-827a-75c501e1b3d6",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Tear Asunder",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "the kicked mode targets any nonland permanent where the unkicked mode targets an \
         artifact or enchantment, and AbilityDef::Spell carries one TargetReq for the whole \
         spell"
    ),
    abilities = &[spell!(
        &[Effect::exile(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT
        )))
    )],
);

// NOT SUPPORTED: "Kicker {1}{B}" and "If this spell was kicked, exile target
// nonland permanent instead" — a spell has one target requirement, so the
// kicked mode cannot widen its target set, and a kicker whose payment changed
// nothing would be an offer the engine cannot apply.
