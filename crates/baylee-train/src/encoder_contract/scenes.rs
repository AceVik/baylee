//! Tables the engine has brought to one of PR #410's questions, for the
//! encoder tests here and the policy's (`netplay4::policy`).

use baylee_core::ids::ObjectId;
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};

use super::{P0, P1, Seat, Table, at_rest_in_main};

/// Doomskar in hand over five Plains.
fn doomskar_table() -> Table {
    Table::new(
        41,
        "Plains",
        [
            Seat {
                hand: &["Doomskar"],
                battlefield: &["Plains"; 5],
            },
            Seat {
                hand: &[],
                battlefield: &[],
            },
        ],
    )
}

/// Its owner's first main phase with the Plains tapped: foretelling
/// Doomskar ({2}, paid from the pool as suspend is) is on offer.
pub(crate) fn foretell_on_offer() -> (Table, ObjectId) {
    let mut table = doomskar_table();
    table.reach_main(P0);
    let card = table.in_hand(P0, "Doomskar");
    table.tap_lands(P0);
    (table, card)
}

/// Doomskar foretold, and its owner's next turn (CR 702.143a: a later one)
/// with the Plains tapped: castable for its foretell cost, {1}{W}{W}.
pub(crate) fn foretold_and_castable() -> (Table, ObjectId) {
    let (mut table, card) = foretell_on_offer();
    table
        .session
        .act(P0, PlayerAction::Foretell { card })
        .expect("foretell");
    table.answer_until(|session| session.state().turn.number > 2 && at_rest_in_main(session, P0));
    table.tap_lands(P0);
    (table, card)
}

/// Serra's Emissary entering: "choose a card type" is asked of its owner.
pub(crate) fn card_type_asked() -> Table {
    let mut table = Table::new(
        43,
        "Plains",
        [
            Seat {
                hand: &["Serra's Emissary"],
                battlefield: &["Plains"; 7],
            },
            Seat {
                hand: &[],
                battlefield: &[],
            },
        ],
    );
    table.reach_main(P0);
    table.tap_lands(P0);
    let emissary = table.in_hand(P0, "Serra's Emissary");
    table
        .session
        .act(P0, PlayerAction::CastSpell { card: emissary })
        .expect("cast Serra's Emissary");
    table.answer_until(|session| matches!(session.pending(), Pending::ChooseCardType { .. }));
    table
}

/// Cabal Therapy resolving at the other player, who holds Grizzly Bears and
/// a Forest: "choose a nonland card name" is asked of its caster.
pub(crate) fn nonland_name_asked() -> Table {
    let mut table = Table::new(
        44,
        "Swamp",
        [
            Seat {
                hand: &["Cabal Therapy"],
                battlefield: &["Swamp"],
            },
            Seat {
                hand: &["Grizzly Bears", "Forest"],
                battlefield: &[],
            },
        ],
    );
    table.reach_main(P0);
    table.tap_lands(P0);
    let therapy = table.in_hand(P0, "Cabal Therapy");
    table
        .session
        .act(P0, PlayerAction::CastSpell { card: therapy })
        .expect("cast Cabal Therapy");
    for _ in 0..64 {
        let pending = table.session.pending().clone();
        let answer = match &pending {
            Pending::ChooseCardName { .. } => return table,
            // The target player, whichever way it is asked.
            Pending::ChoosePlayer { .. } => PlayerAction::ChoosePlayer(P1),
            Pending::ChooseTargets { .. } => PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![P1],
            },
            other => timeout_answer(other).unwrap_or_else(|| panic!("no default for {other:?}")),
        };
        let seat = pending.asked().expect("a question is open");
        table.session.act(seat, answer).expect("legal");
    }
    panic!("Cabal Therapy never asked for a name");
}
