#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn shadowspear_grants_stats_and_strips_opponents_hexproof_and_indestructible() {
    let mut e = prepared(
        &[index::HERO_S_DOWNFALL],
        &[index::SHADOWSPEAR, index::GRIZZLY_BEARS],
        &[index::THRUN_THE_LAST_TROLL, index::BAST_PANTHER_GODDESS],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let thrun = on_battlefield(&e, P1, index::THRUN_THE_LAST_TROLL).unwrap();
    let bast = on_battlefield(&e, P1, index::BAST_PANTHER_GODDESS).unwrap();

    // Equip Shadowspear to Grizzly Bears (ability index 3 is equip {2}).
    activate_by_card(&mut e, P0, index::SHADOWSPEAR, 3);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    let ch = e.state().object(bears).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(3)));
    assert!(ch.keywords.contains(KeywordSet::TRAMPLE));
    assert!(ch.keywords.contains(KeywordSet::LIFELINK));

    // Before activating Shadowspear's {1} ability, Thrun has hexproof and Bast has indestructible.
    assert!(keywords(&e, thrun).contains(KeywordSet::HEXPROOF));
    assert!(keywords(&e, bast).contains(KeywordSet::INDESTRUCTIBLE));

    // Activate Shadowspear's {1} ability (ability index 2).
    activate_by_card(&mut e, P0, index::SHADOWSPEAR, 2);
    pass_until(&mut e, stack_is_empty);

    assert!(!keywords(&e, thrun).contains(KeywordSet::HEXPROOF));
    assert!(!keywords(&e, bast).contains(KeywordSet::INDESTRUCTIBLE));

    // Now Hero's Downfall can target and destroy Bast!
    cast(&mut e, P0, index::HERO_S_DOWNFALL);
    targets(&mut e, vec![bast], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::BAST_PANTHER_GODDESS).is_none());
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
    assert!(ch.keywords.contains(KeywordSet::FLYING));
    assert!(ch.keywords.contains(KeywordSet::INDESTRUCTIBLE));
}
