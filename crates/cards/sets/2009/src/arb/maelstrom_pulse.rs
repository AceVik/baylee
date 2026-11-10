//! Maelstrom Pulse — {1}{B}{G} — Sorcery
//! Oracle: Destroy target nonland permanent and all other permanents with the same name as that permanent.
//! Set: INR #244 — Innistrad Remastered | Scryfall ID: 66f17263-b916-40f4-b175-fcfd5630103d | Oracle ID: 95ce305f-34bc-4d6d-b7ba-ffd4b2a25336
// IMPLEMENTED — the same-name sweep reads the target's name as the spell
// resolves, before the target itself is destroyed, so a copy is swept by the
// name it has on the battlefield.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MAELSTROM_PULSE,
    oracle_id = "95ce305f-34bc-4d6d-b7ba-ffd4b2a25336",
    scryfall_id = "66f17263-b916-40f4-b175-fcfd5630103d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Maelstrom Pulse",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            // "…and all other permanents with the same name as that
            // permanent" — first, while the target is still on the
            // battlefield to be asked its name.
            Effect::DestroyOthersNamedLike {
                target: TargetSpec::Object(&Filter::NONLAND),
            },
            Effect::destroy(TargetSpec::Object(&Filter::NONLAND)),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::NONLAND)))
    )],
);
