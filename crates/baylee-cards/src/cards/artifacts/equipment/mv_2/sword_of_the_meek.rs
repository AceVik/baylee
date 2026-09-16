//! Sword of the Meek — {2} — Artifact — Equipment
//! Oracle: Equipped creature gets +1/+2.
//! Oracle: Equip {2}
//! Oracle: Whenever a 1/1 creature you control enters, you may return this card from your graveyard to the battlefield, then attach it to that creature.
//! Set: 2XM #299 — Double Masters | Scryfall ID: 5a0c2773-3205-4ac4-b31c-c54fb06fdd7c | Oracle ID: 215c287d-56a5-46da-b49e-8524b6d320a4
// PARTIAL — equipped creature gets +1/+2 and equip {2}; the graveyard return-and-attach trigger is not expressible.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SWORD_OF_THE_MEEK,
    oracle_id = "215c287d-56a5-46da-b49e-8524b6d320a4",
    scryfall_id = "5a0c2773-3205-4ac4-b31c-c54fb06fdd7c",
    faces = &[face!(
        name = "Sword of the Meek",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    coverage = Coverage::Partial(
        "the graveyard return-and-attach trigger for 1/1 creatures is not expressible"
    ),
    abilities = &[
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 2)),
        equip!("{2}"),
        // NOT SUPPORTED: "Whenever a 1/1 creature you control enters, you may
        // return this card from your graveyard to the battlefield, then attach
        // it to that creature." — AbilityDef::Triggered functions only on the
        // battlefield, Filter has no 1/1 power/toughness check, and Effect lacks
        // a variant to return self from the graveyard and attach to an entering
        // creature.
    ],
);
