//! `cards/creatures/mv_1/wirewood_symbiote.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn wirewood_symbiote_trades_an_elf_for_an_untap_and_refuses_a_second_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[wirewood_symbiote()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(mine.len(), 2, "two Elves, one to pay and one to wake");
    let (returned, woken) = (mine[0], mine[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");

    // The {G} is paid and both Elves are tapped by the same helper: the one
    // that is handed back is the cost, and the one that is left down is the
    // creature the untap is for.
    cast_from_hand(&mut engine, p0, wirewood_symbiote());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let symbiote = on_battlefield(&engine, p0, wirewood_symbiote()).expect("the Symbiote resolved");
    assert!(
        is_tapped(&engine, returned) && is_tapped(&engine, woken),
        "both Elves tapped for mana on the way to paying {{G}}"
    );

    activate(&mut engine, p0, wirewood_symbiote(), 0);

    // The two questions of one activation — who is untapped (CR 601.2c) and
    // which Elf pays (CR 601.2h) — answered in the order they arrive rather
    // than in the order they are expected.
    let mut asked_target = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if asked_target && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the activating seat chooses its own target");
                assert!(
                    options.contains(&woken),
                    "the Elf still standing is a creature to untap: {options:?}"
                );
                assert!(
                    options.contains(&theirs),
                    "\"untap target creature\" says nothing about who controls \
                     it, so the Elf across the table is offered too: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![woken],
                        },
                    )
                    .expect("the creature the question offered");
                asked_target = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the seat paying the cost is the one asked");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostReturn,
                    "a cost and not a search, which is all a client has to tell \
                     the two apart"
                );
                assert_eq!((min, max), (1, 1), "one Elf, no more and no fewer");
                menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![returned],
                        },
                    )
                    .expect("the Elf the cost question offered pays the cost");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the activation resolves: {other:?}"),
        }
    }
    assert!(
        asked_target,
        "untapping \"target creature\" is a target choice"
    );
    assert_eq!(menu.len(), 2, "the two Elves this seat controls: {menu:?}");
    assert!(
        menu.contains(&returned) && menu.contains(&woken),
        "\"an Elf you control\" is both of them, tapped or not: {menu:?}"
    );
    assert!(
        !menu.contains(&theirs),
        "a seat returns only what it controls, whatever the filter says: {menu:?}"
    );
    assert!(
        !menu.contains(&symbiote),
        "the Symbiote is an Insect and no Elf: {menu:?}"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "\"return an Elf you control to its owner's hand\""
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, llanowar_elves()),
        vec![woken],
        "and only the Elf that was named left the battlefield"
    );
    assert!(
        !is_tapped(&engine, woken),
        "the Elf that was tapped for mana is standing again: \"untap target \
         creature\""
    );
    assert!(
        on_battlefield(&engine, p0, wirewood_symbiote()).is_some(),
        "the Symbiote stays where it is — the cost was somebody else"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the Elf across the table never moved"
    );

    // "Activate only once each turn." The Elf still standing is an Elf this
    // seat controls and the cost names no mana at all, so nothing but the
    // per-turn limit is withholding the line.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == symbiote),
        "\"activate only once each turn\": the ability was taken and is not \
         offered again, though the cost could still be paid: {:?}",
        legal.abilities
    );
}
