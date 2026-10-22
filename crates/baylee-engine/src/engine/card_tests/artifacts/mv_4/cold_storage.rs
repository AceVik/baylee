//! `cards/artifacts/mv_4/cold_storage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn cold_storage() -> CardIndex {
    card_index("e23bf6bd-bc97-425d-902a-c6b606e82b65")
}

/// Cold Storage (e23bf6bd-bc97-425d-902a-c6b606e82b65) — {4} Artifact (Partial):
/// "{3}: Exile target creature you control. Sacrifice this artifact: Return each creature card exiled with this artifact to the battlefield under your control."
#[test]
fn cold_storage_exiles_creature_and_returns_it_when_sacrificed() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                cold_storage(),
                index::GRIZZLY_BEARS,
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, cold_storage(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Exile);

    activate(&mut engine, P0, cold_storage(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, cold_storage()).is_some());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
