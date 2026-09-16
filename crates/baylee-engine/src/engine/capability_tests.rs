//! What a seat is allowed to do beyond answering its own choices.
//!
//! There used to be a `GamePreset::dev_mode` flag. Nothing read it, and it
//! arrived over the wire in `CreateGame` — so the one thing it did do was
//! let whoever opened the socket ask to be trusted. Capabilities replaced
//! it: per seat, granted by the host, never inbound, and empty by default.

use super::testkit::{
    Duel, card_index, keep_mulligans, on_battlefield, reach_main_phase, seed_graveyard,
};
use super::*;
use baylee_core::ids::CardIndex;
use baylee_core::preset::SeatCapabilities;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// A seat with no `dev_commands` capability cannot reach the state, and in a
/// lobby game that is every seat. The old `state_mut_dev()` asked nobody:
/// anything holding an `&mut Engine` could rewrite the board, which is one
/// careless admin endpoint away from a ranked game being editable.
#[test]
fn only_a_seat_granted_dev_commands_can_reach_the_state() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(31, forest()).start();
    assert!(
        engine.dev_state_mut(p0).is_some(),
        "the test harness grants itself dev commands"
    );

    let mut plain = Duel::new(31, forest()).without_capabilities().start();
    assert!(
        plain.dev_state_mut(p0).is_none(),
        "a seat with no capability was handed the game state"
    );
    assert!(
        plain.dev_state_mut(PlayerId::new(9)).is_none(),
        "a seat that does not exist got an answer at all"
    );
    assert_eq!(plain.capabilities(p0), SeatCapabilities::default());
}

fn deathrite_shaman() -> CardIndex {
    card_index("22f1a4a4-c423-4d1c-8775-0ed604a9fa51")
}

/// A dev command that rewrites the board asks the question again.
///
/// The capability hands out `&mut GameState`, so the engine cannot know when
/// the rewriting is over and `Engine::refresh_offer` is the explicit half of
/// that bargain — `seed_graveyard` calls it, and this is what says so. Both
/// halves are struck deliberately: the ability is absent while the graveyard
/// is empty and present once it is not, so a `refresh_offer` that did nothing
/// at all would fail the second assert rather than pass both.
#[test]
fn a_seeded_graveyard_changes_what_the_offer_says() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[deathrite_shaman()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let shaman = on_battlefield(&engine, p0, deathrite_shaman()).expect("the Shaman is out");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(shaman, 0)),
        "\"exile target land card from a graveyard\" has nothing to point at \
         while every graveyard is empty: {:?}",
        legal.abilities
    );

    seed_graveyard(&mut engine, p0, 1);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(shaman, 0)),
        "a land is buried and the published offer still describes the board \
         from before the dev command: {:?}",
        legal.abilities
    );
}
