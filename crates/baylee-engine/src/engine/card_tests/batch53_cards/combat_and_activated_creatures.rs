#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn bast_panther_goddess_cant_attack_with_fewer_than_three_creatures_and_pumps_attacker() {
    let mut e = prepared(
        &[index::LLANOWAR_ELVES],
        &[index::BAST_PANTHER_GODDESS, index::GRIZZLY_BEARS],
        &[],
    );
    let bast = on_battlefield(&e, P0, index::BAST_PANTHER_GODDESS).unwrap();
    assert!(
        keywords(&e, bast).contains(KeywordSet::CANT_ATTACK),
        "Bast can't attack while P0 controls fewer than 3 creatures"
    );

    // Cast Llanowar Elves so P0 controls 3 creatures.
    cast(&mut e, P0, index::LLANOWAR_ELVES);
    pass_until(&mut e, stack_is_empty);
    assert!(
        !keywords(&e, bast).contains(KeywordSet::CANT_ATTACK),
        "Bast can attack once P0 controls 3 creatures"
    );

    // Attack with Bast -> Whenever you attack trigger gives target attacking creature +3/+3 until EOT.
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bast, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![bast], vec![]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert_eq!(
        e.state().object(bast).unwrap().characteristics().power,
        Some(7),
        "Bast (4/4) gets +3/+3 for the 3 creatures P0 controls"
    );
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(e.state().players[1].life, 13);
}

#[test]
fn tinybones_the_pickpocket_casts_permanent_from_damaged_players_graveyard() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::TINYBONES_THE_PICKPOCKET])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);

    let tiny = on_battlefield(&e, P0, index::TINYBONES_THE_PICKPOCKET).unwrap();

    // Put Grizzly Bears into P1's graveyard.
    let bears_hand = in_hand(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let bears_gy = e
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            bears_hand,
            ZoneLocation::Graveyard(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();

    // Attack with Tinybones unblocked.
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(tiny, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Combat damage trigger asks for target nonland permanent card in P1's graveyard.
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![bears_gy], vec![]);

    // Resolution asks YesNo whether to cast the targeted card; saying yes opens a payment window.
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, true);
    mana(&mut e);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some(),
        "Tinybones cast P1's Grizzly Bears under P0's control"
    );
}

#[test]
fn the_serpent_society_ward_poison_and_deathtouch_dies_edict() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[index::THE_SERPENT_SOCIETY, index::TINYBONES_THE_PICKPOCKET],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[index::HERO_S_DOWNFALL])
        .hand(1, &[index::HERO_S_DOWNFALL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);

    let serpent = on_battlefield(&e, P0, index::THE_SERPENT_SOCIETY).unwrap();
    let tiny = on_battlefield(&e, P0, index::TINYBONES_THE_PICKPOCKET).unwrap();
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();

    // P1 casts Hero's Downfall targeting The Serpent Society -> Ward triggers!
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    mana(&mut e);
    let downfall = in_hand(&e, P1, index::HERO_S_DOWNFALL).unwrap();
    e.apply(P1, PlayerAction::CastSpell { card: downfall })
        .unwrap();
    targets(&mut e, vec![serpent], vec![]);

    // Ward resolves and asks P1 whether to get 5 poison counters; P1 declines -> Hero's Downfall is countered!
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, false);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, index::THE_SERPENT_SOCIETY).is_some(),
        "The Serpent Society survived because P1 declined Ward—Get five poison counters"
    );

    // Now Tinybones (another creature P0 controls with deathtouch) dies -> P1 must sacrifice Grizzly Bears!
    cast(&mut e, P0, index::HERO_S_DOWNFALL);
    targets(&mut e, vec![tiny], vec![]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { player: P1, .. })
    });
    choose_objects(&mut e, vec![bears]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P1, index::GRIZZLY_BEARS).is_none(),
        "P1 had to sacrifice their nontoken creature when Tinybones died"
    );
}

#[test]
fn aphelia_viper_whisperer_attack_snake_token_and_deathtouch_half_life_loss() {
    let mut e = prepared(&[], &[index::APHELIA_VIPER_WHISPERER], &[]);
    let aphelia = on_battlefield(&e, P0, index::APHELIA_VIPER_WHISPERER).unwrap();

    // Attack with Aphelia first to create the 1/1 deathtouch Snake token, then on next turn activate {4}{B} and attack with the Snake token!
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(aphelia, Defender::Player(P1))],
        },
    )
    .unwrap();
    mana(&mut e);
    // Aphelia's attack trigger asks P0 to pay {1}{B/G} to create a 1/1 black Snake with deathtouch.
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, true);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    let snake = e
        .state()
        .battlefield_seen()
        .find(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.controller == P0 && o.token.is_some())
        })
        .expect("1/1 deathtouch Snake token created");

    // Advance to P0's next turn, activate Aphelia's delayed trigger ability, and attack with the Snake token.
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    activate_by_card(&mut e, P0, index::APHELIA_VIPER_WHISPERER, 1);
    pass_until(&mut e, stack_is_empty);

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(snake, Defender::Player(P1))],
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

    // P1 took 1 from Aphelia (19), 1 from Snake (18), and then lost half of 18 = 9 life => 9 life!
    assert_eq!(e.state().players[1].life, 9);
}

#[test]
fn tony_stark_transforms_into_iron_man_and_deploys_equipment_attached_at_combat() {
    let mut e = prepared(&[index::SHADOWSPEAR], &[index::TONY_STARK], &[]);
    let tony = on_battlefield(&e, P0, index::TONY_STARK).unwrap();

    // Activate Tony Stark's {4}{U}{R}: Transform Tony Stark (ability index 1).
    activate_by_card(&mut e, P0, index::TONY_STARK, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(tony).unwrap().face_index, 1);

    // Move to beginning of combat: The Invincible Iron Man triggers and puts Shadowspear from hand onto the battlefield attached to him!
    pass_until(&mut e, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let spear_in_hand = in_hand(&e, P0, index::SHADOWSPEAR).unwrap();
    choose_objects(&mut e, vec![spear_in_hand]);
    pass_until(&mut e, stack_is_empty);

    let spear_bf = on_battlefield(&e, P0, index::SHADOWSPEAR).expect("Shadowspear deployed");
    assert_eq!(
        e.state().object(spear_bf).unwrap().attached_to,
        Some(tony),
        "Shadowspear automatically attached to The Invincible Iron Man"
    );
    assert_eq!(
        e.state().object(tony).unwrap().characteristics().power,
        Some(6),
        "Iron Man (5/5) + Shadowspear (+1/+1) = 6/6"
    );
}

#[test]
fn psychic_frog_discards_for_counter_exiles_three_for_flying_and_draws_on_hit() {
    let mut e = prepared(
        &[index::FOREST, index::FOREST, index::FOREST, index::FOREST],
        &[index::PSYCHIC_FROG],
        &[],
    );
    let frog = on_battlefield(&e, P0, index::PSYCHIC_FROG).unwrap();

    // Discard 3 Forests to put 3 +1/+1 counters on Psychic Frog (and fill graveyard with 3 cards).
    for _ in 0..3 {
        let f = in_hand(&e, P0, index::FOREST).unwrap();
        activate_by_card(&mut e, P0, index::PSYCHIC_FROG, 1);
        choose_objects(&mut e, vec![f]);
        pass_until(&mut e, stack_is_empty);
    }
    assert_eq!(
        e.state()
            .object(frog)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        3
    );

    // Exile 3 cards from graveyard to give Psychic Frog flying until EOT (ability index 2).
    let gy: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Graveyard(P0))
        .iter()
        .copied()
        .take(3)
        .collect();
    activate_by_card(&mut e, P0, index::PSYCHIC_FROG, 2);
    for card in gy {
        choose_objects(&mut e, vec![card]);
    }
    pass_until(&mut e, stack_is_empty);
    assert!(keywords(&e, frog).contains(KeywordSet::FLYING));
}

#[test]
fn sab_sunen_luxa_embodied_parity_combat_restriction_and_first_main_trigger() {
    let mut e = prepared(&[], &[index::SAB_SUNEN_LUXA_EMBODIED], &[]);
    let sab = on_battlefield(&e, P0, index::SAB_SUNEN_LUXA_EMBODIED).unwrap();

    // Before Turn 1's FirstMain trigger resolves, Sab-Sunen has 0 (even) counters -> can attack/block!
    assert!(!keywords(&e, sab).contains(KeywordSet::CANT_ATTACK));

    // Resolve Turn 1's FirstMain trigger: Sab-Sunen puts 1 +1/+1 counter on itself (1 = odd) and draws 2 cards!
    let hand_before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(sab)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 2,
        "Sab-Sunen drew 2 cards when reaching an odd number of counters"
    );
    assert!(
        keywords(&e, sab).contains(KeywordSet::CANT_ATTACK),
        "With 1 (odd) counter, Sab-Sunen can't attack or block"
    );

    // Advance to P0's next FirstMain and resolve: Sab-Sunen gets its 2nd counter (2 = even) -> can attack/block again!
    let turn = e.state().turn.number;
    pass_until(&mut e, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && stack_is_empty(e)
    });
    assert_eq!(
        e.state()
            .object(sab)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert!(!keywords(&e, sab).contains(KeywordSet::CANT_ATTACK));
}
