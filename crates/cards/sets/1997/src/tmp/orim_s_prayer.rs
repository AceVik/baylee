//! Orim's Prayer — {1}{W}{W} — Enchantment
//! Oracle: Whenever one or more creatures attack you, you gain 1 life for each attacking creature.
//! Set: TMP #32 — Tempest | Scryfall ID: 2dc45565-4b56-49ba-b115-be8e0de7d937 | Oracle ID: f387b1c6-25a6-4fff-90d3-fff027b247a5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static ATTACKING_YOU: Filter = Filter::And(&[Filter::CREATURE, Filter::AttackingYou]);

card!(
    index = index::ORIM_S_PRAYER,
    oracle_id = "f387b1c6-25a6-4fff-90d3-fff027b247a5",
    scryfall_id = "2dc45565-4b56-49ba-b115-be8e0de7d937",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Orim's Prayer",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::OneOrMoreAttack(&ATTACKING_YOU),
        &[Effect::GainLife {
            amount: Amount::CountOf {
                filter: &Filter::ATTACKING_CREATURE,
                zone: ZoneSel::Battlefield,
            },
        }],
    )],
);
