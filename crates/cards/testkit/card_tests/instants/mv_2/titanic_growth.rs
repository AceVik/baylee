//! Oracle: target creature gets +4/+4 until end of turn.
#[allow(clippy::wildcard_imports)]
use super::*;
fn titanic_growth() -> CardIndex {
    card_index("61e09dd9-7870-48c2-9177-d6abc3162692")
}
/// Checklist: only a creature is targeted; exact equal +4/+4 applies for resolution.
#[test]
fn grants_exact_four_four_to_the_chosen_creature() {
    let p = PlayerId::new(0);
    let mut g = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves(), wild_elephant()])
        .hand(0, &[titanic_growth()])
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    let c = on_battlefield(&g, p, llanowar_elves()).unwrap();
    let b = on_battlefield(&g, p, wild_elephant()).unwrap();
    cast_from_hand(&mut g, p, titanic_growth());
    let Pending::ChooseTargets { options, .. } = g.pending().clone() else {
        panic!("target")
    };
    assert!(options.contains(&c) && options.contains(&b));
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![c],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut g, stack_is_empty);
    assert_eq!(pt(&g, c), (5, 5));
    assert_eq!(pt(&g, b), (3, 3));
    pass_until(&mut g, |e| e.state().turn.active != p);
    assert_eq!(pt(&g, c), (1, 1), "the +4/+4 expires at cleanup");
    assert_eq!(pt(&g, b), (3, 3));
}
