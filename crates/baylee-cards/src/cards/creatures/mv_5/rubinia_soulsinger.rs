//! Rubinia Soulsinger — {2}{G}{W}{U} — Legendary Creature — Faerie
//! Oracle: You may choose not to untap Rubinia Soulsinger during your untap step.
//! Oracle: {T}: Gain control of target creature for as long as you control Rubinia Soulsinger and Rubinia Soulsinger remains tapped.
//! Set: CMA #191 — Commander Anthology | Scryfall ID: ce216785-aa81-44b4-9562-9b7eca743a0c | Oracle ID: bd3eeaba-964b-49ea-bb11-5875a78b8a4c
// PARTIAL — the untap-step choice is built; the control-change ability is
// off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RUBINIA_SOULSINGER,
    oracle_id = "bd3eeaba-964b-49ea-bb11-5875a78b8a4c",
    scryfall_id = "ce216785-aa81-44b4-9562-9b7eca743a0c",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Rubinia Soulsinger",
        mana_cost = mana!("{2}{G}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::FAERIE],
        power = Some(2),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "no Duration lasts while the source remains tapped: \
         WhileYouControlSource would leave the creature under your control \
         after Rubinia untaps, which the printed sentence forbids"
    ),
    // NOT SUPPORTED: "{T}: Gain control of target creature for as long as you
    // control Rubinia Soulsinger and Rubinia Soulsinger remains tapped." —
    // the change itself is Aladdin's shape, `Effect::continuous(&Filter::This,
    // Modifier::GainControl, …)`, but the only duration that ends when the
    // player stops controlling the source is `Duration::WhileYouControlSource`,
    // and none watches the source's tapped state (the missing
    // `Duration::WhileSourceTapped` Tawnos's Weaponry names); shipping the
    // nearest duration would be strictly stronger than the printed card.
    abilities = &[static_ability!(Filter::This, Modifier::MayChooseNotToUntap)],
);
