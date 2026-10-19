//! `cards/creatures/mv_3/necrite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn necrite() -> CardIndex {
    card_index("e6ab8a04-e225-4ff0-bb8b-c80d0e6ce50c")
}

/// Turn one: Necrite attacks unblocked, with a Gray Ogre on each side, and the
/// trigger has asked for (and been given) the opponent's Ogre as its target.
/// Returns the engine at the "you may sacrifice it" question, the target and
/// the menu the target question offered.
fn unblocked_necrite(shield: bool) -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>) {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[necrite(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let thrull = on_battlefield(&engine, P0, necrite()).expect("Necrite is out");
    let theirs = on_battlefield(&engine, P1, gray_ogre()).expect("their Ogre is out");
    if shield {
        engine
            .state
            .object_mut(theirs)
            .expect("their Ogre")
            .regeneration_shields = 1;
    }
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(thrull, Defender::Player(P1))],
            },
        )
        .expect("Necrite attacks");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let menu = aim_at(&mut engine, P0, theirs);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    (engine, theirs, menu)
}

/// Necrite (e6ab8a04-e225-4ff0-bb8b-c80d0e6ce50c) — "Whenever this creature
/// attacks and isn't blocked, you may sacrifice it. If you do, destroy target
/// creature defending player controls. It can't be regenerated." Sacrificed:
/// the defender's creature dies, mine stays, no combat damage.
#[test]
fn necrite_sacrifices_itself_to_destroy_a_defending_creature() {
    let (mut engine, theirs, menu) = unblocked_necrite(false);
    let mine = on_battlefield(&engine, P0, gray_ogre()).expect("my Ogre is out");
    assert!(menu.contains(&theirs));
    assert!(
        !menu.contains(&mine),
        "only creatures the defending player controls are targets"
    );
    engine.apply(P0, PlayerAction::YesNo(true)).expect("yes");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(
        on_battlefield(&engine, P0, necrite()).is_none(),
        "sacrificed"
    );
    assert!(in_graveyard(&engine, P0, necrite()).is_some());
    assert!(
        on_battlefield(&engine, P1, gray_ogre()).is_none(),
        "their Ogre was destroyed"
    );
    assert!(in_graveyard(&engine, P1, gray_ogre()).is_some());
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_some(),
        "my Ogre was not"
    );
    assert_eq!(engine.state().players[1].life, 20, "Necrite left combat");
}

/// "It can't be regenerated": a regeneration shield does not save it.
#[test]
fn necrite_destruction_ignores_a_regeneration_shield() {
    let (mut engine, theirs, _menu) = unblocked_necrite(true);
    assert_eq!(
        engine
            .state()
            .object(theirs)
            .expect("their Ogre")
            .regeneration_shields,
        1
    );
    engine.apply(P0, PlayerAction::YesNo(true)).expect("yes");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, P1, gray_ogre()).is_none(),
        "destroyed in spite of the shield"
    );
    assert!(in_graveyard(&engine, P1, gray_ogre()).is_some());
}

/// Declined: nobody dies and Necrite deals its two to the player.
#[test]
fn necrite_declined_leaves_everything_and_hits_the_player() {
    let (mut engine, theirs, _menu) = unblocked_necrite(false);
    engine.apply(P0, PlayerAction::YesNo(false)).expect("no");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(on_battlefield(&engine, P0, necrite()).is_some());
    assert_eq!(
        engine.state().object(theirs).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(engine.state().players[1].life, 18);
}
