//! `cards/enchantments/mv_2/ancestral_knowledge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ancestral_knowledge() -> CardIndex {
    card_index("2786f210-ec64-46ea-9a7f-b03759fb1dae")
}

/// `Ancestral Knowledge` ({1}{U}, Enchantment, `Coverage::Partial`) reorders
/// the top ten cards of your library when it enters the battlefield.
#[test]
fn ancestral_knowledge_reorders_top_library_on_etb_and_shuffles_on_leave() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[ancestral_knowledge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, ancestral_knowledge());
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));

    assert!(on_battlefield(&engine, p0, ancestral_knowledge()).is_some());
    assert_eq!(library_size(&engine, p0), before);
}

/// Cumulative upkeep {1} and the leave-the-battlefield shuffle, played: the
/// first upkeep puts one age counter on and asks {1}, which an Island pays;
/// the second asks {2} and, declined, the enchantment is sacrificed, which
/// shuffles the controller's library (and only then).
///
/// The upkeep question and the age-counter reader are the Remora's
/// (`enchantments.rs`): the same cumulative-upkeep trigger, the same `PayTax`.
#[test]
fn ancestral_knowledge_cumulative_upkeep_grows_and_its_sacrifice_shuffles() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[ancestral_knowledge(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    let knowledge = on_battlefield(&engine, p0, ancestral_knowledge()).expect("it is seated");
    let islands = all_on_battlefield(&engine, p0, island());

    let mana = remora_upkeep_question(&mut engine, p0);
    assert_eq!(
        age_counters(&engine, knowledge),
        1,
        "the counter goes on first"
    );
    assert_eq!(mana, 1, "{{1}} for one age counter");
    let since = engine.state().journal.last_seq();
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == islands[0]);
    if engine.payment_window().is_some() {
        engine.apply(p0, PlayerAction::PassPriority).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, ancestral_knowledge()).is_some(),
        "paid, it stays"
    );
    let shuffles = |e: &Engine<RegistryLookup>, since: u64| {
        e.state()
            .journal
            .entries()
            .iter()
            .skip(since as usize)
            .filter(|entry| {
                matches!(entry.event, crate::event::GameEvent::Shuffled { player, .. } if player == p0)
            })
            .count()
    };
    assert_eq!(
        shuffles(&engine, since),
        0,
        "staying in play shuffles nothing"
    );

    let mana = remora_upkeep_question(&mut engine, p0);
    assert_eq!(age_counters(&engine, knowledge), 2, "a second counter");
    assert_eq!(mana, 2, "{{1}} for each of the two");
    let since = engine.state().journal.last_seq();
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, ancestral_knowledge()).is_some(),
        "unpaid, it is sacrificed"
    );
    assert_eq!(
        shuffles(&engine, since),
        1,
        "and leaving the battlefield shuffles the library once"
    );
}
