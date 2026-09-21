//! Three Tree City — (no cost) — Legendary Land
//! Oracle: As Three Tree City enters, choose a creature type.
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Choose a color. Add an amount of mana of that color equal to the number of creatures you control of the chosen type.
//! Set: BLB #260 — Bloomburrow | Scryfall ID: 56f88a48-cced-4a9d-8c19-e4f105f0d8a2 | Oracle ID: da3b17a2-e1e1-44e9-b9b1-ae54a92037db
// IMPLEMENTED — ChooseSubtype on entry; {T} for {C}; {2},{T} for one chosen
// color, counted over your creatures of the chosen type.

use baylee_cards_dsl::prelude::*;

static CHOSEN_TYPE_CREATURES: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByYou,
    Filter::MatchesChosenTypeOfSource,
]);

card!(
    index = index::THREE_TREE_CITY,
    oracle_id = "da3b17a2-e1e1-44e9-b9b1-ae54a92037db",
    scryfall_id = "56f88a48-cced-4a9d-8c19-e4f105f0d8a2",
    faces = &[face!(
        name = "Three Tree City",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::ChooseSubtype],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!("{2}", TapSelf),
            &[Effect::mana_choice_dynamic(
                ALL_MANA_COLORS,
                Amount::CountOf {
                    filter: &CHOSEN_TYPE_CREATURES,
                    zone: ZoneSel::Battlefield,
                },
            )],
        ),
    ],
);
