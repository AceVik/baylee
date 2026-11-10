//! Mindwhip Sliver — {2}{B} — Creature — Sliver
//! Oracle: All Slivers have "{2}, Sacrifice this permanent: Target player discards a card at random. Activate only as a sorcery."
//! Set: TMP #145 — Tempest | Scryfall ID: fa966fbb-140d-4057-a4fc-998ebe07c307 | Oracle ID: e9e43839-7cc1-40ff-a8ec-a47316de6440
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SLIVER: Filter = Filter::HasSubtype(subtypes::creature::SLIVER);

card!(
    index = index::MINDWHIP_SLIVER,
    oracle_id = "e9e43839-7cc1-40ff-a8ec-a47316de6440",
    scryfall_id = "fa966fbb-140d-4057-a4fc-998ebe07c307",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mindwhip Sliver",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[static_ability!(
        SLIVER,
        Modifier::GrantActivatedDefinition(&activated!(
            cost!("{2}", SacrificeSelf),
            &[Effect::DiscardRandom {
                who: PlayerRel::Chosen,
                count: Amount::Fixed(1)
            }],
            target = Some(TargetSpec::AnyPlayer),
            timing = ActivationTiming::SorcerySpeed,
        )),
    )],
);
