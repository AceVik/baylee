//! `cards/lands/manlands/hive_of_the_eye_tyrant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

/// Hive of the Eye Tyrant: "{3}{B}: Until end of turn, this land becomes a 3/3
/// black Beholder creature with menace and ... It's still a land."
///
/// Playing this land while controlling two or more other lands enters tapped,
/// and paying `{3}{B}` after untapping animates it into a 3/3 Beholder
/// creature with menace that is still a land.
#[test]
fn hive_of_the_eye_tyrant_animates_into_beholder_with_menace() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[hive_of_the_eye_tyrant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hive_of_the_eye_tyrant());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, hive_of_the_eye_tyrant(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 3));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(keywords(&engine, land).contains(KeywordSet::MENACE));
    assert!(!is_tapped(&engine, land));
    assert!(
        kit::colors_of(&engine, land).contains(baylee_core::color::Color::Black),
        "a black Beholder"
    );
}

/// "If you control two or more other lands, this land enters tapped."
///
/// The boundary is the count of *other* lands: with none or one the Hive
/// enters untapped, with two or three it enters tapped (the Hive does not
/// count itself). One game per count, because there is one land drop per turn.
#[test]
fn hive_of_the_eye_tyrant_enters_tapped_only_with_two_or_more_other_lands() {
    let p0 = PlayerId::new(0);
    for (others, tapped) in [(0_usize, false), (1, false), (2, true), (3, true)] {
        let board: Vec<CardIndex> = vec![swamp(); others];
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .hand(0, &[hive_of_the_eye_tyrant()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let land = play_land(&mut engine, p0, hive_of_the_eye_tyrant());
        pass_until(&mut engine, stack_is_empty);

        assert_eq!(
            entered_tapped(&engine, land),
            tapped,
            "with {others} other land(s) the Hive enters {}",
            if tapped { "tapped" } else { "untapped" }
        );
    }
}

/// "{T}: Add {B}."
///
/// The Hive, untapped and alone, taps for exactly one black mana and is then
/// tapped.
#[test]
fn hive_of_the_eye_tyrant_taps_for_one_black_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hive_of_the_eye_tyrant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, hive_of_the_eye_tyrant()).expect("the Hive");

    activate(&mut engine, p0, hive_of_the_eye_tyrant(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one mana");
    assert_eq!(pool.available(ManaColor::Black), 1, "and it is black");
    assert!(is_tapped(&engine, land), "tapping it is the cost");
}

/// A 3-seat table where player 0's Hive of the Eye Tyrant has been animated
/// and is about to attack. Every player has one card in their graveyard: the
/// attacker's Ogre, player 1's Elves and player 2's Elephant.
///
/// Returns the engine, the Hive and the three cards `(mine, first, second)`.
#[track_caller]
fn the_animated_hive_at_a_table_of_three() -> (Engine<RegistryLookup>, ObjectId, [ObjectId; 3]) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(
            0,
            &[hive_of_the_eye_tyrant(), swamp(), swamp(), swamp(), swamp()],
        )
        .hand(0, &[gray_ogre()])
        .hand(1, &[llanowar_elves()])
        .hand(2, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hive = on_battlefield(&engine, p0, hive_of_the_eye_tyrant()).expect("the Hive");
    let mine = hand_to_graveyard(&mut engine, p0, gray_ogre());
    let first = hand_to_graveyard(&mut engine, PlayerId::new(1), llanowar_elves());
    let second = hand_to_graveyard(&mut engine, PlayerId::new(2), wild_elephant());

    tap_mana_except(&mut engine, p0, hive);
    activate(&mut engine, p0, hive_of_the_eye_tyrant(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, hive),
        (3, 3),
        "a creature, and out since the turn began"
    );
    (engine, hive, [mine, first, second])
}

/// Attacks `defender` with the Hive and returns the menu of its attack
/// trigger's target question, as `(who is asked, objects, players, min, max)`.
#[track_caller]
fn attack_and_read_the_trigger_menu(
    engine: &mut Engine<RegistryLookup>,
    hive: ObjectId,
    defender: PlayerId,
) -> (PlayerId, Vec<ObjectId>, Vec<PlayerId>, u32, u32) {
    kit::declare_attack(engine, &[hive], defender);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the attack trigger asks for its target, got {:?}",
            engine.pending()
        )
    };
    (player, options, player_options, min, max)
}

/// "Whenever this creature attacks, exile target card from defending player's
/// graveyard."
///
/// At a table of three the Hive attacks player 1. Player 0's own graveyard and
/// player 2's both hold a card, and neither may be named: the only legal
/// target is the card in player 1's graveyard, the defending player's. Naming
/// it exiles it and touches nothing else.
#[test]
fn hive_of_the_eye_tyrant_attacking_player_one_may_only_target_player_ones_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, hive, [mine, first, second]) = the_animated_hive_at_a_table_of_three();

    let (asked, options, players, min, max) =
        attack_and_read_the_trigger_menu(&mut engine, hive, p1);

    assert_eq!(asked, p0, "the Hive's controller names the target");
    assert_eq!((min, max), (1, 1), "one target card");
    assert!(players.is_empty(), "a card, not a player: {players:?}");
    assert_eq!(
        options,
        vec![first],
        "only the defending player's graveyard is on the menu, not the \
         attacker's ({mine:?}) and not the other opponent's ({second:?})"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![first],
                players: vec![],
            },
        )
        .expect("the card was on the menu");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(kit::zone_of(&engine, first), Zone::Exile, "exiled");
    assert_eq!(
        kit::zone_of(&engine, mine),
        Zone::Graveyard,
        "mine untouched"
    );
    assert_eq!(
        kit::zone_of(&engine, second),
        Zone::Graveyard,
        "the player who was not attacked loses nothing"
    );
}

/// The other half of "defending player's": attacking player 2 puts player 2's
/// card on the menu and player 1's off it. The sentence follows whoever is
/// attacked, not whoever sits next in turn order.
#[test]
fn hive_of_the_eye_tyrant_attacking_player_two_may_only_target_player_twos_graveyard() {
    let (p0, p2) = (PlayerId::new(0), PlayerId::new(2));
    let (mut engine, hive, [_, first, second]) = the_animated_hive_at_a_table_of_three();

    let (asked, options, _, min, max) = attack_and_read_the_trigger_menu(&mut engine, hive, p2);

    assert_eq!(asked, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(
        options,
        vec![second],
        "player 2 is the defending player, so only their graveyard is offered \
         (player 1's card is {first:?})"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![second],
                players: vec![],
            },
        )
        .expect("the card was on the menu");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(kit::zone_of(&engine, second), Zone::Exile, "exiled");
    assert_eq!(
        kit::zone_of(&engine, first),
        Zone::Graveyard,
        "player 1 was not attacked and keeps their card"
    );
}

/// "…exile target card from defending player's graveyard": with nothing in the
/// defending player's graveyard the trigger has no legal target (CR 603.3d)
/// and is removed from the stack, so no question is asked — even though the
/// *other* opponent's graveyard holds a card.
#[test]
fn hive_of_the_eye_tyrant_with_an_empty_defending_graveyard_asks_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(
            0,
            &[hive_of_the_eye_tyrant(), swamp(), swamp(), swamp(), swamp()],
        )
        .hand(2, &[wild_elephant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hive = on_battlefield(&engine, p0, hive_of_the_eye_tyrant()).expect("the Hive");
    let elsewhere = hand_to_graveyard(&mut engine, PlayerId::new(2), wild_elephant());
    tap_mana_except(&mut engine, p0, hive);
    activate(&mut engine, p0, hive_of_the_eye_tyrant(), 1);
    pass_until(&mut engine, stack_is_empty);

    kit::declare_attack(&mut engine, &[hive], p1);

    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "player 1's graveyard is empty, so there is no target to ask for: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        kit::zone_of(&engine, elsewhere),
        Zone::Graveyard,
        "player 2's card is not the defending player's and stays"
    );
}
