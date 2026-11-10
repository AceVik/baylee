//! `cards/creatures/artifacts/mv_4/dinotomaton.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dinotomaton() -> CardIndex {
    card_index("443f2272-6475-4966-8ca2-717d4cf208ee")
}

/// Dinotomaton — {3}{R} 4/3 with menace: "When this creature enters, target
/// creature you control gains menace until end of turn." Aimed at a Grizzly
/// Bears (only creatures its controller has are on offer), the Bears attack
/// into a lone Bears that cannot block them alone, and the menace is gone
/// next turn.
#[test]
fn dinotomaton_gives_a_creature_menace() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[grizzly_bears(), mountain(), forest(), forest(), forest()],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[dinotomaton()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert!(!keywords(&engine, bears).contains(KeywordSet::MENACE));

    cast_from_hand(&mut engine, p0, dinotomaton());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for this");
    };
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    assert!(options.contains(&bears), "my creature is a target");
    assert!(!options.contains(&theirs), "theirs is not: \"you control\"");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .expect("the Bears are a legal target");
    pass_until(&mut engine, stack_is_empty);
    let golem = on_battlefield(&engine, p0, dinotomaton()).expect("it resolved");
    assert!(
        keywords(&engine, golem).contains(KeywordSet::MENACE),
        "printed"
    );
    assert_eq!(pt(&engine, golem), (4, 3));
    assert!(
        keywords(&engine, bears).contains(KeywordSet::MENACE),
        "granted"
    );

    let blocks = attack_and_collect_blocks(&mut engine, bears, p1);
    assert!(
        blocks.iter().all(|b| !b.attackers.contains(&bears)),
        "menace: one Bears cannot block alone: {blocks:?}"
    );
    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert!(
        !keywords(&engine, bears).contains(KeywordSet::MENACE),
        "until end of turn"
    );
}
