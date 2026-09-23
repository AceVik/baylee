//! Gods' Eye, Gate to the Reikai — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: When Gods' Eye is put into a graveyard from the battlefield, create a 1/1 colorless Spirit creature token.
//! Set: BOK #164 — Betrayers of Kamigawa | Scryfall ID: bdc33a21-d196-4c17-a296-87ff08e7ef69 | Oracle ID: a66008c9-1ede-4dcf-8d35-6c0ed2390996
// IMPLEMENTED — {T}: Add {C}, and the graveyard trigger that leaves a Spirit
// behind.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::GODS_EYE_GATE_TO_THE_REIKAI,
    oracle_id = "a66008c9-1ede-4dcf-8d35-6c0ed2390996",
    scryfall_id = "bdc33a21-d196-4c17-a296-87ff08e7ef69",
    faces = &[face!(
        name = "Gods' Eye, Gate to the Reikai",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    // `Trigger::Dies` is the battlefield → graveyard zone change and asks
    // nothing about being a creature, which is what this card needs: the
    // printed sentence is "put into a graveyard from the battlefield" and
    // the object is a land. `LeavesBattlefield` would be the wrong
    // sentence — it also fires on an exile and on a bounce.
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::CreateToken {
                token: &generated_tokens::SPIRIT_1_1
            }]
        ),
    ],
);
