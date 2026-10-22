//! `cards/sorceries/mv_2/torrent_of_lava.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn torrent_of_lava() -> CardIndex {
    card_index("f467ec85-ffe2-4963-82e0-509a7873fd63")
}

fn grizzly_bears() -> CardIndex {
    baylee_core::generated::index::GRIZZLY_BEARS
}

fn wind_drake() -> CardIndex {
    baylee_core::generated::index::WIND_DRAKE
}

/// Torrent of Lava (`{X}{R}{R}`, `Coverage::Partial`): deals X damage to each
/// creature without flying.
#[test]
fn torrent_of_lava_deals_x_damage_to_each_creature_without_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 4])
        .battlefield(1, &[grizzly_bears(), wind_drake()])
        .hand(0, &[torrent_of_lava()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, torrent_of_lava());
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "non-flying 2/2 dies to X=2 damage"
    );
    assert!(
        on_battlefield(&engine, p1, wind_drake()).is_some(),
        "flying creature is unharmed"
    );
}
