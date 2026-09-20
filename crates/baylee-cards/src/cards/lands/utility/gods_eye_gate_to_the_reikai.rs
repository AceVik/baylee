//! Gods' Eye, Gate to the Reikai — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: When Gods' Eye is put into a graveyard from the battlefield, create a 1/1 colorless Spirit creature token.
//! Set: BOK #164 — Betrayers of Kamigawa | Scryfall ID: bdc33a21-d196-4c17-a296-87ff08e7ef69 | Oracle ID: a66008c9-1ede-4dcf-8d35-6c0ed2390996
// PARTIAL — {T}: Add {C} is built; the graveyard trigger is not, see the
// NOT SUPPORTED line at the foot of this file.

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
    coverage = Coverage::Partial(
        "the death trigger makes a 1/1 colorless Spirit, and no token in the \
         central registry is one — a card file may not define its own TokenDef",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "When Gods' Eye is put into a graveyard from the battlefield,
// create a 1/1 colorless Spirit creature token." `Effect::CreateToken` names a
// `&'static TokenDef` out of `crate::tokens`, which holds no Spirit token, and
// `tokens::no_card_file_defines_its_own_token` refuses a literal written in a
// card file (it would have no id and reach the table nameless). Once a Spirit
// token is registered in `crates/baylee-cards/src/tokens.rs` — and the ledger
// re-run — the clause is one `triggered!` on the leave-to-graveyard event with
// `&[Effect::CreateToken { token: … }]`, and `Trigger::Dies(&Filter::This)` is
// the trigger that says "battlefield → graveyard" rather than
// `LeavesBattlefield`, which would also fire on exile and on a bounce.
