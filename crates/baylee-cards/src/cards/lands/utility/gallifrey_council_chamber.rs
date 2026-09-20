//! Gallifrey Council Chamber — (no cost) — Legendary Land
//! Oracle: When Gallifrey Council Chamber enters, surveil 1. (Look at the top card of your library. You may put that card into your graveyard.)
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Time Lord or Alien spell or activate an ability of a Time Lord or Alien.
//! Set: WHO #188 — Doctor Who | Scryfall ID: 83d990f5-a7b7-4482-95e6-b03a397192e2 | Oracle ID: 26e4b49e-77e7-41d9-94c5-924669a82591
// PARTIAL — the enter-surveil and both mana abilities are built; the spend
// restriction covers the spell half only (see the note beside the ability).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static TIME_LORD_OR_ALIEN: Filter = Filter::Or(&[
    Filter::HasSubtype(creature::TIME_LORD),
    Filter::HasSubtype(creature::ALIEN),
]);

card!(
    index = index::GALLIFREY_COUNCIL_CHAMBER,
    oracle_id = "26e4b49e-77e7-41d9-94c5-924669a82591",
    scryfall_id = "83d990f5-a7b7-4482-95e6-b03a397192e2",
    faces = &[face!(
        name = "Gallifrey Council Chamber",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "the spend restriction's 'or activate an ability of a Time Lord or Alien' half \
         (ManaRestriction reads spells only)",
    ),
    abilities = &[
        triggered!(Trigger::ETB, &[Effect::surveil(1)]),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "or activate an ability of a Time Lord or Alien" —
        // ManaRestriction's filter is read against the spell being cast, so
        // this mana reaches casting a Time Lord or Alien spell and no
        // activation of such a permanent's ability.
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&TIME_LORD_OR_ALIEN, SpendRider::None)
        ]),
    ],
);
