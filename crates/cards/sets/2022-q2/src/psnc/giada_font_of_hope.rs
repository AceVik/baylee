//! Giada, Font of Hope — {1}{W} — Legendary Creature — Angel
//! Oracle: Flying, vigilance
//! Oracle: Each other Angel you control enters with an additional +1/+1 counter on it for each Angel you already control.
//! Oracle: {T}: Add {W}. Spend this mana only to cast an Angel spell.
//! Set: FDN #141 — Foundations | Scryfall ID: 8ae6fc26-cfad-4da8-98d9-49c27c24d293 | Oracle ID: 48e6d3d8-2f27-4017-acdd-40bce8cdbc02
// IMPLEMENTED — entry counters count Angels already controlled; white mana
// carries an Angel-spell restriction.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OTHER_ANGEL: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::creature::ANGEL),
    Filter::Another,
    Filter::ControlledByYou,
]);
static ANGEL: Filter = Filter::HasSubtype(subtypes::creature::ANGEL);

card!(
    index = index::GIADA_FONT_OF_HOPE,
    oracle_id = "48e6d3d8-2f27-4017-acdd-40bce8cdbc02",
    scryfall_id = "8ae6fc26-cfad-4da8-98d9-49c27c24d293",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Giada, Font of Hope",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        static_ability!(
            OTHER_ANGEL,
            Modifier::AdditionalCounterOnEntryByCount {
                kind: CounterKind::P1P1,
                count: PtCount::YouControl(&ANGEL),
            }
        ),
        mana_ability!(&[Effect::mana(ManaColor::White, 1).restricted(&ANGEL, SpendRider::None)]),
    ],
);
