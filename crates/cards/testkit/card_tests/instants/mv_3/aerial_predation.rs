//! Oracle scenarios for Aerial Predation.
#[allow(clippy::wildcard_imports)]
use super::*;
fn aerial_predation() -> CardIndex {
    card_index("6bac1495-16cf-4e1e-9126-482f85322c78")
}

// Checklist: only a flying creature is targetable; it is destroyed and its
// controller distinction does not redirect the caster's 2 life gain.
#[test]
fn aerial_predation_destroys_flyer_gains_two_and_leaves_nonflyer() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[aerial_predation()])
        .battlefield(1, &[air_elemental(), llanowar_elves()])
        .life(0, 17)
        .life(1, 20)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let flyer = on_battlefield(&e, p1, air_elemental()).unwrap();
    let elf = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    cast_from_hand(&mut e, p0, aerial_predation());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("target flyer")
    };
    assert_eq!(options, vec![flyer]);
    assert!(
        !options.contains(&elf),
        "the nonflying creature is not a legal target"
    );
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![flyer],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(e.state().players[1].life, 20);
    assert!(in_graveyard(&e, p1, air_elemental()).is_some());
    assert!(on_battlefield(&e, p1, llanowar_elves()).is_some());
}
