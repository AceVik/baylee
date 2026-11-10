//! `cards/creatures/artifacts/mv_4/filigree_sages.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn filigree_sages() -> CardIndex {
    card_index("d6e78c6a-7555-4818-8054-1cdc0fca92e6")
}

/// Filigree Sages — {3}{U} 2/3: "{2}{U}: Untap target artifact." The Sages
/// are an artifact themselves: tapped by attacking, they untap themselves for
/// {2}{U} in the second main phase, and a Grizzly Bears (not an artifact) is
/// not among the targets.
#[test]
fn filigree_sages_untaps_a_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                filigree_sages(),
                grizzly_bears(),
                island(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let sages = on_battlefield(&engine, p0, filigree_sages()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sages, Defender::Player(p1))],
            },
        )
        .expect("the Sages attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(is_tapped(&engine, sages), "attacking tapped them");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, filigree_sages(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability targets: {:?}", engine.pending());
    };
    assert!(options.contains(&sages), "an artifact");
    assert!(!options.contains(&bears), "the Bears are not");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![sages],
                players: vec![],
            },
        )
        .expect("the Sages are a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, sages), "untapped");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{2}}{{U}}");
}
