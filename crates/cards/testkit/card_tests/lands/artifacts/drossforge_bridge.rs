//! `cards/lands/artifacts/drossforge_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drossforge Bridge prints three lines and carries no basic land type: it
/// enters tapped, it is indestructible, and its `{T}` adds {B} or {R}. The
/// entry is only visible through a real land drop — a card seeded with
/// `starting_battlefield` is placed rather than made to enter — and the turn
/// cycle afterwards shows the tap came from that entry and not from a land
/// that simply never stands up. The second Vindicate is the control for the
/// first: the same spell destroys the Sol Ring on the same board, so the
/// Bridge surviving is the printed keyword and not a destroy that never ran.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn drossforge_bridge_enters_tapped_survives_vindicate_and_taps_for_black_or_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(246, forest())
        // The ordinary artifact the control Vindicate is aimed at.
        .battlefield(0, &[quiet_artifact()])
        .hand(0, &[drossforge_bridge()])
        .battlefield(
            1,
            &[plains(), plains(), plains(), swamp(), swamp(), swamp()],
        )
        .hand(1, &[vindicate(), vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let bridge = play_land(&mut engine, p0, drossforge_bridge());
    assert!(
        entered_tapped(&engine, bridge),
        "\"This land enters tapped\" — the drop, not a placement, is what runs it"
    );
    let kinds = types(&engine, bridge);
    assert!(
        kinds.contains(TypeSet::LAND) && kinds.contains(TypeSet::ARTIFACT),
        "an artifact land, which is why an ordinary destroy spell can aim at it: {kinds:?}"
    );
    assert!(
        keywords(&engine, bridge).contains(KeywordSet::INDESTRUCTIBLE),
        "the printed keyword reaches the permanent"
    );

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert!(
        options.contains(&bridge),
        "\"destroy target permanent\" reaches the artifact land: {options:?}"
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
        on_battlefield(&engine, p0, drossforge_bridge()).is_some(),
        "CR 702.12b: a destroy effect aimed at an indestructible permanent leaves it standing"
    );
    assert!(
        in_graveyard(&engine, p0, drossforge_bridge()).is_none(),
        "and it is in the graveyard on neither reading"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "the spell resolved into its owner's graveyard, so it really was cast"
    );

    reach_their_main_phase(&mut engine, p0);
    assert!(
        !entered_tapped(&engine, bridge),
        "the untap step ran: the tap above was the entry, not a land that never comes back up"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "with an empty stack in p0's own main phase");
    assert!(
        legal.abilities.contains(&(bridge, 0)),
        "the printed `{{T}}: Add {{B}} or {{R}}` is an ordinary indexed ability: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&bridge),
        "and never the CR 305.6 shortcut, which names a basic land type this land does not have"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bridge,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"or\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that taps the land names the colour");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "\"Add {{B}} or {{R}}\": {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "both colours and nothing else — an artifact land makes no colourless: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, bridge), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );

    // The control: a second Vindicate, off a fresh set of untapped lands,
    // aimed at the artifact beside the Bridge that is *not* indestructible.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is still out");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert!(
        options.contains(&rock) && options.contains(&bridge),
        "the same \"target permanent\" offers both artifacts: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none()
            && in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "this Vindicate does destroy what it names, so the Bridge surviving the other one \
         was its own keyword"
    );
    assert!(
        on_battlefield(&engine, p0, drossforge_bridge()).is_some(),
        "and the Bridge stands through both"
    );
}
