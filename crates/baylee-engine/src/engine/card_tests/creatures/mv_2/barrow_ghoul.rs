//! `cards/creatures/mv_2/barrow_ghoul.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn barrow_ghoul() -> CardIndex {
    card_index("1610875f-48da-490c-a371-c439dc36f533")
}

/// `Barrow Ghoul` ({1}{B}, 4/4 Creature — Zombie, `Coverage::Partial`)
/// sacrifices itself at the beginning of your upkeep unless you exile a
/// creature card from your graveyard.
#[test]
fn barrow_ghoul_sacrifices_on_upkeep_unless_creature_card_exiled_from_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, GRIZZLY_BEARS)
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[barrow_ghoul()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let bears = in_graveyard(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, barrow_ghoul());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, barrow_ghoul()).is_some());
    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_none());
}
