//! Carrionette — {1}{B} — Creature — Skeleton
//! Oracle: {2}{B}{B}: Exile this card and target creature unless that creature's controller pays {2}. Activate only if this card is in your graveyard.
//! Set: TMP #111 — Tempest | Scryfall ID: 884e19fb-67a4-42d8-b163-720a99cb8506 | Oracle ID: 24315f60-731a-40bb-85ba-eedba1b011ff
// IMPLEMENTED — Activated from your graveyard for {2}{B}{B} to exile this card and target creature unless that creature's controller pays {2}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CARRIONETTE,
    oracle_id = "24315f60-731a-40bb-85ba-eedba1b011ff",
    scryfall_id = "884e19fb-67a4-42d8-b163-720a99cb8506",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Carrionette",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SKELETON],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{2}{B}{B}"),
        &[Effect::PlayerMayPayThenElse {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(2),
            paid: &[],
            unpaid: &[
                Effect::exile(TargetSpec::ThisObject),
                Effect::exile(TargetSpec::Object(&Filter::CREATURE)),
            ],
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        zone = ActivationZone::Graveyard,
    )],
);
