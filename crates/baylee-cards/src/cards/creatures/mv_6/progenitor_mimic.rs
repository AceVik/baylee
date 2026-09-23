//! Progenitor Mimic — {4}{G}{U} — Creature — Shapeshifter
//! Oracle: You may have this creature enter as a copy of any creature on the battlefield, except it has "At the beginning of your upkeep, if this creature isn't a token, create a token that's a copy of this creature."
//! Set: 2XM #212 — Double Masters | Scryfall ID: acba72e1-3f7f-4e5c-af3f-dfe37b5d61f9 | Oracle ID: 88929ea9-900f-4dbb-b16c-cf3bad4e410c
// IMPLEMENTED — clone + upkeep token-copy-of-self.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::PROGENITOR_MIMIC,
    oracle_id = "88929ea9-900f-4dbb-b16c-cf3bad4e410c",
    scryfall_id = "acba72e1-3f7f-4e5c-af3f-dfe37b5d61f9",
    faces = &[face!(
        name = "Progenitor Mimic",
        mana_cost = mana!("{4}{G}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(0),
    )],
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::Green]),
    coverage = Coverage::Implemented,
    abilities = &[
        // The upkeep trigger exists only inside the quotation marks: the card
        // prints no ability of its own beside the copy clause, and one
        // written there would be overwritten by the copy (CR 707.2). The
        // clause's "if this creature isn't a token" (CR 603.4) needs no
        // condition here, because the grant is on this permanent alone and a
        // token copy of it does not inherit it — the limit
        // `CopyMod::Grant` names, landing on the side the printed condition
        // lands on.
        AbilityDef::CopyOnEnter {
            target: TargetSpec::Object(&Filter::CREATURE),
            mods: &[CopyMod::Grant(&Modifier::GrantTriggered {
                trigger: Trigger::StepBegin {
                    step: StepKind::Upkeep,
                    whose: PlayerRel::You,
                },
                effects: &[Effect::CreateTokenCopyOf {
                    target: None,
                    kicked_bonus: 0,
                }],
                target: None,
            })],
        },
    ],
);
