//! `cards/lands/utility/blighted_fen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blighted Fen: "{T}: Add {C}." / "{4}{B}, {T}, Sacrifice this land: Target opponent sacrifices a creature of their choice."
/// Paid with five Swamps, the sacrifice ability targets the opponent.
/// On resolution, the opponent chooses and sacrifices Llanowar Elves while Blighted Fen is in its owner's graveyard.
#[test]
fn blighted_fen_forces_opponent_to_sacrifice_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(132, forest())
        .battlefield(
            0,
            &[blighted_fen(), swamp(), swamp(), swamp(), swamp(), swamp()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fen = on_battlefield(&engine, p0, blighted_fen()).expect("Fen deployed");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("Elves deployed");

    tap_mana_except(&mut engine, p0, fen);
    activate(&mut engine, p0, blighted_fen(), 1);

    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert!(player_options.contains(&p1));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player: chooser,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(chooser, p1);
    assert!(options.contains(&elves));

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p0, blighted_fen()).is_none());
    assert!(in_graveyard(&engine, p0, blighted_fen()).is_some());
}
