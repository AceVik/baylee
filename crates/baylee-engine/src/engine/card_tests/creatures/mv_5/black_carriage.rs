//! Black Carriage — `{3}{B}{B}` 4/4 Creature — Horse (Homelands)
//!
//! Trample
//! Black Carriage doesn't untap during your untap step.
//! Sacrifice a creature: Untap Black Carriage. Activate only during your
//! upkeep.

#[allow(clippy::wildcard_imports)]
use super::*;

fn black_carriage() -> CardIndex {
    card_index("6bf5dcf1-5c93-42f4-adaf-a6f2e5807596")
}

#[test]
fn black_carriage_stays_tapped_at_untap_step_and_untaps_by_sacrificing_creature_in_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[black_carriage(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let carriage = on_battlefield(&engine, p0, black_carriage()).unwrap();
    let fodder = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, carriage), (4, 4));
    assert!(keywords(&engine, carriage).contains(KeywordSet::TRAMPLE));
    assert!(
        !ability_offered(&engine, carriage, 1),
        "Black Carriage's untap ability can only be activated during your upkeep"
    );

    // Attack with Black Carriage so it taps.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(carriage, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(is_tapped(&engine, carriage));
    assert_eq!(engine.state().players[1].life, 16);

    // "Activate only during your upkeep": in the opponent's upkeep p0 gets
    // priority too, with the Carriage tapped and a creature to sacrifice, and
    // the ability is not offered.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "p0 holds priority in p1's upkeep: {:?}",
        engine.pending()
    );
    assert!(
        !ability_offered(&engine, carriage, 1),
        "Black Carriage's ability is not offered in the opponent's upkeep"
    );

    // Advance to p0's next upkeep: Black Carriage did not untap during p0's untap step.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        is_tapped(&engine, carriage),
        "Black Carriage doesn't untap during your untap step"
    );
    assert!(
        ability_offered(&engine, carriage, 1),
        "Black Carriage's ability is offered during p0's upkeep"
    );

    // Activate ability 1 sacrificing Grizzly Bears -> Black Carriage untaps.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: carriage,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(fodder).unwrap().zone, Zone::Graveyard);
    assert!(!is_tapped(&engine, carriage));
}
