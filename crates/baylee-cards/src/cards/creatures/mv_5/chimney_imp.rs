//! Chimney Imp — {4}{B} — Creature — Imp
//! Oracle: Flying
//! Oracle: When this creature dies, target opponent puts a card from their hand on top of their library.
//! Set: MRD #59 — Mirrodin | Scryfall ID: 0a426922-5e96-48f3-b696-f5dc99258943 | Oracle ID: 3901bf30-b7c1-4977-a7b1-fcdafcc266cd
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHIMNEY_IMP,
    oracle_id = "3901bf30-b7c1-4977-a7b1-fcdafcc266cd",
    scryfall_id = "0a426922-5e96-48f3-b696-f5dc99258943",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING,
    faces = &[face!(
        name = "Chimney Imp",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::IMP],
        power = Some(1),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::PutFromHandOnTopFor {
            player: PlayerRel::Chosen,
            count: 1,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
