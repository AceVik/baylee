//! `cards/creatures/mv_2/tomik_izzet_sparkmage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tomik_izzet_sparkmage() -> CardIndex {
    card_index("16604fee-cd8d-41e4-8269-3c7ff756613d")
}

fn fanatical_firebrand() -> CardIndex {
    card_index("d36e11f1-6ab3-4273-8114-a8fbbe21c1c3")
}

fn slickshot_show_off() -> CardIndex {
    card_index("74c7a029-59aa-4511-8e4c-f8f167cda4e2")
}

fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

fn act_of_treason() -> CardIndex {
    card_index("9d08af23-9f4a-4097-9abc-3b17475ab744")
}

fn aim_at_player(engine: &mut Engine<RegistryLookup>, seat: PlayerId, target: PlayerId) {
    let Pending::ChooseTargets {
        player,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, seat, "the acting seat names the target");
    assert_eq!((min, max), (1, 1), "one target");
    assert!(
        player_options.contains(&target),
        "target player {target:?} must be on the menu: {player_options:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .expect("the named player was on the menu");
}

/// Tomik, Izzet Sparkmage (`{1}{R}` 1/2 Legendary Human Wizard):
/// "If a source you control would deal noncombat damage to an opponent or a
/// permanent an opponent controls, it deals that much damage plus 1 instead."
///
/// When Fanatical Firebrand is sacrificed to pay for its own ability, the
/// Firebrand is still the source of the damage and its last known information
/// (including its controller) is used (CR 113.7a, CR 608.2h, CR 609.7).
/// Tomik's replacement effect applies (CR 614.1a), so the 1 damage becomes 2
/// and kills the opponent's 1/2 Slickshot Show-Off.
#[test]
fn tomik_adds_one_damage_to_sacrificed_firebrand_targeting_opponent_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tomik_izzet_sparkmage(), fanatical_firebrand()])
        .battlefield(1, &[slickshot_show_off()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let slickshot =
        on_battlefield(&engine, p1, slickshot_show_off()).expect("opponent's 1/2 on battlefield");
    assert_eq!(pt(&engine, slickshot), (1, 2));

    activate(&mut engine, p0, fanatical_firebrand(), 0);
    aim_at(&mut engine, p0, slickshot);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, fanatical_firebrand()).is_some(),
        "Firebrand was sacrificed to its own ability"
    );
    assert!(
        on_battlefield(&engine, p1, slickshot_show_off()).is_none(),
        "2 damage (1 + 1 from Tomik) is lethal to a 1/2"
    );
    assert!(
        in_graveyard(&engine, p1, slickshot_show_off()).is_some(),
        "opponent's 1/2 should die from 2 damage"
    );
}

/// Sacrificed Fanatical Firebrand targeting the opponent directly also gets
/// Tomik's +1 replacement (CR 113.7a, CR 608.2h, CR 609.7, CR 614.1a).
#[test]
fn tomik_adds_one_damage_to_sacrificed_firebrand_targeting_opponent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tomik_izzet_sparkmage(), fanatical_firebrand()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, fanatical_firebrand(), 0);
    aim_at_player(&mut engine, p0, p1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, fanatical_firebrand()).is_some(),
        "Firebrand was sacrificed to its own ability"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "1 damage + 1 from Tomik = 2 damage to the opponent"
    );
    assert_eq!(engine.state().players[0].life, 20);
}

/// Casting Shock at the opponent with Tomik on the battlefield deals 3 damage
/// (2 + 1 from Tomik) and triggers Tomik's prowess.
#[test]
fn tomik_adds_one_damage_to_shock_targeting_opponent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tomik_izzet_sparkmage(), mountain()])
        .hand(0, &[shock()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tomik = on_battlefield(&engine, p0, tomik_izzet_sparkmage()).expect("Tomik on battlefield");
    assert_eq!(pt(&engine, tomik), (1, 2));

    cast_from_hand(&mut engine, p0, shock());
    aim_at_player(&mut engine, p0, p1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        17,
        "Shock's 2 damage + 1 from Tomik = 3 damage to the opponent"
    );
    assert_eq!(
        pt(&engine, tomik),
        (2, 3),
        "casting Shock also triggers Tomik's prowess"
    );
}

/// Casting Tomik and Fanatical Firebrand from hand and activating the
/// Firebrand on the same turn (it has haste) kills an opponent's 1/2
/// Slickshot Show-Off with 2 damage.
#[test]
fn tomik_and_firebrand_cast_from_hand_kill_opponent_1_2() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[tomik_izzet_sparkmage(), fanatical_firebrand()])
        .battlefield(1, &[slickshot_show_off()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tomik_izzet_sparkmage());
    pass_until(&mut engine, stack_is_empty);
    cast_from_hand(&mut engine, p0, fanatical_firebrand());
    pass_until(&mut engine, stack_is_empty);

    let slickshot =
        on_battlefield(&engine, p1, slickshot_show_off()).expect("opponent's 1/2 on battlefield");
    activate(&mut engine, p0, fanatical_firebrand(), 0);
    aim_at(&mut engine, p0, slickshot);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, fanatical_firebrand()).is_some(),
        "Firebrand was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p1, slickshot_show_off()).is_some(),
        "2 damage (1 + 1 from Tomik) kills Slickshot Show-Off"
    );
}

/// Stealing the opponent's Fanatical Firebrand with Act of Treason and
/// sacrificing it to its own ability puts the card in the *opponent's*
/// graveyard (owner = p1), so only last known information (CR 113.7a,
/// CR 608.2h, CR 609.7) knows p0 controlled it when it left the battlefield.
#[test]
fn tomik_adds_one_damage_to_stolen_sacrificed_firebrand_via_lki() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[tomik_izzet_sparkmage(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[act_of_treason()])
        .battlefield(1, &[fanatical_firebrand(), slickshot_show_off()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let firebrand = on_battlefield(&engine, p1, fanatical_firebrand()).expect("p1's Firebrand");
    let slickshot = on_battlefield(&engine, p1, slickshot_show_off()).expect("p1's Slickshot");

    cast_from_hand(&mut engine, p0, act_of_treason());
    aim_at(&mut engine, p0, firebrand);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, fanatical_firebrand()).is_some(),
        "p0 now controls the stolen Firebrand"
    );

    activate(&mut engine, p0, fanatical_firebrand(), 0);
    aim_at(&mut engine, p0, slickshot);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, fanatical_firebrand()).is_some(),
        "stolen Firebrand goes to its owner's (p1's) graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, slickshot_show_off()).is_none()
            && in_graveyard(&engine, p1, slickshot_show_off()).is_some(),
        "Tomik must read Firebrand's LKI controller (p0), dealing 2 damage and killing the 1/2"
    );
}

/// Seat 1 controlling Tomik and Fanatical Firebrand also gets Tomik's +1 when
/// sacrificing Firebrand to target seat 0's 1/2 creature.
#[test]
fn tomik_on_seat_one_adds_one_damage_to_sacrificed_firebrand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[slickshot_show_off()])
        .battlefield(1, &[tomik_izzet_sparkmage(), fanatical_firebrand()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);

    let slickshot =
        on_battlefield(&engine, p0, slickshot_show_off()).expect("p0's 1/2 on battlefield");
    activate(&mut engine, p1, fanatical_firebrand(), 0);
    aim_at(&mut engine, p1, slickshot);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, fanatical_firebrand()).is_some(),
        "Firebrand was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, slickshot_show_off()).is_some(),
        "2 damage (1 + 1 from Tomik) kills p0's 1/2"
    );
}
