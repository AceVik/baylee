//! `cards/creatures/artifacts/mv_3/reinforced_bulwark.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn reinforced_bulwark() -> CardIndex {
    card_index("8916b3d4-6a9a-420a-8ee7-be67b2f47201")
}

/// Reinforced Bulwark — "{T}: Prevent the next 1 damage that would be dealt
/// to you this turn." A Lightning Bolt at its own controller deals 3 unless
/// the Bulwark has been tapped first, when it deals 2.
#[test]
fn reinforced_bulwark_prevents_the_next_one_damage_to_you() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[reinforced_bulwark(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bulwark = on_battlefield(&engine, p0, reinforced_bulwark()).expect("seated");
    assert_eq!(pt(&engine, bulwark), (0, 4));
    assert!(keywords(&engine, bulwark).contains(KeywordSet::DEFENDER));
    let mountain_id = on_battlefield(&engine, p0, mountain()).expect("seated");
    tap_mana_where(&mut engine, p0, |id| id == mountain_id);

    activate(&mut engine, p0, reinforced_bulwark(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, bulwark));

    cast_with_floating(&mut engine, p0, lightning_bolt());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at_player(&mut engine, p0, p0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, 0), 18, "3 damage less the 1 prevented");
}

/// The control: the same Bolt with no prevention deals all 3.
#[test]
fn reinforced_bulwark_unused_prevents_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[reinforced_bulwark(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mountain_id = on_battlefield(&engine, p0, mountain()).expect("seated");
    tap_mana_where(&mut engine, p0, |id| id == mountain_id);

    cast_with_floating(&mut engine, p0, lightning_bolt());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at_player(&mut engine, p0, p0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, 0), 17);
}
