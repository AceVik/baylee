//! Oracle: destroy target nonartifact creature.
#[allow(clippy::wildcard_imports)]
use super::*;
fn go_for_the_throat() -> CardIndex {
    card_index("2f092562-9e17-43cd-aeb8-d0567f99363e")
}
/// Checklist: creature target is destroyed; noncreature/player excluded; bystander survives.
#[test]
fn destroys_one_creature() {
    let p = PlayerId::new(0);
    let mut g = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                llanowar_elves(),
                wild_elephant(),
                ornithopter(),
            ],
        )
        .hand(0, &[go_for_the_throat()])
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    let a = on_battlefield(&g, p, llanowar_elves()).unwrap();
    let b = on_battlefield(&g, p, wild_elephant()).unwrap();
    let artifact_creature = on_battlefield(&g, p, ornithopter()).unwrap();
    cast_from_hand(&mut g, p, go_for_the_throat());
    let Pending::ChooseTargets { options, .. } = g.pending().clone() else {
        panic!("creature target")
    };
    assert!(options.contains(&a) && options.contains(&b));
    assert!(
        !options.contains(&artifact_creature),
        "artifact creatures are excluded"
    );
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![a],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut g, stack_is_empty);
    assert!(on_battlefield(&g, p, llanowar_elves()).is_none());
    assert!(in_graveyard(&g, p, llanowar_elves()).is_some());
    assert_eq!(on_battlefield(&g, p, wild_elephant()), Some(b));
}
