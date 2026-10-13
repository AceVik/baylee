//! `cards/lands/manlands/restless_fortress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

/// Restless Fortress: "This land enters tapped." / "{2}{W}{B}: This land
/// becomes a 1/4 white and black Nightmare creature until end of turn. It's
/// still a land."
///
/// Playing this land causes it to enter tapped, and paying `{2}{W}{B}` after
/// untapping animates it into a 1/4 white and black Nightmare creature that is
/// still a land.
#[test]
fn restless_fortress_animates_into_nightmare() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(313, forest())
        .battlefield(0, &[plains(), plains(), swamp(), swamp()])
        .hand(0, &[restless_fortress()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_fortress());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_fortress(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (1, 4));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
    let colors = kit::colors_of(&engine, land);
    assert!(
        colors.contains(baylee_core::color::Color::White)
            && colors.contains(baylee_core::color::Color::Black),
        "a white and black Nightmare"
    );
    assert!(
        kit::has_subtype(
            &engine,
            land,
            baylee_core::generated::subtypes::creature::NIGHTMARE
        ),
        "a Nightmare"
    );
}

/// "{2}{W}{B}: This land becomes a 1/4 … creature until end of turn."
///
/// The animation ends with the turn: on the opponent's turn the Fortress is a
/// plain land again, no creature and no 1/4.
#[test]
fn restless_fortress_stops_being_a_creature_at_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[restless_fortress(), plains(), plains(), swamp(), swamp()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, restless_fortress()).expect("the Fortress");
    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_fortress(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(land)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "a creature this turn"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.state().turn.phase, Phase::FirstMain)
    });

    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(
        !types.contains(TypeSet::CREATURE),
        "the animation lasted until end of turn only"
    );
    assert!(types.contains(TypeSet::LAND), "and it is a land throughout");
}

/// "{T}: Add {W} or {B}."
///
/// Once for each colour: the Fortress taps for exactly one mana of the colour
/// chosen, and is tapped afterwards.
#[test]
fn restless_fortress_taps_for_white_or_black() {
    let p0 = PlayerId::new(0);
    for colour in [ManaColor::White, ManaColor::Black] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[restless_fortress()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let land = on_battlefield(&engine, p0, restless_fortress()).expect("the Fortress");

        activate(&mut engine, p0, restless_fortress(), 0);
        if let Pending::ChooseColor { options, .. } = engine.pending().clone() {
            assert_eq!(
                options.len(),
                2,
                "white or black, nothing else: {options:?}"
            );
            assert!(
                options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
                "{options:?}"
            );
            engine
                .apply(p0, PlayerAction::ChooseColor(colour))
                .expect("the colour was offered");
        }

        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(pool.total(), 1, "one mana");
        assert_eq!(pool.available(colour), 1, "of the colour chosen");
        assert!(is_tapped(&engine, land), "tapping it is the cost");
    }
}

/// A 3-seat table where player 0's Restless Fortress has been animated and is
/// about to attack, player 0 at 20 life, both opponents at 20.
#[track_caller]
fn the_animated_fortress_at_a_table_of_three() -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(
            0,
            &[restless_fortress(), plains(), plains(), swamp(), swamp()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, restless_fortress()).expect("the Fortress");
    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_fortress(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, land),
        (1, 4),
        "a creature, out since the turn began"
    );
    (engine, land)
}

/// Attacks `defender` with the Fortress and lets the attack trigger resolve,
/// stopping before blockers so that only the trigger has touched the life
/// totals.
#[track_caller]
fn attack_and_resolve_the_trigger(
    engine: &mut Engine<RegistryLookup>,
    land: ObjectId,
    defender: PlayerId,
) {
    kit::declare_attack(engine, &[land], defender);
    pass_until(engine, stack_is_empty);
}

/// "Whenever this land attacks, defending player loses 2 life and you gain 2
/// life."
///
/// At a table of three the Fortress attacks player 1: player 1 — and nobody
/// else — loses 2, and the Fortress' controller gains 2. Player 2 was not
/// attacked and is not the defending player.
#[test]
fn restless_fortress_attacking_player_one_drains_player_one_only() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let (mut engine, land) = the_animated_fortress_at_a_table_of_three();

    attack_and_resolve_the_trigger(&mut engine, land, p1);

    assert_eq!(kit::life(&engine, p1), 18, "the defending player loses 2");
    assert_eq!(
        kit::life(&engine, p2),
        20,
        "the other opponent is not the defending player"
    );
    assert_eq!(kit::life(&engine, p0), 22, "and its controller gains 2");
}

/// The same sentence, the other way round: attacking player 2 drains player 2
/// and leaves player 1 alone. "Defending player" is whoever the Fortress
/// attacks, not the next player in turn order.
#[test]
fn restless_fortress_attacking_player_two_drains_player_two_only() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let (mut engine, land) = the_animated_fortress_at_a_table_of_three();

    attack_and_resolve_the_trigger(&mut engine, land, p2);

    assert_eq!(kit::life(&engine, p2), 18, "the defending player loses 2");
    assert_eq!(kit::life(&engine, p1), 20, "player 1 was not attacked");
    assert_eq!(kit::life(&engine, p0), 22, "and its controller gains 2");
}

/// The trigger is a loss of life, not damage, and it is on top of the combat
/// damage a 1/4 unblocked still deals: attacking player 1 and not being
/// blocked leaves player 1 at 20 - 2 - 1 and the controller at 22.
#[test]
fn restless_fortress_drain_comes_on_top_of_its_combat_damage() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let (mut engine, land) = the_animated_fortress_at_a_table_of_three();

    kit::declare_attack(&mut engine, &[land], p1);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(
        kit::life(&engine, p1),
        17,
        "2 lost to the trigger, 1 dealt in combat"
    );
    assert_eq!(kit::life(&engine, p2), 20);
    assert_eq!(kit::life(&engine, p0), 22);
}
