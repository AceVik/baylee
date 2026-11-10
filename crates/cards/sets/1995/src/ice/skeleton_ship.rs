//! Skeleton Ship — {3}{U}{B} — Legendary Creature — Skeleton
//! Oracle: When you control no Islands, sacrifice Skeleton Ship.
//! Oracle: {T}: Put a -1/-1 counter on target creature.
//! Set: ME2 #197 — Masters Edition II | Scryfall ID: 060b8dd4-d632-4216-87ba-355ab8a62e39 | Oracle ID: 8c85887d-5935-46ec-a075-dc9f5133bb96
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static YOUR_ISLAND: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::ISLAND),
    Filter::ControlledByYou,
]);

static NO_ISLANDS: Condition = Condition::ControlCountAtMost(&YOUR_ISLAND, 0);

card!(
    index = index::SKELETON_SHIP,
    oracle_id = "8c85887d-5935-46ec-a075-dc9f5133bb96",
    scryfall_id = "060b8dd4-d632-4216-87ba-355ab8a62e39",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Skeleton Ship",
        mana_cost = mana!("{3}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::SKELETON],
        power = Some(0),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(Trigger::State(&NO_ISLANDS), &[Effect::SacrificeSelf]),
        activated!(
            cost!(TapSelf),
            &[Effect::AddCounter {
                kind: CounterKind::M1M1,
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
