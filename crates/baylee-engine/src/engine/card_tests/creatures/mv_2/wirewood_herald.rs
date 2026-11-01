//! `cards/creatures/mv_2/wirewood_herald.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Wirewood Herald (`7a838374-fc40-4fb6-9d38-a96f01e7038a`) — {1}{G} Creature — Elf 1/1:
/// "When this creature dies, you may search your library for an Elf card, reveal that card, put it into your hand, then shuffle."
#[test]
fn wirewood_herald_searches_library_for_elf_card_when_it_dies() {
    let herald_card = card_index("7a838374-fc40-4fb6-9d38-a96f01e7038a");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[herald_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let herald = on_battlefield(&engine, p0, herald_card).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    bury(&mut engine, &[herald]);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected SearchLibrary prompt");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    assert!(in_hand(&engine, p0, llanowar_elves()).is_some());
}
