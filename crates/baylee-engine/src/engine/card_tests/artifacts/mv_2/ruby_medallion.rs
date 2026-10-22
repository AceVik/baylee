//! `cards/artifacts/mv_2/ruby_medallion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn ruby_medallion() -> CardIndex {
    card_index("0c367958-729e-416d-988b-098e90e7a1fd")
}

/// `Ruby Medallion` ({2}, Artifact, `Coverage::Implemented`):
/// `Red spells you cast cost {1} less to cast.`
#[test]
fn ruby_medallion_reduces_the_generic_cost_of_red_spells_you_cast_by_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        // 3 Mountains + Ruby Medallion: enough to cast Hill Giant ({3}{R} -> {2}{R}).
        .battlefield(0, &[mountain(), mountain(), mountain(), ruby_medallion()])
        .hand(0, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, HILL_GIANT);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, HILL_GIANT).is_some());
}
