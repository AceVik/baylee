//! Dracoplasm — {U}{R} — Creature — Shapeshifter
//! Oracle: Flying
//! Oracle: As this creature enters, sacrifice any number of creatures. This creature's power becomes the total power of those creatures and its toughness becomes their total toughness.
//! Oracle: {R}: This creature gets +1/+0 until end of turn.
//! Set: TPR #208 — Tempest Remastered | Scryfall ID: 5dc9a21b-0935-4526-a0b0-9a85d0613e44 | Oracle ID: 3155c833-8170-4166-8f47-c3c1fad1575a
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRACOPLASM,
    oracle_id = "3155c833-8170-4166-8f47-c3c1fad1575a",
    scryfall_id = "5dc9a21b-0935-4526-a0b0-9a85d0613e44",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "Has flying, sacrifices any number of creatures you control as it enters to set its power and toughness equal to the number of creatures sacrificed (rather than their total power and total toughness), and has '{R}: This creature gets +1/+0 until end of turn.'"
    ),
    faces = &[face!(
        name = "Dracoplasm",
        mana_cost = mana!("{U}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::SacrificeForPowerToughness(
            &Filter::YOUR_CREATURE
        )],
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CharacteristicPT {
                count: PtCount::EntrySacrifices,
                toughness_plus: 0,
            }
        ),
        activated!(
            cost!("{R}"),
            &[Effect::ModifySource {
                modifier: Modifier::ModifyPT(1, 0),
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);
