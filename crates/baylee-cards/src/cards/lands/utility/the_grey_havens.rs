//! The Grey Havens — (no cost) — Legendary Land
//! Oracle: When The Grey Havens enters, scry 1.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color among legendary creature cards in your graveyard.
//! Set: LTR #255 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: dd698f10-b0fc-42fc-84ec-f5a0d96bfa1d | Oracle ID: a1a9695e-073b-4a65-b3ec-2cfddc23202a
// PARTIAL — the enter-trigger scry 1 and the {C} mana ability are implemented;
// the graveyard mana line is not expressible.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THE_GREY_HAVENS,
    oracle_id = "a1a9695e-073b-4a65-b3ec-2cfddc23202a",
    scryfall_id = "dd698f10-b0fc-42fc-84ec-f5a0d96bfa1d",
    faces = &[face!(
        name = "The Grey Havens",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "{T}: Add one mana of any color among legendary creature cards in your graveyard — \
         ManaSource has no variant that reads a colour off a card in a zone"
    ),
    abilities = &[
        triggered!(Trigger::ETB, &[Effect::scry(1)]),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{T}: Add one mana of any color among legendary
        // creature cards in your graveyard." — ManaSource's six variants are
        // Fixed, Choice, CommanderIdentity, LandColor, Chosen and ChosenOr;
        // the nearest, LandColor, reads lands on the battlefield, and no
        // variant reads a colour off a card in a graveyard.
    ],
);
