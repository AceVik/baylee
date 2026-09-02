//! Primaris Eliminator — {4}{B} — Creature — Astartes Warrior
//! Oracle: When this creature enters, choose one —
//! Oracle: • Executioner Round — Destroy target creature.
//! Oracle: • Hyperfrag Round — Creatures target player controls get -2/-2 until end of turn.
//! Set: 40K #50 — Warhammer 40,000 Commander | Scryfall ID: db7ab081-d6cd-4323-98bf-536e4df95115 | Oracle ID: 7d679591-f8ea-4c4c-ab98-7b9e3438cf57
// IMPLEMENTED — modal ETB with both rounds (Hyperfrag's -2/-2 uses X = 2
// chosen automatically as the only value; X-driven pump via NegX).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static DESTROY_EFFECTS: &[Effect] = &[Effect::Destroy {
    target: TargetSpec::Object(&Filter::CREATURE),
}];
static DEBUFF_EFFECTS: &[Effect] = &[Effect::PumpFilter {
    filter: &Filter::CREATURE,
    power: Amount::NegXFixed(2),
    toughness: Amount::NegXFixed(2),
    keywords: KeywordSet::EMPTY,
    duration: Duration::UntilEndOfTurn,
}];

card! {
    index: 118,
    oracle_id: "7d679591-f8ea-4c4c-ab98-7b9e3438cf57",
    scryfall_id: "db7ab081-d6cd-4323-98bf-536e4df95115",
    faces: &[face! {
        name: "Primaris Eliminator",
        mana_cost: baylee_core::mana!("{4}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::ASTARTES, creature::WARRIOR],
        power: Some(3),
        toughness: Some(3),
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalTriggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        modes: &[
            mode!(DESTROY_EFFECTS, target: Some(TargetSpec::Object(&Filter::CREATURE))),
            mode!(DEBUFF_EFFECTS),
        ],
        once_per_turn: false,
    }],
}
