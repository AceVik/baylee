//! `cards/instants/mv_3/aven_warcraft.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c152f829-0452-407b-8020-023bf3d75c74"
fn aven_warcraft() -> CardIndex {
    card_index("c152f829-0452-407b-8020-023bf3d75c74")
}

fn fill_graveyard(engine: &mut Engine<RegistryLookup>, seat: PlayerId, count: usize) {
    let cards: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .iter()
        .rev()
        .take(count)
        .copied()
        .collect();
    for id in cards {
        engine
            .dev_state_mut(seat)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Graveyard(seat),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
    }
}

/// Below threshold (6 cards in graveyard): Aven Warcraft ({2}{W}) gives your
/// creatures +0/+2 until end of turn without choosing a color or granting
/// protection.
#[test]
fn aven_warcraft_below_threshold_pumps_only_without_protection() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[aven_warcraft(), shock()],
        &[],
    );
    fill_graveyard(&mut engine, P0, 6);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::White, 3)]);
    cast_with_floating(&mut engine, P0, aven_warcraft());
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, bears), (2, 4), "+0/+2 to your creatures");
    assert_eq!(pt(&engine, giant), (3, 3), "opponent's creature untouched");

    // No protection granted -> red Shock can still target Bears.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, shock(), &[bears], &[]);
    assert_eq!(lk::damage_on(&engine, bears), 2);

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2), "+0/+2 wears off at EOT");
}

/// With threshold (7+ cards in graveyard): gives +0/+2 AND chooses a color so
/// creatures you control gain protection from that color until end of turn.
#[test]
fn aven_warcraft_with_threshold_grants_protection_from_chosen_color() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[aven_warcraft(), shock()],
        &[],
    );
    fill_graveyard(&mut engine, P0, 7);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(&mut engine, P0, &[(ManaColor::White, 3)]);
    cast_with_floating(&mut engine, P0, aven_warcraft());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, bears), (2, 4));
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, shock());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected Shock target choice, got {:?}", engine.pending());
    };
    assert!(
        !options.contains(&bears),
        "threshold granted protection from red"
    );
    assert!(
        options.contains(&giant),
        "opponent's creature is unprotected"
    );
}
