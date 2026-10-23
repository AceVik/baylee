//! `cards/enchantments/mv_4/mortuary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn mortuary() -> CardIndex {
    card_index("8bd2af55-5a48-4300-a893-ae6cd548db24")
}

/// Mortuary (`8bd2af55-5a48-4300-a893-ae6cd548db24`) — {3}{B} Enchantment (Implemented):
/// "Whenever a creature is put into your graveyard from the battlefield, put that card on top of your library."
#[test]
fn mortuary_puts_dying_owned_creature_on_top_of_library() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mortuary(), index::GRIZZLY_BEARS, mountain()])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "Grizzly Bears should have died"
    );
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P0))
        .last()
        .expect("library should not be empty");
    assert_eq!(
        engine.state().object(top).unwrap().card.map(|c| c.index),
        Some(index::GRIZZLY_BEARS)
    );
}

#[test]
fn ownership_not_control_decides_whose_graveyard_and_library_are_used() {
    let mut e = lk::game(
        SEED,
        &[mortuary(), index::GRIZZLY_BEARS],
        &[index::SAVANNAH_LIONS],
        &[lightning_bolt(), lightning_bolt()],
        &[],
    );
    lk::float(&mut e, P0, &[(ManaColor::Red, 2)]);
    let enemy = lk::obj(&e, lk::P1, index::SAVANNAH_LIONS);
    lk::cast_at(&mut e, P0, lightning_bolt(), &[enemy], &[]);
    assert!(in_graveyard(&e, lk::P1, index::SAVANNAH_LIONS).is_some());
    let mine = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(mine)
        .unwrap()
        .controller = lk::P1;
    e.refresh_offer();
    lk::cast_at(&mut e, P0, lightning_bolt(), &[mine], &[]);
    let top = *e
        .state()
        .zones
        .list(ZoneLocation::Library(P0))
        .last()
        .unwrap();
    assert_eq!(
        e.state().object(top).unwrap().card.unwrap().index,
        index::GRIZZLY_BEARS
    );
}
