//! Oracle: target creature gets -3/-1 until end of turn.
#[allow(clippy::wildcard_imports)]
use super::*;
fn wring_flesh() -> CardIndex {
    card_index("45bb536c-c173-4b6e-a5a8-354c74fd93a1")
}
/// Checklist: creature only; exact -3/-1 applies, then expires at cleanup.
#[test]
fn applies_minus_three_minus_one_to_target() {
    let p = PlayerId::new(0);
    let mut g = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), llanowar_elves(), wild_elephant()])
        .hand(0, &[wring_flesh()])
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    let a = on_battlefield(&g, p, wild_elephant()).unwrap();
    let b = on_battlefield(&g, p, llanowar_elves()).unwrap();
    cast_from_hand(&mut g, p, wring_flesh());
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![a],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut g, stack_is_empty);
    assert_eq!(
        pt(&g, a),
        (0, 2),
        "the power fell by three and toughness by one"
    );
    assert_eq!(pt(&g, b), (1, 1));
    pass_until(&mut g, |e| e.state().turn.active != p);
    assert_eq!(pt(&g, a), (3, 3), "the shrink expires at cleanup");
}
