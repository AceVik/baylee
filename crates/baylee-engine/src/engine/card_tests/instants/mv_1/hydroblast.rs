//! `cards/instants/mv_1/hydroblast.rs`, played.
//!
//! Oracle:
//! - "Choose one —"
//! - "• Counter target spell if it's red."
//! - "• Destroy target permanent if it's red."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hydroblast() -> CardIndex {
    card_index("b94c0336-2ca8-4bb1-bb1f-e9babd1826ad")
}

#[test]
fn hydroblast_destroys_red_permanent_and_leaves_nonred_permanent_alone() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[hydroblast(), hydroblast()],
        &[],
    );
    let red_giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let green_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    lk::cast_modal(&mut engine, P0, hydroblast(), 1, 0, &[red_giant], &[]);
    assert_eq!(lk::zone_of(&engine, red_giant), Zone::Graveyard);

    lk::cast_modal(&mut engine, P0, hydroblast(), 1, 0, &[green_bears], &[]);
    assert_eq!(
        lk::zone_of(&engine, green_bears),
        Zone::Battlefield,
        "Hydroblast does not destroy a non-red permanent"
    );
}

/// Player 1 casts `spell` from hand with `mana` floating, aimed as given;
/// player 0 answers with Hydroblast's first mode, "counter target spell if
/// it's red", aimed at it. Stops with the stack empty.
fn hydroblast_in_response(
    engine: &mut Engine<RegistryLookup>,
    spell: CardIndex,
    mana: &[(ManaColor, u32)],
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    lk::pass_once(engine);
    lk::float(engine, P1, mana);
    cast_with_floating(engine, P1, spell);
    lk::aim(engine, objects, players);
    let on_the_stack = on_stack(engine, spell).expect("player 1's spell is on the stack");
    lk::pass_once(engine);
    lk::float(engine, P0, &[(ManaColor::Blue, 1)]);
    lk::announce(engine, P0, hydroblast(), 0, 0, &[on_the_stack], &[]);
    lk::settle(engine);
}

#[test]
fn hydroblast_counters_a_red_spell() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[hydroblast()],
        &[index::LIGHTNING_BOLT],
    );
    hydroblast_in_response(
        &mut engine,
        index::LIGHTNING_BOLT,
        &[(ManaColor::Red, 1)],
        &[],
        &[P0],
    );
    assert_eq!(
        lk::life(&engine, P0),
        20,
        "the Bolt was countered before it dealt damage"
    );
    assert!(in_graveyard(&engine, P1, index::LIGHTNING_BOLT).is_some());
    assert!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "the first mode was chosen: the permanent mode's candidate is untouched"
    );
}

#[test]
fn hydroblast_targets_a_nonred_spell_and_leaves_it_alone() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[hydroblast()],
        &[index::GIANT_GROWTH],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    hydroblast_in_response(
        &mut engine,
        index::GIANT_GROWTH,
        &[(ManaColor::Green, 1)],
        &[bears],
        &[],
    );
    assert!(in_graveyard(&engine, P0, hydroblast()).is_some());
    assert_eq!(
        power_of(&engine, bears),
        Some(5),
        "a green spell is a legal target, and Hydroblast does not counter it"
    );
}
