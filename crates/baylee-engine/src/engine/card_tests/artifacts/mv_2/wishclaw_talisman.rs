//! `cards/artifacts/mv_2/wishclaw_talisman.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wishclaw Talisman — "{1}, {T}, Remove a wish counter from this artifact:
/// Search your library for a card, put it into your hand, then shuffle. An
/// opponent gains control of this artifact."
///
/// Three printed clauses and the card is only itself when all three happen at
/// once, which is why they are asserted in one activation rather than three
/// tests: a tutor that keeps its counter is a different card, and a tutor
/// that keeps its *controller* is a much better one. The handover is the half
/// that pays for the rest of the sentence, so it is checked against the
/// object's controller and not against the board, because the artifact never
/// moves — it changes hands where it stands.
///
/// The file is `Coverage::Partial` for "Activate only during your turn",
/// which no `ActivationTiming` and no `Condition` can say. That gap is
/// invisible from this scenario by construction: everything here happens in
/// p0's own main phase, which is the turn the printing allows.
#[test]
fn wishclaw_talisman_spends_a_wish_counter_and_hands_itself_to_an_opponent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[wishclaw_talisman()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Three Swamps: two cast the artifact and the third is exactly the {1}
    // the ability charges, so the activation is a real payment out of the
    // pool rather than a label on a free ability.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, wishclaw_talisman());
    pass_until(&mut engine, stack_is_empty);
    let talisman = on_battlefield(&engine, p0, wishclaw_talisman()).expect("it resolved");
    assert_eq!(
        counters_on(&engine, talisman, baylee_cards_dsl::counters::WISH),
        3,
        "\"enters with three wish counters on it\" — the number is printed"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "two of the three Swamps are spent and the {{1}} is still floating"
    );

    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, wishclaw_talisman(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that activated searches");
    assert!(
        !options.is_empty(),
        "\"a card\" is every card in the library"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the card the search offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, talisman, baylee_cards_dsl::counters::WISH),
        2,
        "one wish counter was the cost — three would mean the removal never \
         happened and the card could be activated for ever"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the tutored card left the library"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "and it went to the hand of the player who searched, not to the one \
         about to own the artifact"
    );
    assert_eq!(
        engine
            .state()
            .object(talisman)
            .expect("the artifact is still on the battlefield")
            .controller,
        p1,
        "\"An opponent gains control of this artifact\" — it does not move \
         zones, it changes hands where it stands"
    );
    assert!(
        engine.state().players[0].mana_pool.total() == 0,
        "and the {{1}} it charges was paid"
    );
}

#[test]
fn wishclaw_is_usable_during_its_controllers_turn_even_with_a_nonempty_stack() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::WISHCLAW_TALISMAN])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, index::WISHCLAW_TALISMAN).unwrap();
    // Battlefield setup predates entry replacement; seed the printed counters.
    let state = e.dev_state_mut(p0).unwrap();
    state
        .object_mut(source)
        .unwrap()
        .counters
        .set(baylee_cards_dsl::counters::WISH, 3);
    state.players[0].mana_pool.add(ManaColor::Red, 2);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, index::LIGHTNING_BOLT);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    activate(&mut e, p0, index::WISHCLAW_TALISMAN, 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending() else {
        panic!("search")
    };
    let chosen = options[0];
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![chosen],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(source).unwrap().controller, p1);
    // It is still p0's turn: p1 cannot use the newly acquired artifact.
    e.dev_state_mut(p0).unwrap().set_tapped(source, false);
    e.dev_state_mut(p0).unwrap().players[1]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        e.apply(
            p1,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0
            }
        )
        .is_err()
    );
}
