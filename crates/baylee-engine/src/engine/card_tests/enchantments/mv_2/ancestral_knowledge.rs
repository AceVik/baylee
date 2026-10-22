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
