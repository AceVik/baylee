//! `cards/creatures/mv_5/siege_gang_commander.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Siege-Gang Commander prints two sentences and the test plays both in one
/// main phase: "{3}{R}{R}" for a 2/2 Goblin whose enters-trigger makes three
/// 1/1 red Goblin creature tokens, and "{1}{R}, Sacrifice a Goblin: This
/// creature deals 2 damage to any target."
///
/// The two halves are each other's proof. Seven Mountains pay the cast down to
/// exactly the {1}{R} the ability charges, so the line is read out of an offer
/// the pool really covers (`can_afford` reads the pool and not the untapped
/// lands); the three tokens the entry made are then the only Goblins on the
/// board besides the Commander, so the sacrifice menu holding them — and
/// holding no Mountain — is what says they arrived out of the token pool as
/// Goblins. The damage is aimed across the table at a printed 1/1, where
/// "any target" (CR 115.4) reaches a creature and two damage is lethal.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn siege_gang_commander_makes_three_goblins_and_feeds_one_back_to_its_own_gun() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 7])
        .hand(0, &[siege_gang_commander()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Seven Mountains and nothing else on this side makes mana, so "seven" is a
    // count of lands and not of a creature that taps alongside them.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Mountains tapped, seven red"
    );
    cast_with_floating(&mut engine, p0, siege_gang_commander());
    pass_until(&mut engine, stack_is_empty);

    let commander =
        on_battlefield(&engine, p0, siege_gang_commander()).expect("the Commander resolved");
    assert_eq!(pt(&engine, commander), (2, 2), "the printed 2/2 body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{3}}{{R}}{{R}} is spent and exactly the {{1}}{{R}} the ability \
         charges is still floating"
    );

    // "When this creature enters, create three 1/1 red Goblin creature tokens."
    let goblins = tokens_of(&engine, p0);
    assert_eq!(goblins.len(), 3, "one entry, three tokens");
    for token in &goblins {
        assert_eq!(pt(&engine, *token), (1, 1), "the printed 1/1 body");
        let printed = engine
            .state()
            .object(*token)
            .expect("the token is an object")
            .token
            .expect("it knows which token it is");
        assert!(
            printed.colors.contains(baylee_core::color::Color::Red),
            "a red token, and not a colourless one"
        );
    }

    let victim = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, victim),
        (1, 1),
        "a printed 1/1 for two damage to kill"
    );
    let fodder = goblins[0];

    // Ability 0 is the {1}{R} line; the enters-trigger behind it is a
    // `Trigger` and is never offered as an activation, so the index is the
    // card's own order and nothing has to be guessed.
    activate(&mut engine, p0, siege_gang_commander(), 0);

    // The two questions one activation asks: what is aimed at (CR 601.2c) and
    // which Goblin is given up (CR 601.2h). Answered in the order they arrive.
    let mut target_menu: Vec<ObjectId> = Vec::new();
    let mut sacrifice_menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if !target_menu.is_empty() && !sacrifice_menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat is the one that aims it");
                assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
                assert!(
                    options.contains(&victim),
                    "\"any target\" reaches the creature across the table: {options:?}"
                );
                assert!(
                    player_options.contains(&p0) && player_options.contains(&p1),
                    "CR 115.4: \"any target\" counts players in the same choice: \
                     {player_options:?}"
                );
                target_menu = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![victim],
                            players: vec![],
                        },
                    )
                    .expect("their Elf was one of the options it enumerated");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat answers its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one Goblin, no more and no fewer");
                sacrifice_menu = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the Goblin the question offered pays the cost");
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Commander's ability resolves: {other:?}"),
        }
    }

    assert!(
        !target_menu.is_empty() && !sacrifice_menu.is_empty(),
        "one activation asks both questions"
    );
    for token in &goblins {
        assert!(
            sacrifice_menu.contains(token),
            "a token the entry made is a Goblin and may be given up: {sacrifice_menu:?}"
        );
    }
    assert!(
        sacrifice_menu.contains(&commander),
        "\"a Goblin\" is a subtype and not a name, so the Commander is on its \
         own menu too: {sacrifice_menu:?}"
    );
    assert_eq!(
        sacrifice_menu.len(),
        4,
        "those four Goblins are the whole menu and nothing else is one: {sacrifice_menu:?}"
    );
    for land in all_on_battlefield(&engine, p0, mountain()) {
        assert!(
            !sacrifice_menu.contains(&land),
            "a Mountain is a land and no Goblin: {sacrifice_menu:?}"
        );
    }

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} it charges came out of the pool"
    );
    let left = tokens_of(&engine, p0);
    assert_eq!(left.len(), 2, "one Goblin paid the cost, so two are left");
    assert!(
        !left.contains(&fodder),
        "the token that was sacrificed left the battlefield (CR 111.7)"
    );
    assert!(
        on_battlefield(&engine, p0, siege_gang_commander()).is_some(),
        "the Commander paid the mana and not itself, so it is still standing"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_creature()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to the seat \
         whose board it stood on"
    );
}
