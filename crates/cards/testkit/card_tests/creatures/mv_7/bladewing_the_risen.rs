//! `cards/creatures/mv_7/bladewing_the_risen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn bladewing_the_risen_returns_dragon_from_graveyard_on_etb_and_pumps_dragons() {
    let bladewing = card_index("c22cc70a-9533-4bfc-b3ce-b2d43a73f0c1");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, index::SHIVAN_DRAGON)
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                swamp(),
                mountain(),
            ],
        )
        .hand(0, &[bladewing])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let shivan_gy = in_graveyard(&engine, p0, index::SHIVAN_DRAGON).unwrap();

    // Spend 7 lands to cast Bladewing the Risen ({3}{B}{B}{R}{R}).
    let lands = lands_of(&engine, p0);
    for &l in &lands[..7] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: l })
            .unwrap();
    }
    cast_with_floating(&mut engine, p0, bladewing);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![shivan_gy],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let bladewing_id = on_battlefield(&engine, p0, bladewing).unwrap();
    assert!(keywords(&engine, bladewing_id).contains(KeywordSet::FLYING));
    let shivan_id = on_battlefield(&engine, p0, index::SHIVAN_DRAGON)
        .expect("Shivan Dragon returned to the battlefield");

    // Tap remaining Swamp + Mountain for {B}{R} and activate Bladewing's pump ability.
    for &l in &lands[7..9] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: l })
            .unwrap();
    }
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bladewing_id,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bladewing_id), (5, 5));
    assert_eq!(pt(&engine, shivan_id), (6, 6));
}
