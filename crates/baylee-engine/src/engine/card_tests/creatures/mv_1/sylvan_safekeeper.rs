//! `cards/creatures/mv_1/sylvan_safekeeper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sylvan Safekeeper is a `{G}` 1/1 printing one line: "Sacrifice a land:
/// Target creature you control gains shroud until end of turn." Both halves
/// need a bystander or a test says nothing about the words: a second creature
/// under the same seat and one across the table separate "target creature
/// **you** control" from "target creature", and the opponent's Forest beside
/// the Elves keeps the sacrifice menu from being read as "anything". The
/// +0/+0 pump is asserted too, because a card that bought the shroud by
/// shrinking its target would sail through a bare keyword check.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn sylvan_safekeeper_trades_a_land_for_shroud_on_one_of_your_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[sylvan_safekeeper()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {G} off the board and the 1/1 onto it. `tap_all_mana` takes every
    // source whose whole price is its own tap (#159) — three Forests and the
    // Elf — and the ability that follows pays with a land, so nothing here
    // has to stay untapped.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, sylvan_safekeeper());
    pass_until(&mut engine, stack_is_empty);

    let keeper = on_battlefield(&engine, p0, sylvan_safekeeper()).expect("the Safekeeper resolved");
    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::SHROUD),
        "nothing is shrouded before the ability is pressed"
    );

    // One activation and the two questions it asks: the target (CR 601.2c)
    // and then the land being given up (CR 601.2h). Answered in the order
    // they arrive rather than in the order they are expected.
    let mut target_menu: Vec<ObjectId> = Vec::new();
    let mut sacrifice_menu: Vec<ObjectId> = Vec::new();
    activate(&mut engine, p0, sylvan_safekeeper(), 0);
    for _ in 0..8 {
        if !target_menu.is_empty() && !sacrifice_menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat picks its own creature");
                assert_eq!((min, max), (1, 1), "exactly one creature");
                assert!(
                    player_options.is_empty(),
                    "\"target creature\" names no player: {player_options:?}"
                );
                target_menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![host],
                        },
                    )
                    .expect("the Elves were one of the creatures offered");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat pays the cost");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell \
                     the two apart"
                );
                assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
                sacrifice_menu = options;
                let doomed =
                    on_battlefield(&engine, p0, forest()).expect("a Forest is on the table");
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![doomed],
                        },
                    )
                    .expect("the Forest was on the menu the cost published");
            }
            other => {
                panic!("unexpected while the Safekeeper's ability is being paid for: {other:?}")
            }
        }
    }
    assert_eq!(
        target_menu.len(),
        2,
        "the Safekeeper and the Elves are the creatures this seat controls: {target_menu:?}"
    );
    assert!(
        target_menu.contains(&keeper) && target_menu.contains(&host),
        "both of them, so the source may shroud itself: {target_menu:?}"
    );
    assert!(
        !target_menu.contains(&theirs),
        "\"target creature *you* control\" declines the Elf across the table: {target_menu:?}"
    );
    assert_eq!(
        sacrifice_menu.len(),
        3,
        "the three Forests and nothing else this seat controls: {sacrifice_menu:?}"
    );
    assert!(
        !sacrifice_menu.contains(&host),
        "a creature is no land, however it is tapped: {sacrifice_menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, host).contains(KeywordSet::SHROUD),
        "the creature that was named gained shroud (CR 702.18a)"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the pump is +0/+0: the card buys the keyword and no body with it"
    );
    assert!(
        !keywords(&engine, keeper).contains(KeywordSet::SHROUD),
        "the ability reaches the creature it targeted and not its source"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::SHROUD),
        "nor across the table"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        2,
        "one of the three Forests was the price"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "and it is in its owner's graveyard, where a sacrificed permanent goes"
    );
    assert!(
        on_battlefield(&engine, p0, sylvan_safekeeper()).is_some(),
        "the Safekeeper paid a land and not itself"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elves were the target of the ability, not the price of it"
    );
}
