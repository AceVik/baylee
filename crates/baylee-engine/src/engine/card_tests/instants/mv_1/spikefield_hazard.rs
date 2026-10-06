//! `cards/instants/mv_1/spikefield_hazard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spikefield Hazard, front face: one damage, which is the half it claims.
///
/// The "exile it instead of dying" replacement is refused by name, so what a
/// game shows is a 1/1 taking one damage and going to the graveyard — where
/// the printed card would have exiled it.
#[test]
fn spikefield_hazard_deals_one_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(415, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[spikefield_hazard()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is seated");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, spikefield_hazard());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "one damage on a 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and it is in the graveyard, which is the half the card refuses: the \
         printing exiles it instead"
    );
}
