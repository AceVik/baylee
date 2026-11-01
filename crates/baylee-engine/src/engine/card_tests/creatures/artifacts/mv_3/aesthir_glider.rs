//! Aesthir Glider — `{3}` 2/1 Artifact Creature — Bird Construct (Dominaria)
//!
//! Flying
//! This creature can't block.

#[allow(clippy::wildcard_imports)]
use super::*;

fn aesthir_glider() -> CardIndex {
    card_index("99dcde0b-4f4d-4d7a-89aa-8a89c256e38f")
}

/// "This creature can't block": on the opponent's turn the Glider is not a legal blocker, and
/// declaring no blockers is.
#[test]
fn aesthir_glider_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 3])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[aesthir_glider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, aesthir_glider());
    pass_until(&mut engine, stack_is_empty);
    let glider = on_battlefield(&engine, p0, aesthir_glider()).expect("Aesthir Glider enters");
    assert_eq!(pt(&engine, glider), (2, 1));
    assert!(keywords(&engine, glider).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, glider).contains(KeywordSet::CANT_BLOCK));

    // On P1's turn, Grizzly Bears attacks and Aesthir Glider cannot block it.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::DeclareAttackers
    });
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(glider, bears)],
                },
            )
            .is_err(),
        "Aesthir Glider has CANT_BLOCK and cannot be declared as a blocker"
    );
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
}

/// "Flying": an untapped ground creature may block a ground attacker but not the Glider. The
/// opponent's Grizzly Bears is untapped here (it has not attacked), so only the Glider's flying
/// keeps it from blocking.
#[test]
fn aesthir_glider_can_only_be_blocked_by_flyers_and_reachers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[aesthir_glider(), gray_ogre()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let glider = on_battlefield(&engine, p0, aesthir_glider()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(glider, Defender::Player(p1)), (ogre, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, glider)],
                },
            )
            .is_err(),
        "a creature without flying or reach cannot block a flyer"
    );
    // The same Bears may block the Gray Ogre: it is the Glider's flying that is the difference.
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, ogre)],
            },
        )
        .expect("the Bears may block a ground creature");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the Glider's 2 damage went through; the Ogre was blocked"
    );
}
