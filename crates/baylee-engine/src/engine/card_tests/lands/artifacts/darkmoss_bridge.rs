//! `cards/lands/artifacts/darkmoss_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Darkmoss Bridge prints three sentences and each is the other's witness:
/// "This land enters tapped", "Indestructible", and "{T}: Add {B} or {G}".
/// The land is *played* rather than seeded — `starting_battlefield` places a
/// permanent without an entry, and a placement runs no replacement effect, so
/// a board built that way would arrive untapped whatever the card says. The
/// tapped arrival is then read twice over: off the permanent, and off the
/// offer it makes beside an untapped Forest that does hand its mana over.
/// Indestructible is a Vindicate that resolves against it and leaves it
/// standing, with the same spell pointed at that Forest one cast later so the
/// board is proved to destroy rather than assumed to. And the mana line is
/// taken on the following turn, where the choice it prints is two colours wide
/// and the pool it pays into has no other source left on the table.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn darkmoss_bridge_enters_tapped_survives_vindicate_and_taps_for_black_or_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[darkmoss_bridge()])
        .battlefield(
            1,
            &[plains(), plains(), swamp(), plains(), plains(), swamp()],
        )
        .hand(1, &[vindicate(), vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A real land drop and not a seeded permanent: only an entry runs the
    // `EnterModifier::Tapped` replacement.
    let bridge = play_land(&mut engine, p0, darkmoss_bridge());
    assert_eq!(
        on_battlefield(&engine, p0, darkmoss_bridge()),
        Some(bridge),
        "the card that was played is the permanent that arrived"
    );
    assert!(
        entered_tapped(&engine, bridge),
        "\"This land enters tapped\""
    );
    let kinds = types(&engine, bridge);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::LAND),
        "an artifact land, which is why \"target permanent\" reaches it at \
         all: {kinds:?}"
    );

    // The arrival turn, read off the offer rather than off the card. The
    // Forest is the control: the machinery that hands mana over is live on
    // this board, and the Bridge is not on it, because a land that arrived
    // tapped is tapped until its controller's next untap step. The offer was
    // computed when priority was granted, which was before the land drop, so
    // it is refreshed the way `seed_graveyard` refreshes it.
    engine.refresh_offer();
    let mine = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("playing a land uses no stack, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and passes priority to nobody");
    assert!(
        !deeds(&legal, &[mine]).is_empty(),
        "the untapped Forest beside it offers its mana: {:?}",
        legal.mana_abilities
    );
    assert!(
        deeds(&legal, &[bridge]).is_empty(),
        "and the tapped Bridge offers nothing, though the ability it prints \
         costs exactly its own {{T}} and no mana: {:?}",
        deeds(&legal, &[bridge])
    );

    // p1's turn: the destroy that cannot kill it, then the same destroy on
    // the Forest as the control.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing else")
    };
    assert!(
        options.contains(&bridge) && options.contains(&mine),
        "\"target permanent\" reaches both sides of the table: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bridge],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, darkmoss_bridge()).is_some(),
        "Indestructible (CR 702.12b): \"destroy\" resolves and the artifact \
         land is still on the table"
    );
    assert!(
        in_graveyard(&engine, p0, darkmoss_bridge()).is_none(),
        "and nowhere near a graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "the Vindicate itself resolved and was buried, so the sentence ran \
         and the keyword is what did the work"
    );

    // The control: one cast later, the same spell, on the same permanent type
    // without the keyword.
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "the Forest with no keyword is gone, so this board really does \
         destroy and the Bridge did not simply outlive a spell that aimed \
         nowhere"
    );

    // Back to p0: the untap step stands the Bridge up, and its mana line is
    // the whole of what is left on that side of the table.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, bridge),
        "the untap step ran — a land that entered tapped gives nothing on the \
         turn it arrives and everything on the next"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the tap, so whatever lands in the pool after it \
         came off this one activation"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let route = deeds(&legal, &[bridge])
        .into_iter()
        .next()
        .map(|(_, deed)| deed)
        .expect("an untapped Darkmoss Bridge offers its printed mana ability");
    engine.apply(p0, route.action(bridge)).unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped it names the colour");
    assert_eq!(
        options.len(),
        2,
        "two colours, and colorless is no colour at all (CR 105.4): {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "\"Add {{B}} or {{G}}\" offers exactly those two: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap: the Forest that could have made the green is in \
         its owner's graveyard"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
    assert!(
        is_tapped(&engine, bridge),
        "and the tap is what paid for it"
    );
}
