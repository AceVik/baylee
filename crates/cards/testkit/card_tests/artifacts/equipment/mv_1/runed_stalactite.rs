//! `cards/artifacts/equipment/mv_1/runed_stalactite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn runed_stalactite() -> CardIndex {
    card_index("1c169307-71f5-4679-8b5a-d60c1109b9db")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn has_subtype(
    engine: &Engine<RegistryLookup>,
    id: ObjectId,
    sub: baylee_core::ids::SubtypeId,
) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .characteristics()
        .subtypes
        .contains(sub)
}

fn equip_stalactite(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    stalactite: ObjectId,
    host: ObjectId,
) {
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: stalactite,
                ability_index: 0,
            },
        )
        .expect("equip {2}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// Runed Stalactite (1c169307-71f5-4679-8b5a-d60c1109b9db) — {1} Artifact — Equipment:
/// "Equipped creature gets +1/+1 and is every creature type."
/// "Equip {2}"
///
/// Gives +1/+1 and every creature type only to the equipped creature; moving it
/// removes both the +1/+1 bonus and the extra creature types from the old host.
#[test]
fn runed_stalactite_grants_plus_one_and_every_creature_type_until_unequipped() {
    use baylee_core::generated::subtypes::creature as ct;

    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                runed_stalactite(),
                grizzly_bears(),
                hill_giant(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stalactite = on_battlefield(&engine, p0, runed_stalactite()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    assert!(!has_subtype(&engine, bears, ct::GOBLIN));

    // Equip {2} to Grizzly Bears.
    tap_all_mana(&mut engine, p0);
    equip_stalactite(&mut engine, p0, stalactite, bears);

    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert_eq!(pt(&engine, bears), (3, 3), "2/2 + 1/+1 = 3/3");
    for kind in [
        ct::BEAR,
        ct::ELF,
        ct::GOBLIN,
        ct::VAMPIRE,
        ct::SLIVER,
        ct::WIZARD,
    ] {
        assert!(has_subtype(&engine, bears, kind));
    }
    assert!(!has_subtype(&engine, giant, ct::GOBLIN));

    // Move Runed Stalactite to Hill Giant ({2}): Bears loses +1/+1 and extra types.
    equip_stalactite(&mut engine, p0, stalactite, giant);
    assert_eq!(pt(&engine, bears), (2, 2));
    assert!(!has_subtype(&engine, bears, ct::GOBLIN));
    assert_eq!(pt(&engine, giant), (4, 4), "3/3 + 1/+1 = 4/4");
    assert!(has_subtype(&engine, giant, ct::GOBLIN));
}
