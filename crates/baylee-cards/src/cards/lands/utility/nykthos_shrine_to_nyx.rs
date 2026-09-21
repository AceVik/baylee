//! Nykthos, Shrine to Nyx — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Choose a color. Add an amount of mana of that color equal to your devotion to that color. (Your devotion to a color is the number of mana symbols of that color in the mana costs of permanents you control.)
//! Set: THS #223 — Theros | Scryfall ID: 834b27a0-dfd7-4f96-8cde-cacac4b24acc | Oracle ID: 84dc18f0-8225-4b40-a165-b10321e41769
// PARTIAL — {T}: Add {C} is built; the devotion ability is not sayable.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NYKTHOS_SHRINE_TO_NYX,
    oracle_id = "84dc18f0-8225-4b40-a165-b10321e41769",
    scryfall_id = "834b27a0-dfd7-4f96-8cde-cacac4b24acc",
    faces = &[face!(
        name = "Nykthos, Shrine to Nyx",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "the {2}, {T} ability picks its colour as it is activated and then counts mana symbols of that colour: ManaSource::Chosen reads the colour chosen as the permanent entered, and no Amount counts mana symbols",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{2}, {T}: Choose a color. Add an amount of mana of
        // that color equal to your devotion to that color." — the ability
        // needs a colour chosen as it is *activated* (EnterModifier's
        // ChooseColor is answered as the permanent enters, not per
        // activation) and an amount that counts the mana symbols of that
        // colour among the mana costs of permanents you control. Neither a
        // ManaSource nor an Amount carries it, and a plain CountOf would
        // count permanents rather than symbols (a {W}{W} permanent is
        // devotion 2 to white and one object).
    ],
);
