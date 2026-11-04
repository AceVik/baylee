//! `cards/instants/mv_4/slaughter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn slaughter() -> CardIndex {
    card_index("37de575d-15d4-42a2-a3d2-3855fb6ab110")
}

/// p0 holds Slaughter over four Swamps at `life`; p1 has Grizzly Bears, two
/// Islands and a Counterspell. p0's main phase.
fn a_slaughter_at(life: i32) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 4])
        .battlefield(1, &[index::GRIZZLY_BEARS, island(), island()])
        .hand(0, &[slaughter()])
        .hand(1, &[counterspell()])
        .life(0, life)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

#[test]
fn slaughter_destroys_nonblack_creatures_despite_regeneration_and_excludes_black_targets() {
    let mut e = lk::game(
        SEED,
        &[],
        &[index::PYGMY_TROLL, index::DAUTHI_CUTTHROAT],
        &[slaughter()],
        &[],
    );
    let troll = lk::obj(&e, P1, index::PYGMY_TROLL);
    let black = lk::obj(&e, P1, index::DAUTHI_CUTTHROAT);
    lk::float(&mut e, P0, &[(ManaColor::Black, 4)]);
    lk::pass_once(&mut e);
    lk::float(&mut e, P1, &[(ManaColor::Green, 1)]);
    kit::activate_object(&mut e, P1, troll, 1);
    lk::settle(&mut e);
    cast_with_floating(&mut e, P0, slaughter());
    answer_buyback(&mut e, P0, false);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("targets")
    };
    assert!(options.contains(&troll));
    assert!(!options.contains(&black));
    lk::aim(&mut e, &[troll], &[]);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, P1, index::PYGMY_TROLL).is_some());
    assert!(on_battlefield(&e, P1, index::DAUTHI_CUTTHROAT).is_some());
    assert_not_bought_back(&e, P0, slaughter());
}

/// "Buyback—Pay 4 life": the life goes as the spell is cast, before it
/// resolves, and the card comes home.
#[test]
fn slaughter_with_four_life_paid_comes_back_to_hand() {
    let mut engine = a_slaughter_at(20);
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, slaughter());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    assert!(on_stack(&engine, slaughter()).is_some());
    assert_eq!(engine.state().players[0].life, 16);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[0].life, 16);
    assert_bought_back(&engine, P0, slaughter());
}

/// Declined, no life is paid and it is a Slaughter that is spent.
#[test]
fn slaughter_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_slaughter_at(20);
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, slaughter());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[0].life, 20);
    assert_not_bought_back(&engine, P0, slaughter());
}

/// Countered, it does not resolve: the life stays paid, the card is gone and
/// the creature lives.
#[test]
fn slaughter_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_slaughter_at(20);
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, slaughter());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[0].life, 16);
    assert_not_bought_back(&engine, P0, slaughter());
}

/// With its one target gone it does not resolve either, and the life stays
/// paid.
#[test]
fn slaughter_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = a_slaughter_at(20);
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, slaughter());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    bury(&mut engine, &[bears]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 16);
    assert_not_bought_back(&engine, P0, slaughter());
}

/// A player pays life only out of a total that covers it (CR 119.4): at 3
/// life nothing is asked.
#[test]
fn slaughter_at_three_life_is_not_asked() {
    let mut engine = a_slaughter_at(3);

    cast_from_hand(&mut engine, P0, slaughter());

    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "got {:?}",
        engine.pending()
    );
}

/// At exactly 4 life the cost can be paid, and paying it loses the game
/// (CR 704.5a) before the spell can resolve.
#[test]
fn slaughter_paid_with_the_last_four_life_loses_the_game() {
    let mut engine = a_slaughter_at(4);
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, slaughter());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);

    assert_eq!(engine.state().players[0].life, 0);
    let Pending::GameOver(result) = engine.pending().clone() else {
        panic!("expected the end of the game, got {:?}", engine.pending())
    };
    assert_eq!(result.winner, Some(crate::win::Victor::Player(P1)));
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
