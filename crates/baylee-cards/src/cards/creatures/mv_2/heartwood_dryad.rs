//! Heartwood Dryad — {1}{G} — Creature — Dryad
//! Oracle: This creature can block creatures with shadow as though it had shadow.
//! Set: TPR #173 — Tempest Remastered | Scryfall ID: edeb7cc6-68a5-4825-ad1f-6afd6b0c0b87 | Oracle ID: 976968c5-7ef4-4451-86ff-03a011bbcca7
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HEARTWOOD_DRYAD,
    oracle_id = "976968c5-7ef4-4451-86ff-03a011bbcca7",
    scryfall_id = "edeb7cc6-68a5-4825-ad1f-6afd6b0c0b87",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Has shadow during opponents' turns so it can block creatures with shadow while attacking normally on your turn; simultaneously blocking both shadow and non-shadow creatures without a CanBlockShadow modifier is not yet modeled."
    ),
    faces = &[face!(
        name = "Heartwood Dryad",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRYAD],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::AddKeyword(KeywordSet::SHADOW),
        condition = Some(Condition::OpponentsTurn),
    )],
);
