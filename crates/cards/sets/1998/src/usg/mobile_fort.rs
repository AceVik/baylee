//! Mobile Fort — {4} — Artifact Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {3}: This creature gets +3/-1 until end of turn and can attack this turn as though it didn't have defender. Activate only once each turn.
//! Set: USG #303 — Urza's Saga | Scryfall ID: 5f642246-5b2a-46a6-9236-524a297d7608 | Oracle ID: 0547e426-e23e-4c60-ba6b-cfd4b5ff8485
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOBILE_FORT,
    oracle_id = "0547e426-e23e-4c60-ba6b-cfd4b5ff8485",
    scryfall_id = "5f642246-5b2a-46a6-9236-524a297d7608",
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mobile Fort",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(6),
    ),],
    abilities = &[activated!(
        cost!("{3}"),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(3, -1),
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AttacksDespiteDefender,
                Duration::UntilEndOfTurn,
            ),
        ],
        limit = ActivationLimit::PerTurn(1),
    )],
);
