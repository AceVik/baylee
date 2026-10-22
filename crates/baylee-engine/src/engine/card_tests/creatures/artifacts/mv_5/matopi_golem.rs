//! `cards/creatures/artifacts/mv_5/matopi_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::SHATTER;

fn matopi_golem() -> CardIndex {
    card_index("0308b9f2-11da-41d0-8d18-18e8c0c600ab")
}

/// `Matopi Golem` ({5}, 3/3 Artifact Creature — Golem, `Coverage::Partial`)
/// regenerates for `{1}`.
#[test]
fn matopi_golem_regenerates_for_one_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[matopi_golem(), SHATTER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, matopi_golem());
    pass_until(&mut engine, stack_is_empty);
    let golem = on_battlefield(&engine, p0, matopi_golem()).unwrap();

    activate(&mut engine, p0, matopi_golem(), 0);
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, SHATTER);
    aim_at(&mut engine, p0, golem);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, matopi_golem()).is_some());
    assert!(is_tapped(&engine, golem));
}
