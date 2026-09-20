//! City of Traitors — (no cost) — Land
//! Oracle: When you play another land, sacrifice this land.
//! Oracle: {T}: Add {C}{C}.
//! Set: TPR #237 — Tempest Remastered | Scryfall ID: 71624139-a255-48be-93ca-594a4beba487 | Oracle ID: f161111d-9747-47b3-bb10-3c8bded32e21
// PARTIAL — the sacrifice trigger and the {C}{C} mana ability, both built;
// the trigger is read as a land entering the battlefield because the DSL
// has no "plays a land" event.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CITY_OF_TRAITORS,
    oracle_id = "f161111d-9747-47b3-bb10-3c8bded32e21",
    scryfall_id = "71624139-a255-48be-93ca-594a4beba487",
    faces = &[face!(name = "City of Traitors", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the DSL has no trigger for a land being *played*: `Trigger::EntersBattlefield` is the nearest variant there is, and it also sacrifices this land for a land put onto the battlefield by an effect"
    ),
    abilities = &[
        // NOT SUPPORTED: "When you play another land" — no trigger in the
        // vocabulary reads a land being played, so this fires for every land
        // that enters the battlefield under your control, a fetchland's
        // included.
        triggered!(
            Trigger::EntersBattlefield(&f!(your another LAND)),
            &[Effect::SacrificeSelf]
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 2)]),
    ],
);
