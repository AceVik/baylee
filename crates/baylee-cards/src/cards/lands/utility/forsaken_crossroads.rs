//! Forsaken Crossroads — (no cost) — Land
//! Oracle: Forsaken Crossroads enters the battlefield tapped.
//! Oracle: As Forsaken Crossroads enters the battlefield, choose a color.
//! Oracle: When Forsaken Crossroads enters the battlefield, scry 1. If you weren't the starting player, you may untap Forsaken Crossroads instead.
//! Oracle: {T}: Add one mana of the chosen color.
//! Set: MB2 #264 — Mystery Booster 2 | Scryfall ID: 56491238-5228-4bdd-994c-0849a59ebc2c | Oracle ID: c70598e1-30c6-4f92-a265-34a7a73bc2b8
// PARTIAL — enters tapped with a colour chosen as it arrives and taps for
// that colour; the enter trigger is the scry alone, for the reason below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORSAKEN_CROSSROADS,
    oracle_id = "c70598e1-30c6-4f92-a265-34a7a73bc2b8",
    scryfall_id = "56491238-5228-4bdd-994c-0849a59ebc2c",
    faces = &[face!(
        name = "Forsaken Crossroads",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped, EnterModifier::ChooseColor],
    ),],
    coverage = Coverage::Partial(
        "\"If you weren't the starting player, you may untap Forsaken Crossroads \
         instead\" — no Condition names the starting player, and the \"instead\" \
         half of the trigger has no vocabulary"
    ),
    abilities = &[
        // NOT SUPPORTED: "If you weren't the starting player, you may untap
        // Forsaken Crossroads instead." The scry stands mandatory on every
        // board rather than optional on one.
        triggered!(Trigger::ETB, &[Effect::scry(1)]),
        mana_ability!(&[Effect::mana_chosen()]),
    ],
);
