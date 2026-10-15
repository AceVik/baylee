//! Oracle: “At the beginning of your upkeep, you gain 1 life.”
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

fn nyx_fleece_ram() -> CardIndex {
    card_index("5a20113c-7ea8-4edf-af9f-148ccd326a88")
}

/// Checklist: an upkeep trigger gains exactly one life for its controller;
/// it does not trigger on the opponent's upkeep.
#[test]
fn nyx_fleece_ram_gains_life_on_its_controllers_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, plains())
        .battlefield(0, &[nyx_fleece_ram()])
        .life(0, 18)
        .life(1, 20)
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    assert_eq!(game.state().players[0].life, 19);
    assert_eq!(game.state().players[1].life, 20);

    assert!(on_battlefield(&game, p0, nyx_fleece_ram()).is_some());
    let mut saw_opponent_upkeep = false;
    for _ in 0..200 {
        match game.pending().clone() {
            Pending::Priority { player, .. } => {
                let own_upkeep = game.state().turn.active == p0
                    && game.state().turn.step == Step::Upkeep
                    && game.state().turn.number > 1;
                if own_upkeep {
                    break;
                }
                saw_opponent_upkeep |=
                    game.state().turn.active == p1 && game.state().turn.step == Step::Upkeep;
                game.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => game
                .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                .unwrap(),
            Pending::ChooseBlockers { player, .. } => game
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .unwrap(),
            other => panic!("unexpected while checking opponent upkeep: {other:?}"),
        }
    }
    assert!(saw_opponent_upkeep);
    assert_eq!(
        game.state().players[0].life,
        19,
        "no gain during the opponent's upkeep"
    );
    assert_eq!(game.state().players[1].life, 20);
}
