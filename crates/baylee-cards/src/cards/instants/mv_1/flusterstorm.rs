//! Flusterstorm — {U} — Instant
//! Oracle: Counter target instant or sorcery spell unless its controller pays {1}.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: IMA #55 — Iconic Masters | Scryfall ID: f900eeb7-7c45-44bc-ad3a-0bbe594ecf50 | Oracle ID: 86bf58f2-7f25-4e10-b797-25e0e8e67769

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static COUNTER_IT: Effect = Effect::CounterTargetSpell;

card!(
    index = index::FLUSTERSTORM,
    oracle_id = "86bf58f2-7f25-4e10-b797-25e0e8e67769",
    scryfall_id = "f900eeb7-7c45-44bc-ad3a-0bbe594ecf50",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Flusterstorm",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    keywords = KeywordSet::STORM,
    abilities = &[spell!(
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(1),
            effect: &COUNTER_IT,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(
            &Filter::INSTANT_OR_SORCERY
        )))
    )],
);
