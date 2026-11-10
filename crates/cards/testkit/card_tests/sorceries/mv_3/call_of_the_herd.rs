//! `cards/sorceries/mv_3/call_of_the_herd.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn call_of_the_herd() -> CardIndex {
    card_index("ee243f81-f51c-4d9a-a396-f7cef84b46c1")
}

fn elephants(engine: &Engine<RegistryLookup>, player: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|object| {
                object.controller == player
                    && object.card.is_none()
                    && object
                        .characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::creature::ELEPHANT)
            })
        })
        .collect()
}

/// Both casts create exactly one green 3/3 Elephant under the caster's control.
/// Flashback costs {3}{G}, not the printed {2}{G}, and exiles the card afterwards.
#[test]
fn call_of_the_herd_creates_an_elephant_and_flashes_back_for_four_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 7])
        .hand(0, &[call_of_the_herd()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, call_of_the_herd());
    pass_until(&mut engine, stack_is_empty);
    let first = elephants(&engine, p0);
    assert_eq!(first.len(), 1);
    assert_eq!(pt(&engine, first[0]), (3, 3));
    assert!(
        engine
            .state()
            .object(first[0])
            .unwrap()
            .characteristics()
            .colors
            .contains(baylee_core::color::Color::Green)
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    assert!(in_graveyard(&engine, p0, call_of_the_herd()).is_some());

    let spell = in_graveyard(&engine, p0, call_of_the_herd()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let all = elephants(&engine, p0);
    assert_eq!(all.len(), 2, "each cast creates one token");
    assert!(all.iter().all(|&id| pt(&engine, id) == (3, 3)));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(spell).unwrap().zone, Zone::Exile);
}
