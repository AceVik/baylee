//! `cards/creatures/artifacts/mv_5/anodet_lurker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn anodet_lurker() -> CardIndex {
    card_index("5c2dbe90-5784-4bc4-a715-76a61c1ecfbc")
}

/// Anodet Lurker — "When this creature dies, you gain 3 life." The "you" is
/// the controller of the dying Lurker: seated under either seat, that seat
/// gains and the other does not.
#[test]
fn anodet_lurker_gains_its_controller_three_life_when_it_dies() {
    let p0 = PlayerId::new(0);
    for index in [0u8, 1] {
        let owner = usize::from(index);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(owner, &[anodet_lurker()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let seat = PlayerId::new(index);
        let lurker = on_battlefield(&engine, seat, anodet_lurker()).expect("seated");
        assert_eq!(pt(&engine, lurker), (3, 3), "the printed body");
        let (mine, theirs) = (life_of(&engine, owner), life_of(&engine, 1 - owner));

        kill(&mut engine, lurker);
        assert!(in_graveyard(&engine, seat, anodet_lurker()).is_some());
        assert_eq!(life_of(&engine, owner), mine + 3, "its controller gains 3");
        assert_eq!(
            life_of(&engine, 1 - owner),
            theirs,
            "the other seat gains nothing"
        );
    }
}
