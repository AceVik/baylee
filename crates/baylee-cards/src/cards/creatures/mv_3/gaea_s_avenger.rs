//! Gaea's Avenger — {1}{G}{G} — Creature — Treefolk
//! Oracle: Gaea's Avenger's power and toughness are each equal to 1 plus the number of artifacts your opponents control.
//! Set: ME4 #155 — Masters Edition IV | Scryfall ID: 5df83b52-02df-4dfb-87b5-e2eab3d6c65b | Oracle ID: a4b60080-3f80-4aeb-8bd9-7953e77aaf75
// PARTIAL — the characteristic-defining P/T is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GAEA_S_AVENGER,
    oracle_id = "a4b60080-3f80-4aeb-8bd9-7953e77aaf75",
    scryfall_id = "5df83b52-02df-4dfb-87b5-e2eab3d6c65b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gaea's Avenger",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "the printed P/T is characteristic-defining and is 1 plus the number of \
         artifacts opponents control; CharacteristicPT sets power to the count \
         and adds only to toughness, and PtCount has no offset",
    ),
    // NOT SUPPORTED: "Gaea's Avenger's power and toughness are each equal to 1
    // plus the number of artifacts your opponents control." — the count itself
    // is sayable (PtCount::OnBattlefield over artifacts an opponent controls),
    // but neither value can be that count plus 1: Modifier::CharacteristicPT
    // sets power to the count and only its toughness takes a toughness_plus,
    // and PtCount has no offset. Modifier::ModifyPTPerCount cannot stand in
    // either: it counts permanents the effect's own controller controls, so it
    // can never count an opponent's artifacts. So the characteristic-defining
    // ability comes off the card rather than shipping a 1/1 whose number is
    // wrong in every zone but the battlefield.
);
