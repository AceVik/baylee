//! `cards/creatures/mv_5/kookus.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn kookus() -> CardIndex {
    card_index("4c8196e0-c392-4402-b77c-e3d009e9899c")
}

fn keeper_of_kookus() -> CardIndex {
    card_index("97417270-e525-4261-b7d9-0dde3360e666")
}

/// Six Mountains and Kookus in the hand; a Keeper of Kookus on the board of
/// `keeper_owner` (0 is the Djinn's controller) if there is one. Kookus is
/// cast from the hand so the first upkeep after it is the controller's second
/// turn, with the Djinn free to attack.
fn cast_kookus(keeper_owner: Option<usize>) -> Engine<RegistryLookup> {
    let mut mine = vec![mountain(); 6];
    let mut theirs = vec![];
    match keeper_owner {
        Some(0) => mine.push(keeper_of_kookus()),
        Some(_) => theirs.push(keeper_of_kookus()),
        None => {}
    }
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &mine)
        .battlefield(1, &theirs)
        .hand(0, &[kookus()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    // Six Mountains float {R}; the Djinn costs five and the sixth is left for
    // its own ability.
    tap_all_mana(&mut engine, PlayerId::new(0));
    cast_with_floating(&mut engine, PlayerId::new(0), kookus());
    pass_until(&mut engine, stack_is_empty);
    engine
}

fn attack_question(engine: &mut Engine<RegistryLookup>) -> (Vec<ObjectId>, Vec<ObjectId>) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        required,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    (attackers, required)
}

/// `Kookus` ({3}{R}{R}, 3/5 Djinn, `Coverage::Implemented`):
/// `Trample. At the beginning of your upkeep, if you don't control a creature
/// named Keeper of Kookus, this creature deals 3 damage to you and attacks this
/// turn if able. {R}: This creature gets +1/+0 until end of turn.`
///
/// The firebreathing ability is activated at index 1 (the upkeep trigger is
/// index 0), costs the {R} left floating, and ends with the turn.
#[test]
fn kookus_firebreathing_gives_plus_one_power_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cast_kookus(None);
    let djinn = on_battlefield(&engine, p0, kookus()).unwrap();
    assert!(keywords(&engine, djinn).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, djinn), (3, 5));
    assert_eq!(
        engine.state().players[0].life,
        20,
        "no upkeep has begun: casting it hurts nobody"
    );

    activate(&mut engine, p0, kookus(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, djinn), (4, 5), "one {{R}} is one +1/+0");

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, djinn),
        (3, 5),
        "\"until end of turn\" ended with the turn"
    );
}

/// The upkeep trigger on the controller's own upkeep, every time: 3 damage to
/// the controller, and the Djinn is listed as a required attacker, so an empty
/// declaration is refused; it then attacks and the opponent takes 3. The
/// opponent's upkeep does nothing.
#[test]
fn kookus_hurts_its_controller_and_must_attack_on_each_of_their_upkeeps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cast_kookus(None);
    let djinn = on_battlefield(&engine, p0, kookus()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (20, 20),
        "the opponent's upkeep is not \"your upkeep\""
    );

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        17,
        "3 damage to its controller at their upkeep"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and not to the opponent"
    );

    let (able, required) = attack_question(&mut engine);
    assert!(able.contains(&djinn), "the Djinn is able to attack");
    assert_eq!(required, vec![djinn], "\"attacks this turn if able\"");
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "an empty declaration leaves the required attacker at home"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(djinn, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "it attacked and, unblocked, dealt its 3"
    );

    // Each of the controller's upkeeps, not only the first one.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        14,
        "3 more at the next upkeep"
    );
    let (_, required) = attack_question(&mut engine);
    assert_eq!(required, vec![djinn], "required again");
}

/// The "if you don't control a creature named Keeper of Kookus" clause: with
/// one of your own on the battlefield the trigger does nothing at all, and the
/// Djinn may stay at home.
#[test]
fn kookus_with_their_own_keeper_of_kookus_deals_no_damage_and_is_free_to_stay_home() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cast_kookus(Some(0));
    let djinn = on_battlefield(&engine, p0, kookus()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the Keeper of Kookus switches the damage off"
    );
    let (able, required) = attack_question(&mut engine);
    assert!(able.contains(&djinn), "the Djinn is able to attack");
    assert!(
        required.is_empty(),
        "and nothing forces it to: {required:?}"
    );
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
}

/// "You don't control": a Keeper of Kookus on the opponent's side does not
/// switch anything off.
#[test]
fn kookus_still_hurts_and_forces_an_attack_when_only_the_opponent_has_a_keeper_of_kookus() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cast_kookus(Some(1));
    let djinn = on_battlefield(&engine, p0, kookus()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        17,
        "the opponent's Keeper does not count as yours"
    );
    let (_, required) = attack_question(&mut engine);
    assert_eq!(required, vec![djinn]);
}
