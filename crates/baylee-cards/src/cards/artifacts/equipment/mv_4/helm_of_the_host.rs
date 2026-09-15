//! Helm of the Host — {4} — Legendary Artifact — Equipment
//! Oracle: At the beginning of combat on your turn, create a token that's a copy of equipped creature, except the token isn't legendary. That token gains haste.
//! Oracle: Equip {5}
//! Set: MSC #200 — Marvel Super Heroes Commander | Scryfall ID: 70ffc71f-328d-421d-926b-6f2e45ffb812 | Oracle ID: 83b43aba-bf9c-4da2-967d-9daa632e97d2
// IMPLEMENTED — equipment + combat-begin token copy of the equipped creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::artifact;

card!(
    index = 17621,
    oracle_id = "83b43aba-bf9c-4da2-967d-9daa632e97d2",
    scryfall_id = "70ffc71f-328d-421d-926b-6f2e45ffb812",
    faces = &[face!(
        name = "Helm of the Host",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[artifact::EQUIPMENT],
    )],
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: PlayerRel::You,
            },
            &[Effect::CreateTokenCopyOfEquipped {
                kicked_bonus: 0,
                mods: &[
                    CopyMod::RemoveSupertype(SupertypeSet::LEGENDARY),
                    CopyMod::AddKeyword(KeywordSet::HASTE),
                ],
            }]
        ),
        equip!("{5}"),
    ],
);
