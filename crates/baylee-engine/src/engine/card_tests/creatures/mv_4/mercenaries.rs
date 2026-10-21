//! `cards/creatures/mv_4/mercenaries.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mercenaries() -> CardIndex {
    card_index("9d96671c-7a98-48ec-b479-bc451e18264d")
}

/// Mercenaries (9d96671c-7a98-48ec-b479-bc451e18264d) — {3}{W} 3/3 Creature — Human Mercenary:
/// Coverage::Partial — plays as a 3/3 creature.
#[test]
fn mercenaries_casts_for_three_and_a_white_and_attacks_for_three() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[mercenaries()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mercenaries());
    pass_until(&mut engine, stack_is_empty);

    let merc = on_battlefield(&engine, P0, mercenaries()).expect("Mercenaries entered");
    assert_eq!(pt(&engine, merc), (3, 3));

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(merc, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 17);
}
