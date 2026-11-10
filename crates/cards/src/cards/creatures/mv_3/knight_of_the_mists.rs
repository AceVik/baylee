//! Knight of the Mists — {2}{U} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: When this creature enters, you may pay {U}. If you don't, destroy target Knight and it can't be regenerated.
//! Set: VIS #36 — Visions | Scryfall ID: 37924cbc-fb9d-4906-9ad2-9b6d4ccfff0f | Oracle ID: e9893534-3d05-42c2-93ff-f28ffae89aa3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static KNIGHT_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::KNIGHT),
]);

card!(
    index = index::KNIGHT_OF_THE_MISTS,
    oracle_id = "e9893534-3d05-42c2-93ff-f28ffae89aa3",
    scryfall_id = "37924cbc-fb9d-4906-9ad2-9b6d4ccfff0f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Knight of the Mists",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::PlayerMayPayManaOr {
            player: PlayerRel::You,
            cost: mana!("{U}"),
            effect: &Effect::Destroy {
                target: TargetSpec::Object(&KNIGHT_CREATURE),
                no_regen: true,
            },
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&KNIGHT_CREATURE))),
    ),],
);
