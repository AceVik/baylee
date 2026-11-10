//! `cards/lands/artifacts/silverbluff_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Silverbluff Bridge is an artifact land: it enters tapped, it carries
/// indestructible, and it taps for {U} or {R}. The land arrives as a real
/// `PlayLand` — only a real entry meets the replacement effect that taps it,
/// where a `starting_battlefield` placement sits there untapped whatever the
/// card says — and then the same Vindicate is aimed twice off one board: the
/// Bridge shrugs the destroy off while the Sol Ring beside it does not, so
/// "still on the battlefield" cannot be a spell that never resolved. The mana
/// line is read a turn later, because a land that entered tapped offers no
/// `{T}` at all until its controller's untap step, and the question it asks
/// is exactly the two colours it prints.
#[test]
#[allow(clippy::too_many_lines)] // three printed sentences, and each needs its own board
fn silverbluff_bridge_enters_tapped_survives_destruction_and_taps_for_blue_or_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[quiet_artifact()])
        .hand(0, &[silverbluff_bridge()])
        .battlefield(
            1,
            &[plains(), plains(), plains(), swamp(), swamp(), swamp()],
        )
        .hand(1, &[vindicate(), vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `PlayLand`, not a setup placement: the entry is what the printed
    // sentence modifies, and a board seeded by the harness never enters.
    let bridge = play_land(&mut engine, p0, silverbluff_bridge());
    assert!(is_tapped(&engine, bridge), "\"This land enters tapped\"");
    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. }
                if legal.abilities.iter().any(|(src, _)| *src == bridge)
        )
    };
    assert!(
        !offered(&engine),
        "and a tapped land can pay no `{{T}}`: the one line the card prints \
         is not offered until its controller's untap step"
    );

    // The other side of the table gets two Vindicates — one for the Bridge
    // and one for the Sol Ring standing beside it.
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert_eq!(player, p1, "the seat that cast the spell names the target");
    assert!(
        options.contains(&bridge),
        "\"destroy target permanent\" reaches an artifact land: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bridge],
            },
        )
        .expect("the Bridge was one of the targets it published");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, silverbluff_bridge()).is_some(),
        "indestructible (CR 702.12b): a destroy effect aimed at the Bridge \
         leaves it exactly where it stands"
    );
    assert!(
        in_graveyard(&engine, p0, silverbluff_bridge()).is_none(),
        "and it is in no graveyard to be brought back from"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "the spell did resolve: a Vindicate still on the stack would satisfy \
         the survival above for the wrong reason"
    );

    // The control — the same spell, in the same turn, at a permanent that
    // prints no such keyword.
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring stands");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert!(
        options.contains(&ring),
        "and an ordinary artifact is a target too: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the Sol Ring was one of the targets it published");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "the Sol Ring, which prints no indestructible, is destroyed"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "and the destroy works: it lies in its owner's graveyard"
    );

    // A turn further, because the turn the land arrived in is a turn it
    // spends tapped.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, bridge),
        "the untap step has run, so the Bridge's `{{T}}` is payable at last"
    );
    assert!(
        offered(&engine),
        "and the one line the card prints is offered again"
    );

    activate(&mut engine, p0, silverbluff_bridge(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{U}} or {{R}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "the two colours the card prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "\"or\" names exactly {{U}} and {{R}}, and nothing beside them: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana off one tap, and nothing else");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing waits on one"
    );
    assert!(is_tapped(&engine, bridge), "the Bridge paid its own {{T}}");
}
