//! `cards/creatures/mv_3/mindstab_thrull.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mindstab_thrull() -> CardIndex {
    card_index("43abe1f3-7b92-411a-bf93-e158f949685c")
}

/// Turn one: the Thrull attacks into an empty board with the opponent holding
/// four cards. Returns the engine at the "you may sacrifice it" question.
fn unblocked_thrull() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mindstab_thrull()])
        .hand(1, &[forest(), island(), plains(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let thrull = on_battlefield(&engine, P0, mindstab_thrull()).expect("the Thrull is out");
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
        .expect("the Thrull attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    (engine, thrull)
}

/// Mindstab Thrull (43abe1f3-7b92-411a-bf93-e158f949685c) — "Whenever this
/// creature attacks and isn't blocked, you may sacrifice it. If you do,
/// defending player discards three cards." Sacrificed: the defender (not me)
/// chooses and discards three, and takes no combat damage.
#[test]
fn mindstab_thrull_sacrifices_itself_to_make_the_defender_discard_three() {
    let (mut engine, thrull) = unblocked_thrull();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    assert_eq!(hand_before, 4);
    engine.apply(P0, PlayerAction::YesNo(true)).expect("yes");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the defender is asked what to discard, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, P1, "defending player chooses");
    assert_eq!((min, max), (3, 3), "three cards");
    assert_eq!(options.len(), 4);
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: options.into_iter().take(3).collect(),
            },
        )
        .expect("three discarded");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        1,
        "four cards minus three"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P1)).len(),
        3
    );
    assert!(
        in_graveyard(&engine, P0, mindstab_thrull()).is_some(),
        "the Thrull was sacrificed"
    );
    assert!(
        engine
            .state()
            .object(thrull)
            .is_some_and(|o| o.zone != Zone::Battlefield)
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "it left combat before damage"
    );
}

/// Declined: no discard, and the Thrull hits for two.
#[test]
fn mindstab_thrull_declined_keeps_everything_and_deals_damage() {
    let (mut engine, thrull) = unblocked_thrull();
    engine.apply(P0, PlayerAction::YesNo(false)).expect("no");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P1)).len(), 4);
    assert_eq!(
        engine.state().object(thrull).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(engine.state().players[1].life, 18);
}
