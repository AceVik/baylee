//! Ydwen Efreet — {R}{R}{R} — Creature — Efreet
//! Oracle: Whenever this creature blocks, flip a coin. If you lose the flip, remove this creature from combat and it can't block this turn. Creatures it was blocking that had become blocked by only this creature this combat become unblocked.
//! Set: ME1 #112 — Masters Edition | Scryfall ID: 79a0dd6d-8904-467f-b63b-13dfc45232fb | Oracle ID: 15b9b0cc-47ef-4147-aba7-a7adae41921b
// IMPLEMENTED — "whenever this creature blocks" (Trigger::Blocks, once per
// declaration, CR 509.3a) flips a coin; a lost flip removes it from combat,
// unblocking what only it blocked, and it can't block this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::YDWEN_EFREET,
    oracle_id = "15b9b0cc-47ef-4147-aba7-a7adae41921b",
    scryfall_id = "79a0dd6d-8904-467f-b63b-13dfc45232fb",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Ydwen Efreet",
        mana_cost = mana!("{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::EFREET],
        power = Some(3),
        toughness = Some(6),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Blocks(&Filter::This),
        &[Effect::FlipCoin {
            won: &[],
            lost: &[
                Effect::RemoveTargetFromCombat { unblock: true },
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::CANT_BLOCK),
                    Duration::UntilEndOfTurn,
                ),
            ],
        }]
    ),],
);
