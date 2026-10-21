//! Daughter of Autumn — {2}{G}{G} — Legendary Creature — Avatar
//! Oracle: {W}: The next 1 damage that would be dealt to target white creature this turn is dealt to Daughter of Autumn instead.
//! Set: HML #85 — Homelands | Scryfall ID: 972e9c59-f340-414c-b55b-39d46dd97e8e | Oracle ID: 45f3b9f2-3fce-4f91-bcbd-de069e5f9e4c
// PARTIAL — RedirectNextDamage only redirects to a PlayerRel, not to this creature; prevents the next 1 damage that would be dealt to target white creature this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

card!(
    index = index::DAUGHTER_OF_AUTUMN,
    oracle_id = "45f3b9f2-3fce-4f91-bcbd-de069e5f9e4c",
    scryfall_id = "972e9c59-f340-414c-b55b-39d46dd97e8e",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "RedirectNextDamage only redirects to a PlayerRel, not to this creature; prevents the next 1 damage that would be dealt to target white creature this turn"
    ),
    faces = &[face!(
        name = "Daughter of Autumn",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!("{W}"),
        &[Effect::PreventNextDamage {
            target: TargetSpec::Object(&WHITE_CREATURE),
            amount: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&WHITE_CREATURE)))
    )],
);
