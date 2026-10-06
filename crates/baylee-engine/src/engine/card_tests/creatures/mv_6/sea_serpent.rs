//! `cards/creatures/mv_6/sea_serpent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sea Serpent, "can't attack unless defending player controls an Island"
/// (CR 508.1c): with none on the defending side it is not offered and a
/// declaration naming it is refused — even though its own controller's
/// Island is what keeps the Serpent from being sacrificed, which does not
/// count for the opponent's side. A vanilla Savannah Lions, on the
/// battlefield since the game began like the Serpent, is offered in the
/// same question: this board can offer an attacker, so the Serpent's
/// absence is its restriction at work, not a question that offers nothing
/// at all.
#[test]
fn sea_serpent_cannot_attack_when_the_defending_player_controls_no_island() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sea_serpent(), island(), savannah_lions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let serpent = on_battlefield(&engine, p0, sea_serpent()).expect("seated");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&lions),
        "a vanilla creature on the same board is offered: this question can \
         offer an attacker at all: {attackers:?}"
    );
    assert!(
        !attackers.contains(&serpent),
        "no Island on the defending side, and its own controller's Island \
         does not count: {attackers:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(serpent, Defender::Player(p1))]
                }
            )
            .is_err(),
        "a declaration naming it is refused"
    );
}

/// The same restriction, the other side of it: an Island on the defending
/// player's side and the Serpent is offered and attacks.
#[test]
fn sea_serpent_attacks_when_the_defending_player_controls_an_island() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sea_serpent(), island()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let serpent = on_battlefield(&engine, p0, sea_serpent()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&serpent),
        "an Island on the defending side offers it: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(serpent, Defender::Player(p1))],
            },
        )
        .expect("an Island across the table lets it attack");
    assert!(engine.state().combat.is_attacking(serpent));
}

/// "When you control no Islands, sacrifice this creature": with an Island
/// standing the whole time, the Serpent survives an entire turn of its
/// controller's own — through combat, second main and the end step —
/// without the trigger ever firing.
#[test]
fn a_sea_serpent_with_its_own_island_survives_its_controllers_whole_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sea_serpent(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let serpent = on_battlefield(&engine, p0, sea_serpent()).expect("seated");

    reach_their_main_phase(&mut engine, p1);

    assert_eq!(
        on_battlefield(&engine, p0, sea_serpent()),
        Some(serpent),
        "an Island standing the whole time, so the state trigger never fired"
    );
}

/// "When you control no Islands, sacrifice this creature" (CR 603.8):
/// destroying the Serpent's controller's last Island puts the ability on
/// the stack, and resolving it sacrifices the Serpent.
#[test]
fn a_sea_serpent_sacrifices_itself_when_its_last_island_is_destroyed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[sea_serpent(), island(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[stone_rain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let serpent = on_battlefield(&engine, p0, sea_serpent()).expect("seated");
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
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, sea_serpent()).is_none()
    });

    assert!(
        on_battlefield(&engine, p0, island()).is_none(),
        "the last Island is gone"
    );
    assert!(
        in_graveyard(&engine, p0, sea_serpent()).is_some(),
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
                } if source == serpent
            )),
        "the sacrifice came from the state trigger (ability index 1), not \
         from Stone Rain itself"
    );
}

/// The condition is what makes the ability trigger, not an "if" it
/// rechecks as it resolves — this printed sentence has no intervening "if"
/// clause for CR 603.4 to apply to — so an Island arriving while the
/// ability is on the stack does not stop it (CR 603.8).
#[test]
fn a_new_island_while_the_sacrifice_trigger_waits_does_not_save_the_sea_serpent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[sea_serpent(), island(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[stone_rain(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let serpent = on_battlefield(&engine, p0, sea_serpent()).expect("seated");
    let isle = on_battlefield(&engine, p0, island()).expect("seated");

    cast_from_hand(&mut engine, p0, stone_rain());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![isle],
                players: vec![],
            },
        )
        .expect("its own Island is a legal target for \"destroy target land\"");

    // Stops the instant the Island is gone and the sacrifice trigger has
    // taken its place on the stack, with p0 — who acts next — holding
    // priority: the dev move below is read fresh from there on.
    let before = engine.journal().entries().len();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, island()).is_none() && !stack_is_empty(e)
    });
    assert_eq!(
        on_battlefield(&engine, p0, sea_serpent()),
        Some(serpent),
        "the trigger is only waiting to resolve so far"
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
                } if source == serpent
            )),
        "what is waiting on the stack is the state trigger (ability index 1)"
    );

    let fresh_island = in_hand(&engine, p0, island()).expect("a second Island is in hand");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            fresh_island,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("the harness moves a card");
    assert!(
        on_battlefield(&engine, p0, island()).is_some(),
        "an Island is back under its control before the trigger resolves"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, sea_serpent()).is_some(),
        "the ability had already triggered on the earlier absence and \
         sacrifices the Serpent regardless of the Island that arrived after"
    );
}
