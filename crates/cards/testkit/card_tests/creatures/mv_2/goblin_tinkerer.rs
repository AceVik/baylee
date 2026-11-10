//! `cards/creatures/mv_2/goblin_tinkerer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MANDIBLE_JUSTICIAR};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_tinkerer() -> CardIndex {
    card_index("0afc2d2f-4fdb-440f-9c35-45bf103c0437")
}

/// Goblin Tinkerer (0afc2d2f-4fdb-440f-9c35-45bf103c0437) — {1}{R} 1/2:
/// "{R}, {T}: Destroy target artifact. That artifact deals damage equal to its
/// mana value to this creature."
///
/// Sol Ring (mana value 1) is destroyed and deals 1 to the Tinkerer, which
/// survives with 1 damage. Only an artifact is on the menu: the Bears are not.
#[test]
fn goblin_tinkerer_destroys_artifact_and_takes_mana_value_damage() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_tinkerer(), mountain()])
        .battlefield(1, &[sol_ring(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let tinkerer = on_battlefield(&engine, P0, goblin_tinkerer()).unwrap();
    let ring = on_battlefield(&engine, P1, sol_ring()).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, goblin_tinkerer(), 0);
    let menu = aim_at(&mut engine, P0, ring);
    assert!(menu.contains(&ring), "{menu:?}");
    assert!(!menu.contains(&bears), "only an artifact: {menu:?}");
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, sol_ring()).is_none());
    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().object(tinkerer).unwrap().damage, 1);
}

/// "That artifact deals damage": the artifact is the source, not the Tinkerer.
/// A lifelink Mandible Justiciar (mana value 2) is destroyed, deals 2 to the
/// Tinkerer (toughness 2, so it dies) and its controller gains 2 from the
/// lifelink, which damage from the Tinkerer would not give.
#[test]
fn goblin_tinkerer_damage_comes_from_the_artifact_so_lifelink_counts() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_tinkerer(), mountain()])
        .battlefield(1, &[MANDIBLE_JUSTICIAR])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let justiciar = on_battlefield(&engine, P1, MANDIBLE_JUSTICIAR).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, goblin_tinkerer(), 0);
    aim_at(&mut engine, P0, justiciar);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, MANDIBLE_JUSTICIAR).is_none());
    assert!(
        on_battlefield(&engine, P0, goblin_tinkerer()).is_none(),
        "2 damage to a toughness-2 Tinkerer"
    );
    assert_eq!(
        engine.state().players[1].life,
        22,
        "the artifact's lifelink gained its controller 2"
    );
}
