//! `cards/artifacts/mv_4/etherwrought_page.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn etherwrought_page() -> CardIndex {
    card_index("f1c87913-9374-41d7-ae70-444d427290b4")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

fn choose_page_mode(engine: &mut Engine<RegistryLookup>, p0: PlayerId, mode: usize) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let idx = choose_cast_kind(engine, CastModeKind::Mode(mode));
    engine
        .apply(p0, PlayerAction::ChooseMode(idx))
        .expect("choose Etherwrought Page mode");
}

/// Etherwrought Page (f1c87913-9374-41d7-ae70-444d427290b4) — {1}{W}{U}{B} Artifact:
/// "At the beginning of your upkeep, choose one —
///  • You gain 2 life.
///  • Surveil 1.
///  • Each opponent loses 1 life."
///
/// Exercises mode 0 (gain 2 life), confirms opponent's upkeep does NOT trigger
/// it, then exercises mode 2 (opponent loses 1 life).
#[test]
fn etherwrought_page_upkeep_life_gain_life_loss_and_your_upkeep_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[etherwrought_page()])
        .start();
    keep_mulligans(&mut engine);

    // Turn 1 (p0 upkeep): choose mode 0 -> gain 2 life.
    choose_page_mode(&mut engine, p0, 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 22, "mode 0 gains 2 life");
    assert_eq!(life(&engine, p1), 20);

    // Advance across p1's entire turn without any modal trigger firing on p1's upkeep.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(life(&engine, p0), 22);
    assert_eq!(life(&engine, p1), 20);

    // Next turn (p0 upkeep): choose mode 2 -> each opponent loses 1 life.
    choose_page_mode(&mut engine, p0, 2);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 22);
    assert_eq!(life(&engine, p1), 19, "mode 2 makes opponent lose 1 life");
}

/// Mode 1 (Surveil 1): looks at the top card and lets you put it into your
/// graveyard OR leave it on top of your library.
#[test]
fn etherwrought_page_surveil_mode_bins_or_keeps_top_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[etherwrought_page()])
        .hand(0, &[grizzly_bears(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    hand_to_library_top(&mut engine, p0, grizzly_bears());
    hand_to_library_top(&mut engine, p0, lightning_bolt());

    // Turn 1 upkeep: Surveil 1 and put Lightning Bolt into graveyard.
    choose_page_mode(&mut engine, p0, 1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Surveil,
                ..
            }
        )
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(cards.len(), 1);
    let binned = cards[0];
    engine
        .apply(p0, look_answer(&cards, &[binned]))
        .expect("surveil top card to graveyard");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_some(),
        "surveiled Lightning Bolt moved to graveyard"
    );

    // Next p0 upkeep: Surveil 1 and keep Grizzly Bears on top (then draw it!).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let idx = choose_cast_kind(&engine, CastModeKind::Mode(1));
    engine.apply(p0, PlayerAction::ChooseMode(idx)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Surveil,
                ..
            }
        )
    });
    let Pending::Arrange { cards: cards2, .. } = engine.pending().clone() else {
        unreachable!();
    };
    engine
        .apply(p0, look_answer(&cards2, &[]))
        .expect("keep top card on library");
    reach_main_phase(&mut engine, p0);
    assert!(
        in_hand(&engine, p0, grizzly_bears()).is_some(),
        "kept Grizzly Bears on top and drew it on draw step"
    );
}
