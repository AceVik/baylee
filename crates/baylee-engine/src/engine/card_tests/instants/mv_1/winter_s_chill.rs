//! Winter's Chill — `{X}{U}` Instant (Ice Age)
//!
//! Cast this spell only during combat before blockers are declared.
//! Tap X target attacking creatures. For each of those creatures, its
//! controller may pay `{1}` or `{2}` life. If the player doesn't, prevent all
//! combat damage that would be dealt by and to that creature this turn.
//! (`Coverage::Partial`: unconditionally prevents all damage dealt by and to X
//! target attacking creatures this turn.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn winter_s_chill() -> CardIndex {
    card_index("ae78d498-11c7-427f-9d13-3f1b7096f803")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn winter_s_chill_can_only_be_cast_before_blockers_and_prevents_damage_to_and_from_x_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[winter_s_chill(), winter_s_chill()])
        .battlefield(
            0,
            &[island(), island(), island(), island(), grizzly_bears()],
        )
        .battlefield(1, &[hill_giant(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, winter_s_chill()).unwrap();
    let our_blocker = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let attacker = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let non_attacker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // Advance to p1's declare-attackers step; Hill Giant attacks, Grizzly Bears stays back.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    assert!(
        priority_offer(&engine).castable.contains(&spell),
        "Winter's Chill is castable during declare-attackers step"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();

    // Only attacking creatures can be targeted; non-attackers are excluded.
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(options, &[attacker]);
    assert!(!options.contains(&non_attacker));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Block Hill Giant (3/3) with Grizzly Bears (2/2): damage both ways is prevented!
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(our_blocker, attacker)],
            },
        )
        .unwrap();

    // During declare-blockers step (after blockers are declared), a second Winter's Chill cannot be cast even with mana and an attacking creature.
    let second_spell = in_hand(&engine, p0, winter_s_chill()).unwrap();
    assert!(
        !priority_offer(&engine).castable.contains(&second_spell),
        "Winter's Chill cannot be cast after blockers are declared"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(our_blocker).unwrap().zone,
        Zone::Battlefield,
        "Grizzly Bears survives because damage dealt by Hill Giant was prevented"
    );
    assert_eq!(
        engine.state().object(our_blocker).unwrap().damage,
        0,
        "no damage marked on Grizzly Bears"
    );
    assert_eq!(
        engine.state().object(attacker).unwrap().damage,
        0,
        "damage dealt to Hill Giant was also prevented"
    );
}
