//! An-Havva Constable — {1}{G}{G} — Creature — Human
//! Oracle: An-Havva Constable's toughness is equal to 1 plus the number of green creatures on the battlefield.
//! Set: 5ED #277 — Fifth Edition | Scryfall ID: 5955eb2b-31a2-4e1e-bbbe-82eb53def698 | Oracle ID: 7131cdb6-15d0-407e-a7e2-b2a0deef9363
// IMPLEMENTED — characteristic-defining toughness equal to 1 plus green creatures on the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GREEN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
]);
static GREEN_CREATURES_ON_BATTLEFIELD: PtCount = PtCount::OnBattlefield(&GREEN_CREATURE);

card!(
    index = index::AN_HAVVA_CONSTABLE,
    oracle_id = "7131cdb6-15d0-407e-a7e2-b2a0deef9363",
    scryfall_id = "5955eb2b-31a2-4e1e-bbbe-82eb53def698",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "An-Havva Constable",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPTSeparate {
            power: PtCount::Fixed(2),
            toughness: PtCount::Plus {
                base: &GREEN_CREATURES_ON_BATTLEFIELD,
                offset: 1,
            },
        }
    )],
);
