#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn shadowspear_grants_stats_and_strips_opponents_hexproof_and_indestructible() {
    let mut e = prepared(
        &[index::HERO_S_DOWNFALL],
        &[
            index::SHADOWSPEAR,
            index::GRIZZLY_BEARS,
            index::THRUN_THE_LAST_TROLL,
            index::BAST_PANTHER_GODDESS,
        ],
        &[
            index::THRUN_THE_LAST_TROLL,
            index::BAST_PANTHER_GODDESS,
            index::DARKSTEEL_CITADEL,
        ],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let thrun = on_battlefield(&e, P1, index::THRUN_THE_LAST_TROLL).unwrap();
    let bast = on_battlefield(&e, P1, index::BAST_PANTHER_GODDESS).unwrap();
    let citadel = on_battlefield(&e, P1, index::DARKSTEEL_CITADEL).unwrap();
    let own_thrun = on_battlefield(&e, P0, index::THRUN_THE_LAST_TROLL).unwrap();
    let own_bast = on_battlefield(&e, P0, index::BAST_PANTHER_GODDESS).unwrap();

    // Equip Shadowspear to Grizzly Bears (ability index 3 is equip {2}).
    activate_by_card(&mut e, P0, index::SHADOWSPEAR, 3);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    let ch = e.state().object(bears).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(3)));
    assert!(ch.keywords.contains(KeywordSet::TRAMPLE));
    assert!(ch.keywords.contains(KeywordSet::LIFELINK));

    assert!(keywords(&e, thrun).contains(KeywordSet::HEXPROOF));
    assert!(keywords(&e, bast).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(keywords(&e, citadel).contains(KeywordSet::INDESTRUCTIBLE));

    // Activate Shadowspear's {1} ability (ability index 2).
    activate_by_card(&mut e, P0, index::SHADOWSPEAR, 2);
    pass_until(&mut e, stack_is_empty);

    assert!(!keywords(&e, thrun).contains(KeywordSet::HEXPROOF));
    assert!(!keywords(&e, bast).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(!keywords(&e, citadel).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(
        keywords(&e, own_thrun).contains(KeywordSet::HEXPROOF),
        "the controller's own permanents keep hexproof"
    );
    assert!(
        keywords(&e, own_bast).contains(KeywordSet::INDESTRUCTIBLE),
        "and keep indestructible"
    );

    // With indestructible removed, Hero's Downfall really destroys Bast.
    cast(&mut e, P0, index::HERO_S_DOWNFALL);
    targets(&mut e, vec![bast], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::BAST_PANTHER_GODDESS).is_none());

    baylee_card_testkit::card_tests::lands::cross_into_the_next_own_main(&mut e, P0);
    assert!(
        keywords(&e, thrun).contains(KeywordSet::HEXPROOF),
        "hexproof returns after the end of this turn"
    );
    assert!(
        keywords(&e, citadel).contains(KeywordSet::INDESTRUCTIBLE),
        "indestructible returns too"
    );
}

#[test]
fn umezawas_jitte_gains_charge_counters_on_combat_damage_and_activates_all_three_modes() {
    let mut e = prepared(
        &[],
        &[index::UMEZAWA_S_JITTE, index::GRIZZLY_BEARS],
        &[index::LLANOWAR_ELVES],
    );
    let jitte = on_battlefield(&e, P0, index::UMEZAWA_S_JITTE).unwrap();
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&e, P1, index::LLANOWAR_ELVES).unwrap();

    // Equip Jitte to Grizzly Bears (ability index 2 is equip {2}).
    activate_by_card(&mut e, P0, index::UMEZAWA_S_JITTE, 2);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    // Attack with Grizzly Bears unblocked -> combat damage triggers Jitte for 2 charge counters.
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        e.state()
            .object(jitte)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        2
    );

    // Mode 0: Equipped creature gets +2/+2 until EOT.
    activate_by_card(&mut e, P0, index::UMEZAWA_S_JITTE, 1);
    e.apply(P0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(bears).unwrap().characteristics().power,
        Some(4)
    );

    // Mode 1: Target creature gets -1/-1 until EOT (kills Llanowar Elves).
    activate_by_card(&mut e, P0, index::UMEZAWA_S_JITTE, 1);
    e.apply(P0, PlayerAction::ChooseMode(1)).unwrap();
    targets(&mut e, vec![elves], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::LLANOWAR_ELVES).is_none());

    // Add 1 more charge counter to test Mode 2 (gain 2 life).
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(jitte)
        .unwrap()
        .counters
        .add(CounterKind::Charge, 1);
    e.refresh_offer();
    let life_before = e.state().players[0].life;
    activate_by_card(&mut e, P0, index::UMEZAWA_S_JITTE, 1);
    e.apply(P0, PlayerAction::ChooseMode(2)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, life_before + 2);
}

#[test]
fn vampire_hexmage_first_strike_kills_a_two_two_before_it_deals_damage() {
    let mut e = prepared(&[], &[index::VAMPIRE_HEXMAGE], &[index::GRIZZLY_BEARS]);
    let attacker = on_battlefield(&e, P0, index::VAMPIRE_HEXMAGE).unwrap();
    let blocker = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, attacker)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(in_graveyard(&e, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P0, index::VAMPIRE_HEXMAGE).is_some());
}

#[test]
fn dark_depths_and_vampire_hexmage_combo_creates_marit_lage() {
    let mut e = prepared(&[index::DARK_DEPTHS], &[index::VAMPIRE_HEXMAGE], &[]);
    let depths = play_land(&mut e, P0, index::DARK_DEPTHS);
    assert_eq!(
        e.state()
            .object(depths)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::ICE),
        10,
        "Dark Depths enters with 10 ice counters"
    );

    // Activate Dark Depths's {3} ability once to remove 1 ice counter (down to 9).
    activate_by_card(&mut e, P0, index::DARK_DEPTHS, 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(depths)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::ICE),
        9
    );

    // Sacrifice Vampire Hexmage to remove all counters from Dark Depths!
    activate_by_card(&mut e, P0, index::VAMPIRE_HEXMAGE, 0);
    targets(&mut e, vec![depths], vec![]);
    pass_until(&mut e, stack_is_empty);

    // Dark Depths was sacrificed and Marit Lage (20/20 flying indestructible Avatar) is on the battlefield!
    assert!(on_battlefield(&e, P0, index::DARK_DEPTHS).is_none());
    let marit_lage = e
        .state()
        .battlefield_seen()
        .find(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.controller == P0
                    && o.token.is_some()
                    && o.characteristics().power == Some(20)
                    && o.characteristics().toughness == Some(20)
            })
        })
        .expect("Marit Lage 20/20 token created");
    let ch = e.state().object(marit_lage).unwrap().characteristics();
    assert!(ch.supertypes.contains(SupertypeSet::LEGENDARY));
    assert_eq!(
        ch.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black])
    );
    assert!(ch.types.contains(TypeSet::CREATURE));
    assert!(ch.subtypes.contains(subtypes::creature::AVATAR));
    assert!(ch.keywords.contains(KeywordSet::FLYING));
    assert!(ch.keywords.contains(KeywordSet::INDESTRUCTIBLE));
}
