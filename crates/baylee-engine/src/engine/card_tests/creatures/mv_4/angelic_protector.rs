//! `cards/creatures/mv_4/angelic_protector.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn angelic_protector() -> CardIndex {
    card_index("cff434e1-e567-4a80-8dc5-593d70b0f348")
}

/// Angelic Protector (cff434e1-e567-4a80-8dc5-593d70b0f348) — {3}{W} Creature — Angel:
/// "Flying. Whenever this creature becomes the target of a spell or ability, this creature gets +0/+3 until end of turn."
#[test]
fn angelic_protector_gets_plus_zero_plus_three_when_targeted() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[angelic_protector(), mountain()])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let protector = on_battlefield(&engine, P0, angelic_protector()).unwrap();
    assert_eq!(pt(&engine, protector), (2, 2));

    cast_from_hand(&mut engine, P0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P0, protector);
    pass_until(&mut engine, stack_is_empty);

    // BecomesTarget resolves before Lightning Bolt, raising toughness to 5 so it survives 3 damage.
    assert!(on_battlefield(&engine, P0, angelic_protector()).is_some());
    assert_eq!(pt(&engine, protector), (2, 5));
}
