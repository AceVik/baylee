//! `cards/instants/mv_3/shoving_match.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::choice::granted_slot;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "1ef07ae5-3b04-45c8-a080-852b84da4fef"
fn shoving_match() -> CardIndex {
    card_index("1ef07ae5-3b04-45c8-a080-852b84da4fef")
}

fn granted_index(engine: &Engine<RegistryLookup>, source: ObjectId) -> Option<u32> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    legal
        .abilities
        .iter()
        .find_map(|&(id, idx)| (id == source && granted_slot(idx).is_some()).then_some(idx))
}

/// Shoving Match ({2}{U}): until end of turn, ALL creatures (yours and
/// opponent's) gain "{T}: Tap target creature", and the ability disappears on
/// the next turn.
#[test]
fn shoving_match_grants_tap_creature_ability_to_all_creatures_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        &[index::HILL_GIANT, index::GRAY_OGRE],
        &[shoving_match()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let ogre = lk::obj(&engine, P1, index::GRAY_OGRE);

    // Advance past P1's first turn so neither side has summoning sickness.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut engine, P0, shoving_match());
    lk::settle(&mut engine);

    // P0 activates Bears's granted ability to tap P1's Hill Giant.
    let bears_grant = granted_index(&engine, bears).expect("Bears gained tap ability");
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: bears,
                ability_index: bears_grant,
            },
        )
        .expect("Bears gained {T}: Tap target creature");
    lk::aim(&mut engine, &[giant], &[]);

    // Pass priority to P1 so P1 activates Ogre's granted ability on P0's Lions.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    let ogre_grant = granted_index(&engine, ogre).expect("opponent's creature gained tap ability");
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: ogre,
                ability_index: ogre_grant,
            },
        )
        .expect("opponent's creature also gained {T}: Tap target creature");
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![lions],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(is_tapped(&engine, bears) && is_tapped(&engine, giant));
    assert!(is_tapped(&engine, ogre) && is_tapped(&engine, lions));

    // Next turn: the granted ability is gone.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        granted_index(&engine, giant).is_none() && granted_index(&engine, ogre).is_none(),
        "granted tap ability wore off at end of turn"
    );
}
