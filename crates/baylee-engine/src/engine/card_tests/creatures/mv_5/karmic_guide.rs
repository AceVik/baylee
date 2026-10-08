//! `cards/creatures/mv_5/karmic_guide.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Karmic Guide returns a creature card from its controller's graveyard as
/// it enters; it flies; and at its controller's next upkeep its echo comes
/// due, and with nothing to pay it with it is sacrificed while the creature
/// it brought back stays.
#[test]
fn karmic_guide_reanimates_and_its_echo_takes_it_when_unpaid() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                steadfast_guard(),
            ],
        )
        .hand(0, &[karmic_guide_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p0, steadfast_guard()).unwrap();
    bury(&mut engine, &[guard]);
    assert!(in_graveyard(&engine, p0, steadfast_guard()).is_some());
    cast_from_hand(&mut engine, p0, karmic_guide_card());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let dead = in_graveyard(&engine, p0, steadfast_guard()).unwrap();
    aim_at(&mut engine, p0, dead);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, steadfast_guard()).is_some(),
        "back"
    );
    let guide = on_battlefield(&engine, p0, karmic_guide_card()).unwrap();
    assert!(keywords(&engine, guide).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert!(
        on_battlefield(&engine, p0, karmic_guide_card()).is_some(),
        "echo waits for our upkeep"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && !matches!(
                e.state().turn.step,
                crate::turn::Step::Untap | crate::turn::Step::Upkeep
            )
    });
    assert!(
        in_graveyard(&engine, p0, karmic_guide_card()).is_some(),
        "echo unpaid: sacrificed"
    );
    assert!(
        on_battlefield(&engine, p0, steadfast_guard()).is_some(),
        "what it returned stays"
    );
}

/// "Mulldrifter": "When this creature enters, draw two cards."
fn mulldrifter() -> CardIndex {
    card_index("24d0f5e7-0d9e-4b76-900e-a7274e80312d")
}

/// The creature Karmic Guide returns *enters* (CR 603.6a): its own enters
/// trigger fires, from the graveyard as it would from a hand.
///
/// Mulldrifter's "draw two cards" is the observable. It is buried first, so
/// the two cards in hand afterwards can only be the trigger of an arrival
/// from the graveyard; the hand is counted around the cast, and Karmic Guide
/// itself leaves it.
#[test]
fn karmic_guide_returns_a_creature_whose_enters_trigger_fires() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                mulldrifter(),
            ],
        )
        .hand(0, &[karmic_guide_card()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let drifter = on_battlefield(&engine, p0, mulldrifter()).expect("Mulldrifter is seated");
    bury(&mut engine, &[drifter]);
    let dead = in_graveyard(&engine, p0, mulldrifter()).expect("Mulldrifter is in the graveyard");
    let hand = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let before = hand(&engine);
    assert_eq!(before, 1, "only Karmic Guide is in hand");

    cast_from_hand(&mut engine, p0, karmic_guide_card());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, dead);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, mulldrifter()).is_some(),
        "Mulldrifter is back"
    );
    assert_eq!(
        hand(&engine),
        before - 1 + 2,
        "Karmic Guide left the hand and the returned Mulldrifter drew two"
    );
}
