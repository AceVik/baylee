//! Keeper of Tresserhorn — `{5}{B}` 6/6 Creature — Avatar (Alliances)
//!
//! Whenever this creature attacks and isn't blocked, it assigns no combat
//! damage this turn and defending player loses 2 life.

#[allow(clippy::wildcard_imports)]
use super::*;

fn keeper_of_tresserhorn() -> CardIndex {
    card_index("ce976cb2-2b69-4d00-918c-d4900244ac86")
}

#[test]
fn keeper_of_tresserhorn_unblocked_assigns_no_combat_damage_and_defender_loses_2_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[keeper_of_tresserhorn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p0);

    let keeper = on_battlefield(&engine, p0, keeper_of_tresserhorn()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(keeper, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "defending player loses 2 life and takes 0 combat damage from 6/6 Keeper of Tresserhorn"
    );
}

/// "Whenever this creature attacks and isn't blocked": blocked by a Grizzly Bears it is just a
/// 6/6 that kills it, and the defending player loses no life.
#[test]
fn keeper_of_tresserhorn_blocked_deals_its_combat_damage_and_costs_no_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[keeper_of_tresserhorn()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let keeper = on_battlefield(&engine, p0, keeper_of_tresserhorn()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(keeper, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, keeper)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "it dealt damage"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "no life lost: it was blocked"
    );
}
