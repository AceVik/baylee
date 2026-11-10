//! `cards/instants/mv_2/imps_taunt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn imps_taunt() -> CardIndex {
    card_index("7ebec169-d941-460e-9e3c-ba54b5066b05")
}

/// p0 has `lands` Swamps and Grizzly Bears, with Imps' Taunt in hand; p1 has
/// two Islands and a Counterspell. p0's main phase.
fn a_taunt_for_the_bears(lands: usize) -> Engine<RegistryLookup> {
    let mut board = vec![swamp(); lands];
    board.push(index::GRIZZLY_BEARS);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[island(), island()])
        .hand(0, &[imps_taunt()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Walks to the attack and answers whether staying home is refused.
#[track_caller]
fn must_attack(engine: &mut Engine<RegistryLookup>) -> bool {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .is_err()
}

/// Imps' Taunt (`7ebec169-d941-460e-9e3c-ba54b5066b05`) — {1}{B} Instant:
/// "Buyback {3}. Target creature attacks this turn if able."
#[test]
fn imps_taunt_forces_target_creature_to_attack_this_turn() {
    let mut engine = a_taunt_for_the_bears(2);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, imps_taunt());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(must_attack(&mut engine));
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert_not_bought_back(&engine, P0, imps_taunt());
    assert!(!must_attack(&mut engine), "only this turn");
}

/// "Buyback {3}": the {3} is paid on top, the creature must still attack, and
/// the card comes home.
#[test]
fn imps_taunt_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_taunt_for_the_bears(5);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, imps_taunt());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{B}} and {{3}} are the five floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_bought_back(&engine, P0, imps_taunt());
    assert!(must_attack(&mut engine));
}

/// Declined, it costs three less and is an Imps' Taunt that is spent.
#[test]
fn imps_taunt_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_taunt_for_the_bears(5);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, imps_taunt());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, imps_taunt());
    assert!(must_attack(&mut engine));
}

/// Countered, it does not resolve: buyback paid, card gone, the creature free
/// to stay home.
#[test]
fn imps_taunt_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_taunt_for_the_bears(5);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, imps_taunt());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, imps_taunt());
    assert!(!must_attack(&mut engine));
}

/// With its one target gone it does not resolve either.
#[test]
fn imps_taunt_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = a_taunt_for_the_bears(5);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, imps_taunt());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    bury(&mut engine, &[bears]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, imps_taunt());
}
