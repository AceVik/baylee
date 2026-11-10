//! `cards/enchantments/mv_4/conversion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Conversion: "All Mountains are Plains." Every player's Mountain becomes
/// one and taps for {W}; a Forest beside it, never a Mountain, is
/// untouched.
#[test]
fn conversion_makes_every_players_mountains_plains() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), mountain(), forest()],
        )
        .battlefield(1, &[mountain()])
        .hand(0, &[conversion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p0_mountain = on_battlefield(&engine, p0, mountain()).expect("p0's Mountain is seated");
    let p0_forest = on_battlefield(&engine, p0, forest()).expect("p0's Forest is seated");
    let p1_mountain = on_battlefield(&engine, p1, mountain()).expect("p1's Mountain is seated");

    let plains_ids = all_on_battlefield(&engine, p0, plains());
    assert_eq!(plains_ids.len(), 4, "four Plains pay for Conversion");
    tap_mana_where(&mut engine, p0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, p0, conversion());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, conversion()).is_some()
    });

    let mountain_subtype = baylee_core::generated::subtypes::land::MOUNTAIN;
    let plains_subtype = baylee_core::generated::subtypes::land::PLAINS;
    let forest_subtype = baylee_core::generated::subtypes::land::FOREST;

    let p0_mountain_now = engine
        .state()
        .object(p0_mountain)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(
        p0_mountain_now.contains(plains_subtype),
        "p0's own Mountain is a Plains now"
    );
    assert!(
        !p0_mountain_now.contains(mountain_subtype),
        "and no longer a Mountain"
    );

    let p1_mountain_now = engine
        .state()
        .object(p1_mountain)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(
        p1_mountain_now.contains(plains_subtype),
        "\"All Mountains\": the opponent's too, not just the caster's own"
    );
    assert!(!p1_mountain_now.contains(mountain_subtype));

    let p0_forest_now = engine
        .state()
        .object(p0_forest)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(
        p0_forest_now.contains(forest_subtype),
        "a Forest, never a Mountain, is untouched"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: p0_mountain,
            },
        )
        .unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: p0_forest })
        .unwrap();
    let pool0 = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool0.available(ManaColor::White),
        1,
        "the converted Mountain taps for {{W}}"
    );
    assert_eq!(pool0.available(ManaColor::Red), 0, "not {{R}} any more");
    assert_eq!(
        pool0.available(ManaColor::Green),
        1,
        "the Forest still taps for {{G}}"
    );

    reach_their_main_phase(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateManaAbility {
                source: p1_mountain,
            },
        )
        .unwrap();
    let pool1 = &engine.state().players[1].mana_pool;
    assert_eq!(
        pool1.available(ManaColor::White),
        1,
        "the opponent's converted Mountain also taps for {{W}}"
    );
    assert_eq!(pool1.available(ManaColor::Red), 0);
}

/// Conversion: "At the beginning of your upkeep, sacrifice this
/// enchantment unless you pay {W}{W}." Paying keeps it; declining
/// sacrifices it; the question is never asked at the opponent's upkeep.
#[test]
fn conversion_upkeep_trigger_pays_ww_or_sacrifices_itself() {
    let p0 = PlayerId::new(0);

    // Paying {W}{W} keeps Conversion.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[conversion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let all_plains = all_on_battlefield(&engine, p0, plains());
    assert_eq!(all_plains.len(), 6, "six Plains are seated");
    let reserved: Vec<_> = all_plains[..2].to_vec();
    tap_mana_where(&mut engine, p0, |id| !reserved.contains(&id));
    cast_with_floating(&mut engine, p0, conversion());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, conversion()).is_some()
    });

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(engine.state().turn.active, p0, "p1's upkeep asked nothing");
    assert_eq!(
        player, p0,
        "\"your upkeep\" asks Conversion's own controller"
    );
    assert_eq!(
        prompt,
        YesNoPrompt::PayMana {
            cost: baylee_core::mana!("{W}{W}")
        }
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        on_battlefield(&engine, p0, conversion()).is_some(),
        "paying {{W}}{{W}} keeps Conversion on the battlefield"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        4,
        "six Plains made six white, and two of them paid the {{W}}{{W}}"
    );

    // Declining sacrifices it.
    let mut decline = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[conversion()])
        .start();
    keep_mulligans(&mut decline);
    reach_main_phase(&mut decline, p0);
    cast_from_hand(&mut decline, p0, conversion());
    pass_until(&mut decline, |e| {
        on_battlefield(e, p0, conversion()).is_some()
    });
    pass_until(&mut decline, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    decline.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert!(
        on_battlefield(&decline, p0, conversion()).is_none(),
        "declining sacrifices Conversion"
    );
    assert!(
        in_graveyard(&decline, p0, conversion()).is_some(),
        "sacrificed means the graveyard, not gone from the game"
    );
}
