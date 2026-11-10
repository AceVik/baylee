//! `cards/sorceries/mv_4/acorn_harvest.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::{color::Color, generated::subtypes};

fn acorn_harvest() -> CardIndex {
    card_index("6aeba615-2d37-4eb8-a377-41a517e95aa3")
}

fn assert_green_squirrels(engine: &Engine<RegistryLookup>, player: PlayerId, count: usize) {
    let squirrels = tokens_of(engine, player);
    assert_eq!(squirrels.len(), count);
    for squirrel in squirrels {
        let object = engine.state().object(squirrel).unwrap();
        assert_eq!(pt(engine, squirrel), (1, 1));
        assert!(object.characteristics().colors.contains(Color::Green));
        assert!(
            object
                .characteristics()
                .subtypes
                .contains(subtypes::creature::SQUIRREL)
        );
    }
}

/// The ordinary spell creates two Squirrels. Flashback creates two more for
/// {1}{G} plus exactly 3 life, rather than the printed {3}{G}, then exiles.
#[test]
fn acorn_harvest_creates_two_squirrels_on_each_cast_and_pays_flashback_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 6])
        .hand(0, &[acorn_harvest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, acorn_harvest());
    pass_until(&mut engine, stack_is_empty);
    assert_green_squirrels(&engine, p0, 2);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert_eq!(life_of(&engine, p0), 20);
    let harvest = in_graveyard(&engine, p0, acorn_harvest()).unwrap();

    engine
        .apply(p0, PlayerAction::CastSpell { card: harvest })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine
            .apply(p0, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
    assert_eq!(life_of(&engine, p0), 17, "the life is paid while casting");
    pass_until(&mut engine, stack_is_empty);
    assert_green_squirrels(&engine, p0, 4);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(harvest).unwrap().zone, Zone::Exile);
}
