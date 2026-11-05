//! `cards/sorceries/mv_4/evincar_s_justice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit::damage_on;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn evincar_s_justice() -> CardIndex {
    card_index("c0d3dfc0-1ff2-4ac3-98eb-32eaa5b84c05")
}

/// p0 has `lands` Swamps and a Hill Giant, with Evincar's Justice in hand; p1
/// has Grizzly Bears, two Islands and a Counterspell. p0's main phase.
fn justice_over(lands: usize) -> Engine<RegistryLookup> {
    let mut board = vec![swamp(); lands];
    board.push(index::HILL_GIANT);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[index::GRIZZLY_BEARS, island(), island()])
        .hand(0, &[evincar_s_justice()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// "Deals 2 damage to each creature and each player": both creatures and both
/// players took it.
#[track_caller]
fn assert_two_damage_everywhere(engine: &Engine<RegistryLookup>, giant: ObjectId) {
    assert!(on_battlefield(engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(damage_on(engine, giant), 2);
    assert_eq!(engine.state().players[0].life, 18);
    assert_eq!(engine.state().players[1].life, 18);
}

/// Evincar's Justice (`c0d3dfc0-1ff2-4ac3-98eb-32eaa5b84c05`) — {2}{B}{B} Sorcery:
/// "Buyback {3}. Evincar's Justice deals 2 damage to each creature and each player."
#[test]
fn evincar_s_justice_deals_two_damage_to_each_creature_and_each_player() {
    let mut engine = justice_over(4);

    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, P0, evincar_s_justice());
    pass_until(&mut engine, stack_is_empty);

    assert_two_damage_everywhere(&engine, giant);
    assert_not_bought_back(&engine, P0, evincar_s_justice());
}

/// "Buyback {3}": the {3} is paid on top, and the card comes home.
#[test]
fn evincar_s_justice_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = justice_over(7);
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, P0, evincar_s_justice());
    answer_buyback(&mut engine, P0, true);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}}{{B}}{{B}} and {{3}} are the seven floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_two_damage_everywhere(&engine, giant);
    assert_bought_back(&engine, P0, evincar_s_justice());
}

/// Declined, it costs three less and is an Evincar's Justice that is spent.
#[test]
fn evincar_s_justice_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = justice_over(7);
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, P0, evincar_s_justice());
    answer_buyback(&mut engine, P0, false);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert_two_damage_everywhere(&engine, giant);
    assert_not_bought_back(&engine, P0, evincar_s_justice());
}

/// Countered, it does not resolve: buyback paid, card gone, nobody hurt.
/// "Each creature and each player" targets nothing, so no target can leave it
/// (CR 608.2b).
#[test]
fn evincar_s_justice_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = justice_over(7);
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, P0, evincar_s_justice());
    answer_buyback(&mut engine, P0, true);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(damage_on(&engine, giant), 0);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, evincar_s_justice());
}
