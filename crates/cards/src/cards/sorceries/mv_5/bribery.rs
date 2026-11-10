//! Bribery — {3}{U}{U} — Sorcery
//! Oracle: Search target opponent's library for a creature card and put that card onto the battlefield under your control. Then that player shuffles.
//! Set: CMM #77 — Commander Masters | Scryfall ID: 49fd737a-d7da-421b-a741-d6d0d213299f | Oracle ID: 6d194882-ca37-49bb-ac9f-a751c53850a8
// IMPLEMENTED — you search the targeted opponent's library; the find enters
// under your control and that library is the one shuffled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRIBERY,
    oracle_id = "6d194882-ca37-49bb-ac9f-a751c53850a8",
    scryfall_id = "49fd737a-d7da-421b-a741-d6d0d213299f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Bribery",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::SearchLibraryOf {
            library: PlayerRel::Chosen,
            owner_searches: false,
            filter: &Filter::CREATURE,
            mana_value: None,
            finds: &[Find::BATTLEFIELD],
            optional: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    )],
);
