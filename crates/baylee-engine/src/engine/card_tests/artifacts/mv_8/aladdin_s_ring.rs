//! `cards/artifacts/mv_8/aladdin_s_ring.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Aladdin's Ring` is an artifact under `Coverage::Implemented` with an activated ability costing `{8}` and tapping to deal 4 damage to any target.
/// The choice offers both creatures and players as legal targets while excluding noncreature artifacts.
/// Targeting the opponent player deals 4 damage upon resolution, reducing their life total from 20 to 16.
#[test]
fn aladdins_ring_taps_and_pays_eight_mana_to_deal_four_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
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
                aladdin_s_ring(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ring = on_battlefield(&engine, p0, aladdin_s_ring()).expect("Ring deployed");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("Elf deployed");

    assert!(!is_tapped(&engine, ring), "Ring starts untapped");

    // Float {8} from the eight Mountains while keeping Aladdin's Ring untapped.
    tap_all_mana_but(&mut engine, p0, Some(aladdin_s_ring()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Mountains produce eight mana"
    );

    activate(&mut engine, p0, aladdin_s_ring(), 0);

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "activating player chooses target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&elf),
        "creatures are valid targets for any target: {options:?}"
    );
    assert!(
        !options.contains(&ring),
        "the artifact is not a legal target: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "both players are valid targets: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeting opponent player is legal");

    assert!(
        is_tapped(&engine, ring),
        "Aladdin's Ring tapped as activation cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{8}} was paid from the pool"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        16,
        "opponent took 4 damage from Aladdin's Ring"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "activator life is unchanged"
    );
}
