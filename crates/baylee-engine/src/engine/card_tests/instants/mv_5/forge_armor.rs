//! `cards/instants/mv_5/forge_armor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::JUGGERNAUT;

fn forge_armor() -> CardIndex {
    card_index("7d4d9bfa-7e85-461b-bd62-1de97690a110")
}

/// Forge Armor — {4}{R} Instant
/// "As an additional cost to cast this spell, sacrifice an artifact."
/// "Put X +1/+1 counters on target creature, where X is the sacrificed artifact's mana value."
#[test]
fn forge_armor_sacrifices_artifact_and_puts_mana_value_plus_one_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[forge_armor()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                JUGGERNAUT,
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let juggernaut = on_battlefield(&engine, p0, JUGGERNAUT).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, forge_armor());
    // Target creature: Grizzly Bears
    aim_at(&mut engine, p0, bear);
    // Additional cost: sacrifice an artifact (Juggernaut, MV 4)
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![juggernaut],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, JUGGERNAUT).is_none());
    // Grizzly Bears (2/2) gets 4 +1/+1 counters -> 6/6.
    assert_eq!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        4
    );
    assert_eq!(pt(&engine, bear), (6, 6));
}
