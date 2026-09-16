//! Boggart Trawler // Boggart Bog — {2}{B} — Creature — Goblin // Land
//! Oracle: When this creature enters, exile target player's graveyard.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: MH3 #243 — Modern Horizons 3 | Scryfall ID: d0d484a6-5610-4f1d-95ec-eda273c255e4 | Oracle ID: 727f3201-1cfc-4ab2-9dfe-be4f7251f42f
//! Face: Boggart Trawler — {2}{B} — Creature — Goblin
//! Face: Boggart Bog —  — Land
// IMPLEMENTED — the Goblin front exiles a chosen player's graveyard on its own
// ETB; the modal back is reached by the face-choice land play (CR 712.12),
// asks for 3 life as it enters or comes down tapped, and taps for {B}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BOG_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

card!(
    index = index::BOGGART_TRAWLER,
    oracle_id = "727f3201-1cfc-4ab2-9dfe-be4f7251f42f",
    scryfall_id = "d0d484a6-5610-4f1d-95ec-eda273c255e4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Boggart Trawler",
            mana_cost = mana!("{2}{B}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::GOBLIN],
            power = Some(3),
            toughness = Some(1),
        ),
        face!(
            name = "Boggart Bog",
            types = TypeSet::LAND,
            abilities = BOG_MANA,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::ExileGraveyard {
            player: PlayerRel::Chosen,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
    )],
);

// Engine-level test belongs in baylee-engine (mdfc_tests): the creature front
// asks for a player and empties that graveyard on entry, while the back face
// is offered as a second `CastModeKind::PlayLandFace` on a land play, puts up
// the pay-3-life-or-enter-tapped question, and taps for {B}.
