//! `cards/creatures/mv_4/crovax_the_cursed.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn crovax_the_cursed() -> CardIndex {
    card_index("e1b74ef7-ab6b-4d40-8087-3f3a448b7e0a")
}

/// Crovax the Cursed (`e1b74ef7-ab6b-4d40-8087-3f3a448b7e0a`) — {2}{B}{B} 0/0 Legendary Creature — Vampire Noble (Partial):
/// "This creature enters with four +1/+1 counters on it.
/// At the beginning of your upkeep, you may sacrifice a creature. If you do, put a +1/+1 counter on Crovax.
/// {B}: Crovax gains flying until end of turn."
#[test]
fn crovax_the_cursed_enters_four_four_sacrifices_at_upkeep_and_gains_flying() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[crovax_the_cursed()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, crovax_the_cursed());
    pass_until(&mut engine, stack_is_empty);

    let crovax = on_battlefield(&engine, P0, crovax_the_cursed()).unwrap();
    assert_eq!(pt(&engine, crovax), (4, 4));

    activate(&mut engine, P0, crovax_the_cursed(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, crovax).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, P1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == P0),
    );
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(pt(&engine, crovax), (5, 5));
}
