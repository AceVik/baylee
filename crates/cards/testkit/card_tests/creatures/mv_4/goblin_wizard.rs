//! `cards/creatures/mv_4/goblin_wizard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn goblin_wizard() -> CardIndex {
    card_index("912c99a0-ee97-4bb9-8ab7-2e469d74fa03")
}

fn goblin_raiders() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn float(engine: &mut Engine<RegistryLookup>, seat: usize, mana: &[(ManaColor, u32)]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for (color, n) in mana {
        state.players[seat].mana_pool.add(*color, *n);
    }
    engine.refresh_offer();
}

fn board(hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_wizard()])
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, PlayerId::new(0)),
        "p0 reaches its own main"
    );
    engine
}

/// Goblin Wizard (912c99a0-ee97-4bb9-8ab7-2e469d74fa03) — "{T}: You may put a
/// Goblin permanent card from your hand onto the battlefield."
#[test]
fn goblin_wizard_puts_a_goblin_from_hand_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = board(&[goblin_raiders(), grizzly_bears()]);
    let wizard = on_battlefield(&engine, p0, goblin_wizard()).expect("the Wizard is out");
    let raiders = in_hand(&engine, p0, goblin_raiders()).expect("the Raiders are in hand");

    activate(&mut engine, p0, goblin_wizard(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(options, vec![raiders], "only the Goblin is on the menu");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![raiders],
            },
        )
        .expect("the Raiders are on the menu");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, goblin_raiders()).is_some(),
        "the Goblin is on the battlefield"
    );
    assert!(in_hand(&engine, p0, goblin_raiders()).is_none());
    assert!(
        in_hand(&engine, p0, grizzly_bears()).is_some(),
        "the Bears stay in hand"
    );
    assert!(is_tapped(&engine, wizard), "{{T}} is part of the cost");
}

/// "You may": the menu can be declined.
#[test]
fn goblin_wizard_may_decline_to_put_the_goblin() {
    let p0 = PlayerId::new(0);
    let mut engine = board(&[goblin_raiders()]);
    activate(&mut engine, p0, goblin_wizard(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("choosing nothing is allowed");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, goblin_raiders()).is_some(),
        "declined: still in hand"
    );
    assert!(on_battlefield(&engine, p0, goblin_raiders()).is_none());
}

/// A hand with no Goblin permanent card gives nothing to choose and nothing
/// happens.
#[test]
fn goblin_wizard_with_no_goblin_in_hand_does_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = board(&[grizzly_bears()]);
    activate(&mut engine, p0, goblin_wizard(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
}

/// "{R}: Target Goblin gains protection from white until end of turn."
/// Seat 1 answers with a Swords to Plowshares (white): with the protection
/// the Goblin is not a target, without it it is.
#[test]
fn goblin_wizard_gives_a_goblin_protection_from_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for protect in [false, true] {
        let mut engine = Duel::new(SEED, mountain())
            .battlefield(
                0,
                &[
                    goblin_wizard(),
                    goblin_raiders(),
                    grizzly_bears(),
                    mountain(),
                ],
            )
            .hand(1, &[swords_to_plowshares()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        let wizard = on_battlefield(&engine, p0, goblin_wizard()).expect("the Wizard is out");
        let raiders = on_battlefield(&engine, p0, goblin_raiders()).expect("the Raiders are out");
        let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");

        if protect {
            float(&mut engine, 0, &[(ManaColor::Red, 1)]);
            activate(&mut engine, p0, goblin_wizard(), 1);
            let options = aim_at(&mut engine, p0, raiders);
            assert!(
                options.contains(&raiders) && options.contains(&wizard),
                "both Goblins can be named: {options:?}"
            );
            assert!(
                !options.contains(&bears),
                "the Bears are no Goblin: {options:?}"
            );
            pass_until(&mut engine, stack_is_empty);
        }

        engine
            .apply(p0, PlayerAction::PassPriority)
            .expect("p0 passes");
        assert!(
            matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
            "p1 has priority, got {:?}",
            engine.pending()
        );
        float(&mut engine, 1, &[(ManaColor::White, 1)]);
        cast_with_floating(&mut engine, p1, swords_to_plowshares());
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!(
                "expected the Swords target choice, got {:?}",
                engine.pending()
            )
        };
        assert_eq!(
            options.contains(&raiders),
            !protect,
            "protection from white keeps a white spell off the Goblin (protect = {protect}): {options:?}"
        );
        assert!(
            options.contains(&wizard),
            "the unprotected Wizard is still a target: {options:?}"
        );
    }
}
