//! `cards/creatures/mv_2/dandan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dandân — `{U}{U}` 4/1 Fish: "This creature can't attack unless defending
/// player controls an Island." (CR 508.1c: the legality of an attacking
/// creature is checked as the declaration is made, against the defending
/// player's board at that moment.)
///
/// Its own controller keeps an Island so the card's other half ("When you
/// control no Islands, sacrifice this creature") never fires, and a vanilla
/// Savannah Lions stands beside it so the absence below is the restriction
/// and not a question that offers no attacker at all. The defender has no
/// Island: the attacker's own Island being present is what says the word
/// "defending" is the one being read.
#[test]
fn dandan_cannot_attack_when_the_defending_player_controls_no_island() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dandan(), island(), savannah_lions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let fish = on_battlefield(&engine, p0, dandan()).expect("seated");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&lions),
        "a vanilla creature on the same board is offered, so this question can \
         offer an attacker at all: {attackers:?}"
    );
    assert!(
        !attackers.contains(&fish),
        "no Island on the defending side, and its own controller's Island does \
         not count: {attackers:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(fish, Defender::Player(p1))]
                }
            )
            .is_err(),
        "a declaration naming it is refused (CR 508.1c)"
    );
}

/// The same restriction from the other side: an Island on the *defending*
/// player's side and Dandân is offered and attacks. The attacker's Island
/// stands in both tests, so the difference between them is exactly the
/// defender's board.
#[test]
fn dandan_attacks_when_the_defending_player_controls_an_island() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dandan(), island()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let fish = on_battlefield(&engine, p0, dandan()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&fish),
        "an Island on the defending side offers it: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fish, Defender::Player(p1))],
            },
        )
        .expect("an Island across the table lets it attack");
    assert!(engine.state().combat.is_attacking(fish));
}

/// "When you control no Islands, sacrifice this creature" (CR 603.8): a
/// state trigger, not an "if" — destroying Dandân's last Island puts the
/// ability on the stack, and resolving it takes the Fish. The journal names
/// ability 1, so the sacrifice is read as this trigger and not as Stone
/// Rain's own doing.
#[test]
fn dandan_sacrifices_itself_when_its_last_island_is_destroyed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dandan(), island(), mountain(), mountain(), mountain()])
        .hand(0, &[stone_rain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let fish = on_battlefield(&engine, p0, dandan()).expect("seated");
    let isle = on_battlefield(&engine, p0, island()).expect("seated");

    cast_from_hand(&mut engine, p0, stone_rain());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"destroy target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&isle));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![isle],
                players: vec![],
            },
        )
        .expect("its own Island is a legal target for \"destroy target land\"");

    let before = engine.journal().entries().len();
    pass_until(&mut engine, |e| on_battlefield(e, p0, dandan()).is_none());

    assert!(
        on_battlefield(&engine, p0, island()).is_none(),
        "the last Island is gone"
    );
    assert!(
        in_graveyard(&engine, p0, dandan()).is_some(),
        "\"sacrifice this creature\" put it in its owner's graveyard"
    );
    assert!(
        engine.journal().entries()[before..]
            .iter()
            .any(|e| matches!(
                e.event,
                crate::event::GameEvent::AbilityTriggered {
                    source,
                    ability_index: 1,
                    ..
                } if source == fish
            )),
        "the sacrifice came from the state trigger (ability index 1), not from \
         Stone Rain itself"
    );
}
