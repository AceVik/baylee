//! `cards/enchantments/mv_4/parallax_wave.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn parallax_wave_exiles_creatures_and_returns_them_when_leaving_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(
            0,
            &[
                card_index("0accfb04-bc8b-4cab-997a-2337ac42db8f"),
                index::DISENCHANT,
            ],
        )
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(
        &mut engine,
        p0,
        card_index("0accfb04-bc8b-4cab-997a-2337ac42db8f"),
    );
    pass_until(&mut engine, stack_is_empty);

    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("opponent Grizzly Bears");
    activate(
        &mut engine,
        p0,
        card_index("0accfb04-bc8b-4cab-997a-2337ac42db8f"),
        1,
    );
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none(),
        "Grizzly Bears exiled by Parallax Wave"
    );

    let wave = on_battlefield(
        &engine,
        p0,
        card_index("0accfb04-bc8b-4cab-997a-2337ac42db8f"),
    )
    .expect("Parallax Wave");
    cast_with_floating(&mut engine, p0, index::DISENCHANT);
    aim_at(&mut engine, p0, wave);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_some(),
        "Grizzly Bears returned to battlefield when Parallax Wave left"
    );
}
