//! `cards/sorceries/mv_4/massacre.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn massacre_gives_all_creatures_minus_two_minus_two_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[card_index("ce1866e9-28de-4bcc-9999-11958e6c13a6")])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), index::HILL_GIANT])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p0, index::HILL_GIANT).expect("Hill Giant");
    cast_from_hand(
        &mut engine,
        p0,
        card_index("ce1866e9-28de-4bcc-9999-11958e6c13a6"),
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (1, 1));
    assert_eq!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS), None);
}
