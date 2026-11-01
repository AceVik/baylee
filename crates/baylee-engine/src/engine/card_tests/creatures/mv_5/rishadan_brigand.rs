//! `cards/creatures/mv_5/rishadan_brigand.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "ae2db472-c699-4c39-aa35-c359ac42df4e"
fn rishadan_brigand() -> CardIndex {
    card_index("ae2db472-c699-4c39-aa35-c359ac42df4e")
}

/// `Rishadan Brigand` (`Coverage::Generated`):
/// "Flying. When this creature enters, each opponent sacrifices a permanent
/// unless they pay {3}. This creature can block only creatures with flying."
///
/// Verifies both branches of `Rishadan Brigand`'s enters-the-battlefield trigger
/// (sacrificing when declining to pay `{3}`, keeping permanents when paying `{3}`)
/// as well as its flying and `CanBlockOnlyFlying` static restriction.
#[test]
fn rishadan_brigand_taxes_opponents_three_on_etb_and_blocks_only_flyers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Case 1: Opponent declines to pay {3} -> sacrifices their permanent.
    let mut engine = Duel::new(9516, island())
        .battlefield(0, &[island(); 5])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[rishadan_brigand()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Grizzly Bears");
    cast_from_hand(&mut engine, p0, rishadan_brigand());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 3 },
                ..
            } if *player == p1
        )
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "P1 sacrificed Grizzly Bears when declining to pay {{3}}"
    );

    // Case 2: Opponent pays {3} -> keeps their permanent, then attacks with a ground creature
    // that Rishadan Brigand cannot block.
    let mut engine = Duel::new(9517, island())
        .battlefield(0, &[island(); 5])
        .battlefield(1, &[island(), island(), island(), grizzly_bears()])
        .hand(0, &[rishadan_brigand()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, rishadan_brigand());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 3 },
                ..
            } if *player == p1
        )
    });
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let brigand = on_battlefield(&engine, p0, rishadan_brigand()).expect("P0 Rishadan Brigand");
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 kept Grizzly Bears");
    assert!(keywords(&engine, brigand).contains(KeywordSet::FLYING));

    assert!(walk_to_own_main(&mut engine, p1));
    let blockers = attack_and_collect_blocks(&mut engine, bear, p0);
    assert!(!blockers.iter().any(|option| option.blocker == brigand));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(brigand, bear)],
                }
            )
            .is_err(),
        "flying does not let the Brigand block a nonflying attacker"
    );
    assert!(walk_to_own_main(&mut engine, p0));
    assert_eq!(
        engine.state().players[0].life,
        18,
        "Rishadan Brigand cannot block non-flying Grizzly Bears"
    );
}
