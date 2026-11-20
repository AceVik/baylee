//! `cards/creatures/mv_3/sunscape_battlemage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn sunscape_battlemage() -> CardIndex {
    card_index("c8fd2232-e499-4889-a2ca-e486ea87d02b")
}

/// Casts Sunscape Battlemage off floating mana and answers its dual kicker
/// prompts `{1}{G}` (index 0) and `{2}{U}` (index 1) in Oracle order.
fn cast_sunscape_battlemage(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick_green: bool,
    kick_blue: bool,
) {
    cast_with_floating(engine, seat, sunscape_battlemage());
    let mut saw_0 = false;
    let mut saw_1 = false;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::KickerOption { index, cost, parts },
                ..
            } => {
                assert_eq!(player, seat);
                assert_eq!(parts, baylee_engine::choice::BuybackParts::default());
                match index {
                    0 => {
                        assert!(!saw_0, "index 0 asked once");
                        assert_eq!(
                            cost,
                            baylee_core::mana::ManaCost::parse("{1}{G}"),
                            "Sunscape Battlemage's first kicker in Oracle text is {{1}}{{G}}"
                        );
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_green)).unwrap();
                    }
                    1 => {
                        assert!(saw_0, "first kicker precedes second kicker in Oracle order");
                        assert!(!saw_1, "index 1 asked once");
                        assert_eq!(
                            cost,
                            baylee_core::mana::ManaCost::parse("{2}{U}"),
                            "Sunscape Battlemage's second kicker in Oracle text is {{2}}{{U}}"
                        );
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_blue)).unwrap();
                    }
                    other => panic!("unexpected kicker index {other}"),
                }
            }
            _ => break,
        }
    }
    assert!(saw_0 && saw_1, "both dual kicker options were offered");
}

#[test]
fn sunscape_battlemage_unkicked_does_not_stack_either_etb_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                forest(),
                forest(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[sunscape_battlemage()])
        .battlefield(1, &[flying_men()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = cards_in(&engine, ZoneLocation::Hand(p0));
    tap_all_mana(&mut engine, p0);
    cast_sunscape_battlemage(&mut engine, p0, false, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let mage = on_battlefield(&engine, p0, sunscape_battlemage()).expect("Battlemage entered");
    assert_eq!(pt(&engine, mage), (2, 2));
    assert!(
        on_battlefield(&engine, p1, flying_men()).is_some(),
        "intervening if: unkicked Battlemage never destroys the flier"
    );
    assert_eq!(
        cards_in(&engine, ZoneLocation::Hand(p0)),
        hand_before - 1,
        "intervening if: unkicked Battlemage draws no cards"
    );
}

#[test]
fn sunscape_battlemage_first_kicker_destroys_only_flying_creature_and_skips_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                forest(),
                forest(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[sunscape_battlemage()])
        .battlefield(1, &[flying_men(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let flier = on_battlefield(&engine, p1, flying_men()).unwrap();
    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let hand_before = cards_in(&engine, ZoneLocation::Hand(p0));

    tap_all_mana(&mut engine, p0);
    cast_sunscape_battlemage(&mut engine, p0, true, false);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&flier),
        "flier is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&bear),
        "non-flying creature cannot be targeted: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![flier],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, flying_men()).is_some());
    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(
        cards_in(&engine, ZoneLocation::Hand(p0)),
        hand_before - 1,
        "{{2}}{{U}} kicker was not paid, so no cards are drawn"
    );
}

#[test]
fn sunscape_battlemage_second_kicker_draws_two_and_skips_destroy() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                forest(),
                forest(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[sunscape_battlemage()])
        .battlefield(1, &[flying_men()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = cards_in(&engine, ZoneLocation::Hand(p0));
    let lib_before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_sunscape_battlemage(&mut engine, p0, false, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p1, flying_men()).is_some(),
        "{{1}}{{G}} kicker was not paid, so the flier trigger never stacks"
    );
    assert_eq!(
        cards_in(&engine, ZoneLocation::Hand(p0)),
        hand_before - 1 + 2,
        "{{2}}{{U}} kicker draws two cards"
    );
    assert_eq!(library_size(&engine, p0), lib_before - 2);
}

#[test]
fn sunscape_battlemage_both_kickers_destroy_flier_and_draw_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                forest(),
                forest(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[sunscape_battlemage()])
        .battlefield(1, &[flying_men()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let flier = on_battlefield(&engine, p1, flying_men()).unwrap();
    let hand_before = cards_in(&engine, ZoneLocation::Hand(p0));
    tap_all_mana(&mut engine, p0);
    cast_sunscape_battlemage(&mut engine, p0, true, true);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, flier);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_graveyard(&engine, p1, flying_men()).is_some());
    assert_eq!(
        cards_in(&engine, ZoneLocation::Hand(p0)),
        hand_before - 1 + 2
    );
}
