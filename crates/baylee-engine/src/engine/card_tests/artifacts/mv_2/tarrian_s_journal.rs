//! `cards/artifacts/mv_2/tarrian_s_journal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Tarrian's Journal` // `The Tomb of Aclazotz` (`Coverage::Partial`): "{T}, Sacrifice another
/// artifact or creature: Draw a card. Activate only as a sorcery. {2}, {T}, Discard your hand:
/// Transform Tarrian's Journal. // {T}: Add {B}. {T}: You may cast a creature spell from your
/// graveyard this turn. If you do, it enters with a finality counter on it and is a Vampire in
/// addition to its other types."
///
/// Under `Coverage::Partial`, the front-face sorcery-speed activated ability to tap and sacrifice
/// another artifact or creature to draw a card is implemented. The test activates ability 0,
/// confirms that `ChoicePrompt::CostSacrifice` prompts for the sacrifice, verifies that `Tarrian's Journal`
/// cannot sacrifice itself via `Filter::Another`, sacrifices a controlled creature, and confirms
/// that the creature is buried while a card is drawn.
#[test]
fn tarrians_journal_taps_and_sacrifices_another_creature_to_draw_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(126, swamp())
        .battlefield(0, &[swamp(), tarrian_s_journal(), quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, quiet_creature()).expect("p0 controls an elf");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("p1 controls an elf");
    let journal = on_battlefield(&engine, p0, tarrian_s_journal()).expect("journal on battlefield");
    let lib_before = library_size(&engine, p0);

    assert!(!is_tapped(&engine, journal), "journal starts untapped");

    // Ability 0 requires only {T} and sacrificing another artifact or creature; no mana is spent.
    activate(&mut engine, p0, tarrian_s_journal(), 0);

    let Pending::ChooseCards {
        options,
        prompt,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice prompt, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the cost prompt is ChoicePrompt::CostSacrifice"
    );
    assert_eq!((min, max), (1, 1), "costs exactly one sacrifice");
    assert!(
        options.contains(&my_elf),
        "controlled creature is legal sacrifice fodder"
    );
    assert!(
        !options.contains(&journal),
        "the journal cannot sacrifice itself due to Filter::Another"
    );
    assert!(
        !options.contains(&their_elf),
        "opponent's creature cannot be sacrificed by p0"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_elf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, journal),
        "journal tapped to pay its activation cost"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_some(),
        "the sacrificed creature is in the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "a card was drawn from the library"
    );
}
