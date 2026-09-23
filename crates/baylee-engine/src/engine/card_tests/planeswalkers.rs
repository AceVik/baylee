//! Planeswalkers, the door `cards/planeswalkers/` puts them behind:
//! loyalty abilities, and what a walker's own ability may reach.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Karn, the Great Creator −2: "reveal an artifact card you own from outside
/// the game ... put that card into your hand."
///
/// Also the regression test for the sideboard itself: those cards must be
/// reachable by the wish and absent from the library, which is where they
/// used to end up.
#[test]
fn karn_minus_two_pulls_an_artifact_from_outside_the_game() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[karn_the_great_creator()])
        .sideboard(0, &[chromatic_lantern()])
        .start();
    keep_mulligans(&mut engine);

    let library = engine.state().zones.list(ZoneLocation::Library(p0));
    assert!(
        !library.iter().any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == chromatic_lantern()))
        }),
        "the sideboard was shuffled into the library"
    );

    reach_main_phase(&mut engine, p0);
    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 2,
            },
        )
        .unwrap();

    // The ability goes on the stack; the wish is offered when it resolves.
    let mut offered = None;
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                options, min, max, ..
            } => {
                assert_eq!((min, max), (0, 1), "the wish is optional and singular");
                offered = Some(options);
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while resolving the wish: {other:?}"),
        }
    }
    let offered = offered.expect("the wish offered the sideboard");
    assert_eq!(
        offered.len(),
        1,
        "only the artifact outside the game qualifies"
    );

    let wanted = offered[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wanted],
            },
        )
        .unwrap();
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&wanted),
        "the wished-for card is in hand"
    );
}

/// Karn, the Great Creator +1: "up to **one target** noncreature artifact
/// becomes an artifact creature with power and toughness each equal to its
/// mana value."
///
/// It animated *every* noncreature artifact on every battlefield, because
/// both halves of the sentence were written with the same filter the
/// targeting used — which reads like the same claim and is not. Pointed at a
/// nought-cost artifact it was a one-sided board wipe: everything it touched
/// became a 0/0 and the next state-based check swept it up. Reported from a
/// game as "all lands and artifacts were removed from all fields, except
/// creatures", which is this filter exactly — an artifact *creature* is not
/// a noncreature artifact and was the only thing left standing.
#[test]
fn karn_plus_one_animates_the_target_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(43, forest())
        .battlefield(
            0,
            &[karn_the_great_creator(), mox_opal(), chromatic_lantern()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    let mox = on_battlefield(&engine, p0, mox_opal()).expect("the mox is out");
    let lantern = on_battlefield(&engine, p0, chromatic_lantern()).expect("the lantern is out");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 1,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mox) && options.contains(&lantern),
        "both noncreature artifacts are targetable: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![mox] })
        .unwrap();
    pass_until(&mut engine, |e| on_battlefield(e, p0, mox_opal()).is_none());

    // The mox is a nought-cost artifact, so it animated into a 0/0 and died
    // to a state-based action. That is the rules answer for the card the
    // ability was pointed at, and it is how the fault was noticed at all.
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == mox_opal()))
            }),
        "the target became a 0/0 and was put into its owner's graveyard"
    );
    // And the bystander is untouched: still on the battlefield, and still
    // not a creature. Before the fix it was a 0/0 in the graveyard beside
    // the mox, along with every other noncreature artifact in the game.
    let still = engine
        .state()
        .object(lantern)
        .expect("the lantern was never targeted and is still on the table");
    assert!(
        !still.characteristics().types.intersects(TypeSet::CREATURE),
        "an untargeted artifact was animated too"
    );
    assert!(
        on_battlefield(&engine, p0, chromatic_lantern()).is_some(),
        "and it is still on the battlefield"
    );
}

/// And the −2 offers the sideboard and exile, never the library.
///
/// Filed beside the wish's own test because the two failures look identical
/// from the client: a dialog full of artifact cards the player did not
/// expect. This one fills the library with the very artifact the wish
/// matches, so a version that read `Library(you)` would offer sixty of them.
#[test]
fn karn_minus_two_never_offers_the_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(44, chromatic_lantern())
        .battlefield(0, &[karn_the_great_creator()])
        .sideboard(0, &[mox_opal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 2,
            },
        )
        .unwrap();
    let offered = loop {
        match engine.pending().clone() {
            Pending::ChooseCards { options, .. } => break options,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while resolving the wish: {other:?}"),
        }
    };
    for id in &offered {
        let zone = engine.state().object(*id).map(|o| o.zone);
        assert!(
            matches!(zone, Some(Zone::OutsideGame | Zone::Exile)),
            "the wish offered a card in {zone:?}"
        );
    }
    assert_eq!(offered.len(), 1, "only the mox is outside the game");
}

/// Aminatou's −1 exiles a permanent you own and returns it — the flicker
/// the owner reported. A tapped land came back tapped.
///
/// CR 400.7: what comes back is a new object with no memory of the old
/// one, and nothing cleared the old one's status, so the tapped bit rode
/// through the exile zone and back. The fix is in
/// [`crate::state::GameState::move_object`], which is the door every zone
/// change goes through, so a bounced creature and a reanimated one are
/// covered by the same three lines.
#[test]
fn a_blinked_permanent_comes_back_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[aminatou(), forest(), plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Tap the land the only way a player can: use it.
    tap_all_mana(&mut engine, p0);
    let land = on_battlefield(&engine, p0, forest()).expect("the forest is on the battlefield");
    assert!(
        engine
            .state()
            .object(land)
            .expect("the land is there")
            .status
            .contains(Status::TAPPED),
        "the land is tapped before the flicker",
    );

    activate(&mut engine, p0, aminatou(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(options.contains(&land), "the tapped land is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: vec![],
            },
        )
        .expect("the land is targeted");

    pass_until(&mut engine, |e| {
        e.state()
            .object(land)
            .is_some_and(|o| o.zone == Zone::Battlefield)
            && e.state().zones.list(ZoneLocation::Stack).is_empty()
    });
    assert!(
        !engine
            .state()
            .object(land)
            .expect("the land came back")
            .status
            .contains(Status::TAPPED),
        "a permanent that changed zones is a new object and enters untapped",
    );
}

/// Jace, the Mind Sculptor's +2: "Look at the top card of **target
/// player's** library. **You** may put that card on the bottom of **that
/// player's** library."
///
/// Two players, two roles, and the engine had each of them on the wrong
/// seat. `Effect::ScryFor` handed the `Pending::ChooseCards` to the target,
/// so the *opponent* decided whether to keep their own card — the opposite
/// of what the card says. And `AwaitingOp::Scry` then bottomed the chosen
/// card into `res.controller`'s library, which is not a misplacement so
/// much as a theft: the card came out of one player's library and went into
/// another's.
///
/// Jace is the only card in the pool that uses `ScryFor`, and nothing in
/// the suite had ever activated it, which is how both survived. The two
/// library counts below are what catch the second one; the asked seat
/// catches the first.
#[test]
fn jace_looks_at_a_targets_library_and_the_controller_decides() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(53, forest())
        .battlefield(0, &[jace_the_mind_sculptor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = (library_size(&engine, p0), library_size(&engine, p1));
    let top_of_theirs = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .expect("the opponent has a library");

    let jace = on_battlefield(&engine, p0, jace_the_mind_sculptor()).expect("jace deployed");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: jace,
                ability_index: 0,
            },
        )
        .expect("the +2 activates");
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        panic!("the +2 asks whose library, got {:?}", engine.pending())
    };
    assert!(options.contains(&p1), "the opponent is a legal target");
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();

    // The ability resolves, and the question it raises goes to Jace's
    // controller — never to the player whose library is being looked at.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "\"you may put that card on the bottom\" — you");
    assert_eq!(cards, vec![top_of_theirs], "and it is their top card");
    assert_eq!(piles, scry_piles(1), "the \"may\" is the zero minimum");

    engine
        .apply(p0, look_answer(&cards, &[top_of_theirs]))
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. }) && stack_is_empty(e)
    });

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p1))
            .first()
            .copied(),
        Some(top_of_theirs),
        "the card goes to the bottom of the library it came out of"
    );
    assert_eq!(
        (library_size(&engine, p0), library_size(&engine, p1)),
        before,
        "and no card crossed between the two libraries"
    );
}

/// The owner's report: Venser's +2 flickered a Great Divide Guide, and when
/// it came back at the beginning of the end step the Wartime Protestors
/// standing beside it — "whenever **another Ally** you control enters, put a
/// +1/+1 counter on that creature and it gains haste" — said nothing.
///
/// A permanent returning from exile is a permanent *entering the
/// battlefield* (CR 400.7 makes it a new object, and CR 603.6a has the
/// leaves/enters pair), so every watcher on the board is owed its trigger —
/// the one that returned it is not a private arrangement between two
/// objects. Aminatou's immediate flicker is the same claim on the other
/// path and is held by `a_blinked_permanent_comes_back_untapped`; this is
/// the *delayed* one, which runs out of `Engine::process_delayed` rather
/// than out of an effect.
#[test]
fn an_ally_returning_at_the_end_step_still_rallies_the_board() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(53, forest())
        .battlefield(
            0,
            &[
                venser_the_sojourner(),
                wartime_protestors(),
                great_divide_guide(),
                forest(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guide = on_battlefield(&engine, p0, great_divide_guide()).expect("the guide is out");
    assert_eq!(
        engine
            .state()
            .object(guide)
            .expect("the guide is on the battlefield")
            .counters
            .get(CounterKind::P1P1),
        0,
        "nothing has rallied yet"
    );

    activate(&mut engine, p0, venser_the_sojourner(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&guide), "the guide is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![guide],
                players: vec![],
            },
        )
        .expect("the guide is targeted");

    // Out to exile first: the return is a separate, delayed thing, and a
    // test that only watched the end result could not tell the two apart.
    pass_until(&mut engine, |e| {
        e.state()
            .object(guide)
            .is_some_and(|o| o.zone == Zone::Exile)
    });

    // Then the end step brings it back, and everything the return sets off
    // has to finish before the assertion.
    pass_until(&mut engine, |e| {
        e.state()
            .object(guide)
            .is_some_and(|o| o.zone == Zone::Battlefield)
            && e.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(e.pending(), Pending::Priority { .. })
    });

    assert_eq!(
        engine
            .state()
            .object(guide)
            .expect("the guide came back")
            .counters
            .get(CounterKind::P1P1),
        1,
        "the Protestors' rally trigger did not see the Ally come back"
    );
    assert!(
        keywords(&engine, guide).contains(KeywordSet::HASTE),
        "and the same trigger grants haste until end of turn"
    );
}

/// `Grist, the Hunger Tide` (`Coverage::Partial`):
/// "As long as Grist isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types.
/// +1: Create a 1/1 black and green Insect creature token, then mill a card. If an Insect card was milled this way,
/// put a loyalty counter on Grist and repeat this process.
/// −2: You may sacrifice a creature. When you do, destroy target creature or planeswalker.
/// −5: Each opponent loses life equal to the number of creature cards in your graveyard."
///
/// Under `Coverage::Partial`, the static zone animation, +1 loop, and −2 reflexive sacrifice are omitted,
/// leaving the −5 loyalty ability. With `Doubling Season` on the battlefield, `Grist, the Hunger Tide` enters
/// with 6 loyalty counters (doubling its starting loyalty of 3). Two creature cards are seeded into the graveyard
/// via `seed_graveyard`, and activating Grist's −5 ability reduces its loyalty to 1 and causes the opponent to lose 2 life.
#[test]
fn grist_the_hunger_tide_activates_minus_five_to_drain_life_for_graveyard_creatures() {
    let p0 = PlayerId::new(0);
    let doubling_season = card_index("01546b7d-a233-4176-8843-d732074dc5b6");
    let mut engine = Duel::new(103, quiet_creature())
        .battlefield(0, &[doubling_season, swamp(), forest(), swamp()])
        .hand(0, &[grist_the_hunger_tide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, grist_the_hunger_tide());
    pass_until(&mut engine, stack_is_empty);

    let grist = on_battlefield(&engine, p0, grist_the_hunger_tide()).expect("Grist on battlefield");
    assert_eq!(
        counters_on(&engine, grist, CounterKind::Loyalty),
        6,
        "Doubling Season doubled starting loyalty to 6"
    );

    seed_graveyard(&mut engine, p0, 2);

    activate(&mut engine, p0, grist_the_hunger_tide(), 0);
    assert_eq!(
        counters_on(&engine, grist, CounterKind::Loyalty),
        1,
        "paid 5 loyalty, leaving 1"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "opponent lost 2 life from 2 creature cards in graveyard"
    );
}

/// `Oko, Thief of Crowns` (`Coverage::Partial`):
/// "+2: Create a Food token.
/// +1: Target artifact or creature loses all abilities and becomes a green Elk creature with base power and toughness 3/3.
/// −5: Exchange control of target artifact or creature you control and target creature an opponent controls with power 3 or less."
///
/// Under `Coverage::Partial`, the +1 elk transformation and −5 control exchange are omitted.
/// The +2 ability is implemented. The test casts `Oko, Thief of Crowns` with 4 starting loyalty,
/// activates the +2 loyalty ability to tick Oko up to 6 loyalty, and confirms that a Food artifact token
/// is created under the player's control.
#[test]
fn oko_thief_of_crowns_ticks_up_and_creates_food_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, forest())
        .battlefield(0, &[forest(), island(), forest()])
        .hand(0, &[oko_thief_of_crowns()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, oko_thief_of_crowns());
    pass_until(&mut engine, stack_is_empty);

    let oko = on_battlefield(&engine, p0, oko_thief_of_crowns()).expect("Oko on battlefield");
    assert_eq!(
        counters_on(&engine, oko, CounterKind::Loyalty),
        4,
        "starts with 4 loyalty counters"
    );

    activate(&mut engine, p0, oko_thief_of_crowns(), 0);
    assert_eq!(
        counters_on(&engine, oko, CounterKind::Loyalty),
        6,
        "increased loyalty to 6"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one token was created");
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::ARTIFACT),
        "created token is an artifact"
    );
}

/// `Wrenn and Realmbreaker` (`Coverage::Partial`):
/// "Lands you control have '{T}: Add one mana of any color.'
/// +1: Up to one target land you control becomes a 3/3 Elemental creature with vigilance, hexproof, and haste until your next turn. It's still a land.
/// −2: Mill three cards. You may put a permanent card from among the milled cards into your hand.
/// −7: You get an emblem with 'You may play lands and cast permanent spells from your graveyard.'"
///
/// Under `Coverage::Partial`, the any-color land static, −2 milled-card selection, and −7 permanent spell graveyard permission are omitted.
/// The +1 land animation is implemented. The test casts `Wrenn and Realmbreaker`, activates its +1 ability targeting
/// a controlled `Forest`, verifies that Wrenn ticks up from 4 to 5 loyalty, and confirms that the targeted land becomes
/// a 3/3 Elemental creature retaining its land type with vigilance, hexproof, and haste.
#[test]
fn wrenn_and_realmbreaker_plus_one_animates_land_with_vigilance_hexproof_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(105, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[wrenn_and_realmbreaker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, wrenn_and_realmbreaker());
    pass_until(&mut engine, stack_is_empty);

    let wrenn =
        on_battlefield(&engine, p0, wrenn_and_realmbreaker()).expect("Wrenn on battlefield");
    assert_eq!(
        counters_on(&engine, wrenn, CounterKind::Loyalty),
        4,
        "starts with 4 loyalty counters"
    );

    let land = on_battlefield(&engine, p0, forest()).expect("Forest on battlefield");
    assert!(
        !types(&engine, land).contains(TypeSet::CREATURE),
        "target land is not initially a creature"
    );

    // The static grant is ability 0, in printed order, so the +1 is 1.
    activate(&mut engine, p0, wrenn_and_realmbreaker(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for +1 ability, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&land), "controlled land is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, wrenn, CounterKind::Loyalty),
        5,
        "loyalty ticked up to 5"
    );
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "target is still a land"
    );
    assert!(
        types(&engine, land).contains(TypeSet::CREATURE),
        "target became a creature"
    );
    assert_eq!(pt(&engine, land), (3, 3), "target is a 3/3");

    let kw = keywords(&engine, land);
    assert!(kw.contains(KeywordSet::VIGILANCE), "gained vigilance");
    assert!(kw.contains(KeywordSet::HEXPROOF), "gained hexproof");
    assert!(kw.contains(KeywordSet::HASTE), "gained haste");
}

/// What Grist does **not** offer, which is the other half of its
/// `Coverage::Partial` and the half a card test cannot see by playing.
///
/// Three of its four printed sentences are refused by name — the static that
/// animates it outside the battlefield, the `+1` mill loop, and the `-2`
/// reflexive sacrifice — and the test above proves the fourth. A card that
/// silently grew a half-written `+1` would pass that test unchanged, so the
/// refusal is pinned here instead: **one** loyalty ability is offered, and it
/// is the one that costs five.
///
/// This is a limitation written as a test rather than as a comment, so the
/// day the `+1` is implemented the build says so. Going red is the success.
#[test]
fn grist_offers_only_the_ability_that_is_written() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, quiet_creature())
        .battlefield(0, &[swamp(), forest(), swamp()])
        .hand(0, &[grist_the_hunger_tide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, grist_the_hunger_tide());
    pass_until(&mut engine, stack_is_empty);
    let grist = on_battlefield(&engine, p0, grist_the_hunger_tide()).expect("Grist arrived");

    // Five loyalty is more than the three it entered with, so the one ability
    // it has is not activatable yet — which is what makes this a statement
    // about the card and not about the board.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let mine: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(src, _)| *src == grist)
        .map(|(_, index)| *index)
        .collect();
    assert!(
        mine.is_empty(),
        "a -5 cannot be paid at three loyalty, so nothing of Grist's is \
         offered here: {mine:?}"
    );

    // And the pool's own view of the card: exactly one ability is written on
    // it, so the three refusals above are still refusals.
    let def = baylee_cards::by_index(grist_the_hunger_tide()).expect("Grist is in the pool");
    assert_eq!(
        def.abilities.len(),
        1,
        "Grist prints four sentences and this pool writes one of them — if \
         that number has moved, the `NOT SUPPORTED` notes on the card and \
         this test both need rereading"
    );
}

/// Wrenn and Realmbreaker prints `Lands you control have "{{T}}: Add one mana of any color."` and
/// `+1: Up to one target land you control becomes a 3/3 Elemental creature with vigilance, hexproof, and haste until your next turn. It's still a land.`
/// Under `Coverage::Partial`, the static mana grant and +1 animation are implemented, while the −2 selection and −7 permanent casting are omitted.
/// This test verifies that Wrenn starts with 4 loyalty, grants an any-color mana ability to controlled lands (used by `forest()` to produce black mana),
/// Wrenn and Realmbreaker prints "Lands you control have \"{T}: Add one mana
/// of any color.\"" — the same static Chromatic Lantern prints, and the half
/// this card silently dropped for as long as `Modifier::GrantActivated` was
/// listed as unenforced.
///
/// Both directions are asserted: a land under this seat is offered the
/// granted ability, and a land across the table is not. One of them alone
/// would pass on a grant that reached every land on the battlefield.
#[test]
fn wrenn_and_realmbreaker_grants_your_lands_any_colour_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wrenn_and_realmbreaker(), forest()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_forest = on_battlefield(&engine, p0, forest()).expect("Forest on battlefield");
    let opp_mountain =
        on_battlefield(&engine, p1, mountain()).expect("opponent Mountain on battlefield");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal
            .abilities
            .contains(&(my_forest, crate::choice::GRANTED_ABILITY)),
        "a land this seat controls is offered the granted any-colour ability"
    );
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(id, index)| *id == opp_mountain && *index == crate::choice::GRANTED_ABILITY),
        "and a land across the table is not — \"lands you control\""
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: my_forest,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .expect("the granted mana ability activates");

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5, "\"one mana of any color\" is all five");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "a Forest made black, which is the whole point of the sentence"
    );
    assert!(is_tapped(&engine, my_forest), "and it paid its own {{T}}");
}

/// The +1: "Up to one target land you control becomes a 3/3 Elemental
/// creature with vigilance, hexproof, and haste until your next turn. It's
/// still a land."
///
/// Ability index 1, because the static grant above is ability 0 — printed
/// order, the way Chromatic Lantern writes it.
#[test]
fn wrenn_and_realmbreaker_plus_one_animates_a_land_you_control() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wrenn_and_realmbreaker(), plains()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wrenn =
        on_battlefield(&engine, p0, wrenn_and_realmbreaker()).expect("Wrenn on battlefield");
    let my_plains = on_battlefield(&engine, p0, plains()).expect("Plains on battlefield");
    let opp_mountain =
        on_battlefield(&engine, p1, mountain()).expect("opponent Mountain on battlefield");
    assert_eq!(
        counters_on(&engine, wrenn, CounterKind::Loyalty),
        4,
        "she arrives on four"
    );

    activate(&mut engine, p0, wrenn_and_realmbreaker(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("\"target land you control\" asks: {:?}", engine.pending());
    };
    assert!(options.contains(&my_plains), "a land this seat controls");
    assert!(
        !options.contains(&opp_mountain),
        "and not one across the table"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![my_plains],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(
        counters_on(&engine, wrenn, CounterKind::Loyalty),
        5,
        "the +1 is paid before the ability resolves"
    );
    pass_until(&mut engine, stack_is_empty);

    let chars = engine
        .state()
        .object(my_plains)
        .expect("plains exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::LAND), "it's still a land");
    assert!(
        chars.types.contains(TypeSet::CREATURE),
        "and now a creature"
    );
    assert_eq!(pt(&engine, my_plains), (3, 3), "a 3/3");
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ELEMENTAL)
    );

    let kw = keywords(&engine, my_plains);
    assert!(kw.contains(KeywordSet::VIGILANCE), "vigilance");
    assert!(kw.contains(KeywordSet::HEXPROOF), "hexproof");
    assert!(kw.contains(KeywordSet::HASTE), "haste");
}

/// `Ashiok, Dream Render` is a legendary planeswalker costing `{1}{U/B}{U/B}` under `Coverage::Implemented`.
/// It enters with 5 loyalty counters and prints a −1 loyalty ability:
/// "Target player mills four cards. Then exile each opponent's graveyard."
/// Activating this ability targets an opponent via `Pending::ChoosePlayer`, reduces loyalty by 1,
/// mills four cards from the opponent's library, and exiles their entire graveyard.
#[test]
fn ashiok_dream_render_mills_target_player_and_exiles_opponents_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[ashiok_dream_render()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ashiok =
        on_battlefield(&engine, p0, ashiok_dream_render()).expect("Ashiok is on the battlefield");
    assert_eq!(
        counters_on(&engine, ashiok, CounterKind::Loyalty),
        5,
        "starts with 5 loyalty counters"
    );

    let p1_lib_before = library_size(&engine, p1);

    activate(&mut engine, p0, ashiok_dream_render(), 1);

    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChoosePlayer prompt for Ashiok, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&p1), "opponent is a legal target");

    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, ashiok, CounterKind::Loyalty),
        4,
        "loyalty decreased to 4"
    );
    assert_eq!(
        library_size(&engine, p1),
        p1_lib_before - 4,
        "target player milled four cards"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .is_empty(),
        "opponent's graveyard was completely exiled"
    );
    assert!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len() >= 4,
        "milled cards were moved to exile"
    );
}
