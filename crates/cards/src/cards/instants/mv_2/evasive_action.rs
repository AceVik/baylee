//! Evasive Action — {1}{U} — Instant
//! Oracle: Domain — Counter target spell unless its controller pays {1} for each basic land type among lands you control.
//! Set: DDE #50 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: d8fad630-bd1c-42df-86b5-cc00da28abfd | Oracle ID: 4543a99d-eefa-470d-976d-11250524ae28
// IMPLEMENTED — the tax is the domain count (CR 305.6's five basic land
// types among lands *you* control, not the spell's controller), and the
// counter runs when it goes unpaid.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EVASIVE_ACTION,
    oracle_id = "4543a99d-eefa-470d-976d-11250524ae28",
    scryfall_id = "d8fad630-bd1c-42df-86b5-cc00da28abfd",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Evasive Action",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::BasicLandTypesAmong(&Filter::YOUR_LAND),
            effect: &Effect::CounterTargetSpell,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )],
);
