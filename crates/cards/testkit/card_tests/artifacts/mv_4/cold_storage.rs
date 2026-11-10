//! `cards/artifacts/mv_4/cold_storage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cold_storage() -> CardIndex {
    card_index("e23bf6bd-bc97-425d-902a-c6b606e82b65")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn in_exile(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|object| object.card.is_some_and(|printing| printing.index == card))
        })
}

/// p0 has Cold Storage, Grizzly Bears, a Hill Giant and six Plains; p1 has a
/// Hill Giant. p0's first main phase.
fn a_cold_storage_over_creatures() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                cold_storage(),
                grizzly_bears(),
                hill_giant(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Pays {3} and exiles `target` with Cold Storage, returning the menu the
/// target question offered.
#[track_caller]
fn exile_with_storage(engine: &mut Engine<RegistryLookup>, target: ObjectId) -> Vec<ObjectId> {
    tap_all_mana(engine, P0);
    activate(engine, P0, cold_storage(), 0);
    let menu = aim_at(engine, P0, target);
    pass_until(engine, stack_is_empty);
    menu
}

/// Cold Storage (e23bf6bd-bc97-425d-902a-c6b606e82b65) — {4} Artifact (Partial):
/// "{3}: Exile target creature you control. Sacrifice this artifact: Return each creature card exiled with this artifact to the battlefield under your control."
#[test]
fn cold_storage_exiles_only_a_creature_you_control() {
    let mut engine = a_cold_storage_over_creatures();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();
    let their_giant = on_battlefield(&engine, P1, hill_giant()).unwrap();

    let menu = exile_with_storage(&mut engine, bears);

    assert!(menu.contains(&bears) && menu.contains(&giant));
    assert!(
        !menu.contains(&their_giant),
        "\"target creature you control\" leaves the opponent's creature off the menu: {menu:?}"
    );
    assert!(in_exile(&engine, P0, grizzly_bears()));
    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_none());
    assert!(
        on_battlefield(&engine, P0, hill_giant()).is_some(),
        "only the target left"
    );
    assert!(on_battlefield(&engine, P1, hill_giant()).is_some());
}

#[test]
fn cold_storage_does_not_offer_a_return_to_the_wrong_controller() {
    let mut engine = a_cold_storage_over_creatures();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    exile_with_storage(&mut engine, bears);
    let source = on_battlefield(&engine, P0, cold_storage()).unwrap();
    assert!(!ability_offered(&engine, source, 1));
    assert!(in_exile(&engine, P0, grizzly_bears()));
}
