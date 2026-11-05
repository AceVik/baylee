//! `cards/instants/mv_1/whim_of_volrath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn whim() -> CardIndex {
    card_index("51a20c9f-68d2-471b-8622-67d481b53b92")
}

/// p0 holds Whim of Volrath and two Giant Growths over an Island and `forests`
/// Forests, beside a Black Knight (protection from white) and a Grizzly
/// Bears; p1 has two Islands and a Counterspell. p0's main phase.
fn a_knight_and_a_whim(forests: usize) -> Engine<RegistryLookup> {
    let mut mine = vec![island(), index::BLACK_KNIGHT, index::GRIZZLY_BEARS];
    mine.extend(vec![forest(); forests]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(1, &[island(), island()])
        .hand(0, &[whim(), giant_growth(), giant_growth()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Answers the card's own questions: colour words, then white to green, in
/// the word order `TextReplacement::from_choice` lists them.
fn white_becomes_green(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(P0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(engine, stack_is_empty);
}

/// The targets a Giant Growth is offered right now, and it is cast at the
/// Bears so the table moves on.
fn growth_can_reach(engine: &mut Engine<RegistryLookup>, bears: ObjectId) -> Vec<ObjectId> {
    let spell = in_hand(engine, P0, giant_growth()).unwrap();
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    let options = aim_at(engine, P0, bears);
    pass_until(engine, stack_is_empty);
    options
}

/// "Replacing all instances of one color word with another … until end of
/// turn": the Knight has protection from green this turn and from white
/// again the next. Held back while the change never ended.
#[test]
fn whim_of_volrath_changes_a_word_until_end_of_turn() {
    let mut engine = a_knight_and_a_whim(2);
    let knight = on_battlefield(&engine, P0, index::BLACK_KNIGHT).unwrap();
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, whim());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, knight);
    white_becomes_green(&mut engine);
    assert!(
        !growth_can_reach(&mut engine, bears).contains(&knight),
        "protection from green turns the Growth away"
    );

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    assert!(
        growth_can_reach(&mut engine, bears).contains(&knight),
        "the word is white again"
    );
}

/// "Buyback {2}": the {2} is paid on top, and the card comes home.
#[test]
fn whim_of_volrath_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = a_knight_and_a_whim(2);
    let knight = on_battlefield(&engine, P0, index::BLACK_KNIGHT).unwrap();

    cast_from_hand(&mut engine, P0, whim());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, knight);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    white_becomes_green(&mut engine);

    assert_bought_back(&engine, P0, whim());
}

/// Declined, it is spent.
#[test]
fn whim_of_volrath_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = a_knight_and_a_whim(2);
    let knight = on_battlefield(&engine, P0, index::BLACK_KNIGHT).unwrap();

    cast_from_hand(&mut engine, P0, whim());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, knight);
    white_becomes_green(&mut engine);

    assert_not_bought_back(&engine, P0, whim());
}

/// Countered, it does not resolve: buyback paid, card gone, word unchanged.
#[test]
fn whim_of_volrath_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = a_knight_and_a_whim(2);
    let knight = on_battlefield(&engine, P0, index::BLACK_KNIGHT).unwrap();

    cast_from_hand(&mut engine, P0, whim());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, knight);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_not_bought_back(&engine, P0, whim());
    assert!(engine.state().text_changes.is_empty());
}

/// With its one target gone it does not resolve either.
#[test]
fn whim_of_volrath_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = a_knight_and_a_whim(2);
    let knight = on_battlefield(&engine, P0, index::BLACK_KNIGHT).unwrap();

    cast_from_hand(&mut engine, P0, whim());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, knight);
    bury(&mut engine, &[knight]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, whim());
}

/// "…or one basic land type with another until end of turn": the card's other
/// half. A Forest is an Island for the turn (it is blue's land, and taps for
/// blue) and a Forest again at the next, and the Forest beside it is
/// untouched. The colour-word test above never plays this sentence.
#[test]
fn whim_of_volrath_changes_a_basic_land_type_until_end_of_turn() {
    use crate::text_changes::TextReplacement;
    use baylee_cards_dsl::TextWordKind;
    use baylee_core::generated::subtypes::land;

    let mut engine = a_knight_and_a_whim(2);
    let forests = all_on_battlefield(&engine, P0, forest());
    let (changed, other) = (forests[0], forests[1]);
    let choice = (0..20)
        .find(|n| {
            TextReplacement::from_choice(TextWordKind::BasicLandType, *n)
                .is_some_and(|r| r.from == 4 && r.to == 1)
        })
        .expect("Forest to Island is offered");

    cast_from_hand(&mut engine, P0, whim());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, changed);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(P0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseNumber(choice))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let subtypes = |engine: &Engine<RegistryLookup>, id| {
        engine
            .state()
            .object(id)
            .unwrap()
            .characteristics()
            .subtypes
    };
    assert!(subtypes(&engine, changed).contains(land::ISLAND));
    assert!(!subtypes(&engine, changed).contains(land::FOREST));
    assert!(subtypes(&engine, other).contains(land::FOREST));
    assert_not_bought_back(&engine, P0, whim());

    reach_their_main_phase(&mut engine, P1);
    assert!(subtypes(&engine, changed).contains(land::FOREST));
    assert!(!subtypes(&engine, changed).contains(land::ISLAND));
}
