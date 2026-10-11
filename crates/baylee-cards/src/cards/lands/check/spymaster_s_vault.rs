//! Spymaster's Vault — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Swamp.
//! Oracle: {T}: Add {B}.
//! Oracle: {B}, {T}: Target creature you control connives X, where X is the number of creatures that died this turn. (Draw X cards, then discard X cards. Put a +1/+1 counter on that creature for each nonland card discarded this way.)
//! Set: MH3 #230 — Modern Horizons 3 | Scryfall ID: 3d5fbb30-abfc-4e79-8ce5-bbb04a241c9f | Oracle ID: 69ddca4b-5cc0-45f3-b2e6-a047c8d601be

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SWAMP_YOU_CONTROL: Filter =
    Filter::And(&[Filter::HasSubtype(land::SWAMP), Filter::ControlledByYou]);

card!(
    index = index::SPYMASTER_S_VAULT,
    oracle_id = "69ddca4b-5cc0-45f3-b2e6-a047c8d601be",
    scryfall_id = "3d5fbb30-abfc-4e79-8ce5-bbb04a241c9f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spymaster's Vault",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&SWAMP_YOU_CONTROL)],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            cost!("{B}", TapSelf),
            &[Effect::Connive {
                amount: Amount::CreaturesDiedThisTurn
            }],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE))
        ),
    ],
);
