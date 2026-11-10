//! `cards/enchantments/mv_4/snake_pit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn snake_pit() -> CardIndex {
    card_index("115bd33d-4875-4014-ae9f-0052f5acdbb2")
}

fn pearl_trident_merfolk() -> CardIndex {
    card_index("218d9277-c179-4de3-9c7f-79b5a6d4fa38")
}

/// A Snake Pit on seat 0 and a blue or black spell cast by seat 1, walked
/// to the question the trigger asks the Pit's controller.
fn a_pit_asking(spell: CardIndex, color: ManaColor) -> Engine<RegistryLookup> {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[snake_pit()])
        .hand(1, &[spell])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    float(&mut engine, p1, color, 3);
    cast_with_floating(&mut engine, p1, spell);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
}

/// "Whenever an opponent casts a blue or black spell, you may create a 1/1
/// green Snake creature token": asked of the Pit's controller, accepted for
/// a black spell, the Snake is mine and a 1/1.
#[test]
fn snake_pit_offers_a_snake_for_an_opponents_black_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = a_pit_asking(dark_ritual(), ManaColor::Black);
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, p0, "the Pit's controller decides");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let snakes = tokens_of(&engine, p0);
    assert_eq!(snakes.len(), 1, "one Snake");
    assert_eq!(pt(&engine, snakes[0]), (1, 1));
}

/// Blue counts too, and "may" means a no creates nothing.
#[test]
fn snake_pit_declined_creates_nothing_for_a_blue_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = a_pit_asking(pearl_trident_merfolk(), ManaColor::Blue);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(tokens_of(&engine, p0).is_empty(), "declined: no Snake");
}

/// A green spell of the opponent's asks nothing.
#[test]
fn snake_pit_ignores_an_opponents_green_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[snake_pit()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    float(&mut engine, p1, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert!(tokens_of(&engine, p0).is_empty(), "no trigger, no question");
}
