//! Teferi's Isle — (no cost) — Legendary Land
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Oracle: Teferi's Isle enters tapped.
//! Oracle: {T}: Add {U}{U}.
//! Set: MIR #330 — Mirage | Scryfall ID: b6ed7ca8-fd91-46e3-9149-a3de23c7078e | Oracle ID: ce55657d-d82f-4528-a83e-5cad7de111fd
// IMPLEMENTED — enters tapped; {T}: Add {U}{U}. Phasing is a Partial.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEFERI_S_ISLE,
    oracle_id = "ce55657d-d82f-4528-a83e-5cad7de111fd",
    scryfall_id = "b6ed7ca8-fd91-46e3-9149-a3de23c7078e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Teferi's Isle",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "phasing: no engine keyword bit, and no recurring untap-step phase-out/phase-in ability"
    ),
    // NOT SUPPORTED: Phasing — "This phases in or out before you untap during
    // each of your untap steps. While it's phased out, it's treated as though
    // it doesn't exist." No keyword bit is read for phasing, `Trigger::StepBegin`
    // has no untap step, and `Effect::PhaseOut` is a one-shot effect.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 2)])],
);
