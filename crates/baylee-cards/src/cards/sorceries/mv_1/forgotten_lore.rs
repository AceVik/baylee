//! Forgotten Lore — {G} — Sorcery
//! Oracle: Target opponent chooses a card in your graveyard. You may pay {G}. If you do, repeat this process except that opponent can't choose a card already chosen for Forgotten Lore. Then put the last chosen card into your hand.
//! Set: ME2 #164 — Masters Edition II | Scryfall ID: 4b45aad3-bc82-46f3-8032-8bfcbd6a007a | Oracle ID: ca9f538d-4191-448a-870d-f645b7c37c1b
use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORGOTTEN_LORE,
    oracle_id = "ca9f538d-4191-448a-870d-f645b7c37c1b",
    scryfall_id = "4b45aad3-bc82-46f3-8032-8bfcbd6a007a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Forgotten Lore",
        mana_cost = mana!("{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::RechooseGraveyardCard {
            owner: PlayerRel::You,
            chooser: PlayerRel::Chosen,
            cost: mana!("{G}"),
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
