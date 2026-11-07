//! `cards/sorceries/mv_1/lab_rats.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lab_rats() -> CardIndex {
    card_index("c53d6d3e-64e3-48b9-afab-3bde3164888f")
}

/// p0 has `lands` Swamps and Lab Rats in hand; p1 has two Islands and a
/// Counterspell. p0's main phase.
fn lab_rats_over(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &vec![swamp(); lands])
        .battlefield(1, &[island(), island()])
        .hand(0, &[lab_rats()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Lab Rats (`c53d6d3e-64e3-48b9-afab-3bde3164888f`) — {B} Sorcery:
/// "Create a 1/1 black Rat creature token."
#[test]
fn lab_rats_creates_one_one_black_rat_token() {
    let mut engine = lab_rats_over(1);

    cast_from_hand(&mut engine, P0, lab_rats());
    pass_until(&mut engine, stack_is_empty);
    assert_not_bought_back(&engine, P0, lab_rats());

    let tokens = tokens_of(&engine, P0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(power_toughness(&engine, tokens[0]), (1, 1));
    let token = engine.state().object(tokens[0]).unwrap().characteristics();
    assert_eq!(
        token.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black])
    );
    assert!(
        token
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::RAT)
    );
}

/// "Buyback {4}": the {4} is paid on top, and the card comes home.
#[test]
fn lab_rats_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = lab_rats_over(5);

    cast_from_hand(&mut engine, P0, lab_rats());
    answer_buyback(&mut engine, P0, true);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{B}} and {{4}} are the five floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(tokens_of(&engine, P0).len(), 1);
    assert_bought_back(&engine, P0, lab_rats());
}

/// Declined, it costs four less and is a Lab Rats that is spent.
#[test]
fn lab_rats_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = lab_rats_over(5);

    cast_from_hand(&mut engine, P0, lab_rats());
    answer_buyback(&mut engine, P0, false);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(tokens_of(&engine, P0).len(), 1);
    assert_not_bought_back(&engine, P0, lab_rats());
}

/// Countered, it does not resolve: buyback paid, card gone, no Rat. "Create a
/// token" targets nothing, so no target can leave it (CR 608.2b).
#[test]
fn lab_rats_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = lab_rats_over(5);

    cast_from_hand(&mut engine, P0, lab_rats());
    answer_buyback(&mut engine, P0, true);
    counter_the_top_spell(&mut engine, P0, P1);

    assert!(tokens_of(&engine, P0).is_empty());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, lab_rats());
}
