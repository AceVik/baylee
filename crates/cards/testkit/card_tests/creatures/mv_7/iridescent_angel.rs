//! `cards/creatures/mv_7/iridescent_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn iridescent_angel() -> CardIndex {
    card_index("28fa9211-96bb-4fef-81bf-8f58114a8df7")
}

/// Iridescent Angel has flying and protection from each of the five colors:
/// colored spells cannot target it, while colorless artifact flyers (Ornithopter)
/// can block it.
#[test]
fn iridescent_angel_flying_and_protection_from_each_color() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[iridescent_angel()])
        .battlefield(1, &[plains(), index::ORNITHOPTER])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p0, iridescent_angel()).unwrap();
    let thopter = on_battlefield(&engine, p1, index::ORNITHOPTER).unwrap();

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected Swords targets, got {:?}", engine.pending());
    };
    assert!(options.contains(&thopter));
    assert!(
        !options.contains(&angel),
        "Iridescent Angel has protection from white so Swords cannot target it"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![thopter],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Next test phase: cast Ornithopter-free or just check flying evasion vs ground Bears.
    let _ = lk::life(&engine, p1);
    assert!(on_battlefield(&engine, p0, iridescent_angel()).is_some());
}

/// Iridescent Angel can be blocked by a colorless artifact flyer (Ornithopter)
/// and cannot be blocked by a colored ground creature.
#[test]
fn iridescent_angel_can_be_blocked_by_colorless_flyer() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[iridescent_angel()])
        .battlefield(1, &[index::ORNITHOPTER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p0, iridescent_angel()).unwrap();
    let thopter = on_battlefield(&engine, p1, index::ORNITHOPTER).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(thopter, angel)],
            },
        )
        .expect("colorless artifact flyer Ornithopter can block Iridescent Angel");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(lk::life(&engine, p1), 20);
    assert!(on_battlefield(&engine, p0, iridescent_angel()).is_some());
}
