//! `cards/artifacts/mv_0/tormod_s_crypt.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tormod_s_crypt() -> CardIndex {
    card_index("1573f7f9-672c-421a-b1ac-3d0d8aea59ca")
}

fn graveyard(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .clone()
}

/// Names `victim` for "target player", whichever way the engine asks.
#[track_caller]
fn name_player(engine: &mut Engine<RegistryLookup>, seat: PlayerId, victim: PlayerId) {
    match engine.pending().clone() {
        Pending::ChoosePlayer { .. } => {
            engine
                .apply(seat, PlayerAction::ChoosePlayer(victim))
                .expect("the player was on the offer");
        }
        Pending::ChooseTargets { .. } => {
            engine
                .apply(
                    seat,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![victim],
                    },
                )
                .expect("the player was on the offer");
        }
        other => panic!("expected the player question, got {other:?}"),
    }
}

/// A board with the Crypt, two cards in p0's graveyard and three in p1's.
fn a_crypt_over_two_graveyards() -> (Engine<RegistryLookup>, Vec<ObjectId>, Vec<ObjectId>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tormod_s_crypt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 2);
    seed_graveyard(&mut engine, p1, 3);
    let mine = graveyard(&engine, p0);
    let theirs = graveyard(&engine, p1);
    assert_eq!((mine.len(), theirs.len()), (2, 3));
    (engine, mine, theirs)
}

fn exiled(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.zone == Zone::Exile)
}

/// Tormod's Crypt (1573f7f9-672c-421a-b1ac-3d0d8aea59ca) — "{T}, Sacrifice this
/// artifact: Exile target player's graveyard."
///
/// Aimed at the opponent: all three of their cards are exiled, p0's own two
/// stay, and the Crypt, paid as a cost, is in p0's graveyard beside them.
#[test]
fn tormod_s_crypt_exiles_the_targeted_players_graveyard_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, mine, theirs) = a_crypt_over_two_graveyards();
    activate(&mut engine, p0, tormod_s_crypt(), 0);
    // The target is chosen before the cost is paid (CR 602.2b, 601.2c, 601.2h).
    name_player(&mut engine, p0, p1);
    assert!(
        on_battlefield(&engine, p0, tormod_s_crypt()).is_none(),
        "sacrificed as a cost, before the ability resolves"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        graveyard(&engine, p1).is_empty(),
        "their graveyard is empty"
    );
    for id in &theirs {
        assert!(exiled(&engine, *id), "{id:?} was exiled");
    }
    let mine_now = graveyard(&engine, p0);
    for id in &mine {
        assert!(mine_now.contains(id), "p0's own {id:?} was left alone");
    }
    assert!(
        in_graveyard(&engine, p0, tormod_s_crypt()).is_some(),
        "the sacrificed Crypt sits in its owner's graveyard"
    );
}

/// "Target player": the controller may name themselves. The Crypt was
/// sacrificed to pay for the ability, so it is in the graveyard it exiles
/// as the ability resolves, and the opponent's graveyard is untouched.
#[test]
fn tormod_s_crypt_can_exile_its_own_controllers_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, mine, theirs) = a_crypt_over_two_graveyards();
    activate(&mut engine, p0, tormod_s_crypt(), 0);
    name_player(&mut engine, p0, p0);
    pass_until(&mut engine, stack_is_empty);

    assert!(graveyard(&engine, p0).is_empty(), "p0's graveyard is empty");
    for id in &mine {
        assert!(exiled(&engine, *id), "{id:?} was exiled");
    }
    assert!(
        in_graveyard(&engine, p0, tormod_s_crypt()).is_none(),
        "the Crypt itself went with it"
    );
    assert_eq!(
        graveyard(&engine, p1),
        theirs,
        "the opponent's is untouched"
    );
}
