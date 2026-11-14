//! `cards/1995/src/ice/melting.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn melting() -> CardIndex {
    card_index("599247f2-7f88-40ab-be08-e284e5f231d9")
}

fn supertypes_of(engine: &Engine<RegistryLookup>, id: ObjectId) -> SupertypeSet {
    engine
        .state()
        .object(id)
        .expect("object exists")
        .characteristics()
        .supertypes
}

/// While Melting is on the battlefield, snow lands on both sides lose the Snow
/// supertype while keeping Basic and their land subtypes; when Melting leaves,
/// Snow returns immediately.
#[test]
fn melting_removes_snow_supertype_from_all_lands_and_restores_when_removed() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                index::SNOW_COVERED_FOREST,
            ],
        )
        .battlefield(1, &[index::SNOW_COVERED_ISLAND])
        .hand(0, &[melting(), index::DISENCHANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_snow = on_battlefield(&engine, P0, index::SNOW_COVERED_FOREST).unwrap();
    let their_snow = on_battlefield(&engine, P1, index::SNOW_COVERED_ISLAND).unwrap();
    let plain_mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    let mountains = all_on_battlefield(&engine, P0, mountain());
    let plains_ids = all_on_battlefield(&engine, P0, plains());

    assert!(supertypes_of(&engine, my_snow).contains(SupertypeSet::SNOW));
    assert!(supertypes_of(&engine, their_snow).contains(SupertypeSet::SNOW));
    assert!(!supertypes_of(&engine, plain_mtn).contains(SupertypeSet::SNOW));

    tap_mana_where(&mut engine, P0, |id| mountains.contains(&id));
    cast_with_floating(&mut engine, P0, melting());
    pass_until(&mut engine, stack_is_empty);

    let melt = on_battlefield(&engine, P0, melting()).expect("Melting resolved");
    assert!(!supertypes_of(&engine, my_snow).contains(SupertypeSet::SNOW));
    assert!(!supertypes_of(&engine, their_snow).contains(SupertypeSet::SNOW));
    assert!(supertypes_of(&engine, my_snow).contains(SupertypeSet::BASIC));
    assert!(supertypes_of(&engine, their_snow).contains(SupertypeSet::BASIC));
    assert!(
        engine
            .state()
            .object(my_snow)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::FOREST)
    );
    assert!(
        engine
            .state()
            .object(their_snow)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::ISLAND)
    );

    tap_mana_where(&mut engine, P0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, P0, index::DISENCHANT);
    aim_at(&mut engine, P0, melt);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, melting()).is_none());
    assert!(supertypes_of(&engine, my_snow).contains(SupertypeSet::SNOW));
    assert!(supertypes_of(&engine, their_snow).contains(SupertypeSet::SNOW));
}

/// Because Melting removes Snow from lands in Layer 4, spells whose casting
/// condition checks for a snow land (Blizzard) are locked out until Melting is
/// destroyed.
#[test]
fn melting_disables_snow_land_cast_condition_until_destroyed() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                forest(),
                forest(),
                forest(),
                forest(),
                plains(),
                plains(),
                index::SNOW_COVERED_FOREST,
            ],
        )
        .hand(0, &[melting(), index::BLIZZARD, index::DISENCHANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mountains = all_on_battlefield(&engine, P0, mountain());
    let forests = all_on_battlefield(&engine, P0, forest());
    let plains_ids = all_on_battlefield(&engine, P0, plains());

    tap_mana_where(&mut engine, P0, |id| mountains.contains(&id));
    cast_with_floating(&mut engine, P0, melting());
    pass_until(&mut engine, stack_is_empty);
    let melt = on_battlefield(&engine, P0, melting()).expect("Melting on board");

    tap_mana_where(&mut engine, P0, |id| id == forests[0] || id == forests[1]);
    let blizz = in_hand(&engine, P0, index::BLIZZARD).expect("Blizzard in hand");
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(!legal.castable.contains(&blizz));
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: blizz })
            .is_err()
    );

    tap_mana_where(&mut engine, P0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, P0, index::DISENCHANT);
    aim_at(&mut engine, P0, melt);
    pass_until(&mut engine, stack_is_empty);

    tap_mana_where(&mut engine, P0, |id| id == forests[2] || id == forests[3]);
    cast_with_floating(&mut engine, P0, index::BLIZZARD);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, index::BLIZZARD).is_some());
}
