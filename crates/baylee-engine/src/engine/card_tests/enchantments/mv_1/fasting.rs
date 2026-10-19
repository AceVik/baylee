//! `cards/enchantments/mv_1/fasting.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn fasting_card() -> CardIndex {
    card_index("0d787c6b-ab82-42c5-b840-8f3a09af30ac")
}

fn hunger(e: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    e.state()
        .object(id)
        .expect("Fasting is out")
        .counters
        .get(baylee_cards_dsl::counters::HUNGER)
}

/// Walks to the next upkeep of seat 0 and lets its triggers resolve.
fn to_my_upkeep(e: &mut Engine<RegistryLookup>) {
    let p0 = PlayerId::new(0);
    pass_until(e, |e| {
        e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    pass_until(e, stack_is_empty);
}

/// Walks to the draw-step question of seat 0 and answers it.
fn answer_the_draw_skip(e: &mut Engine<RegistryLookup>, skip: bool) {
    let p0 = PlayerId::new(0);
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    e.apply(p0, PlayerAction::YesNo(skip))
        .expect("the skip question takes a yes or a no");
}

/// Fasting (0d787c6b-ab82-42c5-b840-8f3a09af30ac) — "At the beginning of your
/// upkeep, put a hunger counter on this enchantment. Then destroy this
/// enchantment if it has five or more hunger counters on it."
#[test]
fn fasting_adds_a_hunger_counter_each_upkeep_and_dies_at_the_fifth() {
    let p0 = PlayerId::new(0);
    let mut e = table(SEED, &[], &[fasting_card()], &[]);
    let fasting = on_battlefield(&e, p0, fasting_card()).expect("Fasting is out");
    assert_eq!(hunger(&e, fasting), 1, "turn one's upkeep put the first");

    for expected in 2..=4u16 {
        to_my_upkeep(&mut e);
        assert_eq!(hunger(&e, fasting), expected);
        assert!(
            on_battlefield(&e, p0, fasting_card()).is_some(),
            "fewer than five counters: it stays"
        );
        // Skip the draw (and gain 2) so the enchantment is not destroyed by a draw.
        answer_the_draw_skip(&mut e, true);
    }

    to_my_upkeep(&mut e);
    assert!(
        on_battlefield(&e, p0, fasting_card()).is_none(),
        "the fifth hunger counter destroys it"
    );
    assert_eq!(
        e.state().object(fasting).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
}

/// Fasting — "If you would begin your draw step, you may skip that step
/// instead. If you do, you gain 2 life."
#[test]
fn fasting_lets_you_skip_the_draw_step_to_gain_two_life() {
    let p0 = PlayerId::new(0);
    let mut e = table(SEED, &[], &[fasting_card()], &[]);
    to_my_upkeep(&mut e);
    let hand_before = hand_size(&e, p0);
    let library_before = library_size(&e, p0);
    let life_before = life(&e, p0);

    answer_the_draw_skip(&mut e, true);
    assert_eq!(life(&e, p0), life_before + 2, "skipping gains 2 life");
    assert_eq!(hand_size(&e, p0), hand_before, "no card was drawn");
    assert_eq!(library_size(&e, p0), library_before);
    assert!(
        on_battlefield(&e, p0, fasting_card()).is_some(),
        "no draw happened, so Fasting stays"
    );
}

/// Fasting — "When you draw a card, destroy this enchantment." Declining to
/// skip means drawing, which destroys it; the opponent's draw does not.
#[test]
fn fasting_is_destroyed_when_you_draw_but_not_when_the_opponent_does() {
    let p0 = PlayerId::new(0);
    let mut e = table(SEED, &[], &[fasting_card()], &[]);

    // Turn two is the opponent's: they draw, Fasting stays.
    let p1 = PlayerId::new(1);
    pass_until(&mut e, |e| {
        e.state().turn.active == p1 && e.state().turn.step == crate::turn::Step::Draw
    });
    pass_until(&mut e, |e| e.state().turn.active == p0);
    assert!(
        on_battlefield(&e, p0, fasting_card()).is_some(),
        "an opponent's draw does not destroy it"
    );

    to_my_upkeep(&mut e);
    let hand_before = hand_size(&e, p0);
    answer_the_draw_skip(&mut e, false);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(hand_size(&e, p0), hand_before + 1, "the card was drawn");
    assert!(
        on_battlefield(&e, p0, fasting_card()).is_none(),
        "drawing a card destroys Fasting"
    );
}
