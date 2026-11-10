//! Barbed Foliage — {2}{G}{G} — Enchantment
//! Oracle: Whenever a creature attacks you, it loses flanking until end of turn.
//! Oracle: Whenever a creature without flying attacks you, this enchantment deals 1 damage to it.
//! Set: MIR #207 — Mirage | Scryfall ID: 1220fdd1-e4b0-4175-9b8a-178f6a84b8e6 | Oracle ID: f9275c99-0616-44b3-a19e-98c683e87ad3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static ATTACKING_YOU: Filter = Filter::And(&[Filter::CREATURE, Filter::AttackingYou]);
static WITHOUT_FLYING_ATTACKING_YOU: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::AttackingYou,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

card!(
    index = index::BARBED_FOLIAGE,
    oracle_id = "f9275c99-0616-44b3-a19e-98c683e87ad3",
    scryfall_id = "1220fdd1-e4b0-4175-9b8a-178f6a84b8e6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Barbed Foliage",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::Attacks(&ATTACKING_YOU),
            &[Effect::continuous(
                &Filter::Reference,
                Modifier::RemoveKeyword(KeywordSet::FLANKING),
                Duration::UntilEndOfTurn,
            )],
        ),
        triggered!(
            Trigger::Attacks(&WITHOUT_FLYING_ATTACKING_YOU),
            &[Effect::DealDamageEach {
                amount: Amount::Fixed(1),
                filter: &Filter::Reference,
            }],
        ),
    ],
);
