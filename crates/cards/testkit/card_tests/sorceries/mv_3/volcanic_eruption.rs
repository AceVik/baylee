//! `cards/sorceries/mv_3/volcanic_eruption.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Volcanic Eruption — "Destroy X target Mountains. Volcanic Eruption deals
/// damage to each creature and each player equal to the number of Mountains
/// put into a graveyard this way." Two targets, one of them with a
/// regeneration shield: one Mountain goes, so the damage is 1 — the Elves
/// die, the Bears live, and each player loses 1. A count of the targets
/// would have dealt 2.
#[test]
fn volcanic_eruption_deals_the_number_of_mountains_that_reached_a_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let eruption = card_index("4c15889e-3172-413b-b805-a2b7ad05f636");
    let bears = card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0");
    let mut engine = Duel::new(112, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), bears],
        )
        .hand(0, &[eruption])
        .battlefield(1, &[mountain(), mountain(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mountains = all_on_battlefield(&engine, p1, mountain());
    let (shielded, doomed) = (mountains[0], mountains[1]);
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(shielded)
        .expect("seated")
        .regeneration_shields = 1;
    let bear = on_battlefield(&engine, p0, bears).expect("the Bears are out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elves are out");
    let life = |e: &Engine<RegistryLookup>| [e.state().players[0].life, e.state().players[1].life];
    let before = life(&engine);

    cast_from_hand(&mut engine, p0, eruption);
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![shielded, doomed],
            },
        )
        .expect("X = 2 names both Mountains");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(doomed).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(
        engine.state().object(shielded).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the shield saved it"
    );
    assert_eq!(
        life(&engine),
        [before[0] - 1, before[1] - 1],
        "1 to each player"
    );
    assert_eq!(
        engine.state().object(elf).map(|o| o.zone),
        Some(Zone::Graveyard),
        "1 damage kills the Elves"
    );
    assert_eq!(
        engine.state().object(bear).map(|o| (o.zone, o.damage)),
        Some((Zone::Battlefield, 1)),
        "and marks 1 on the Bears"
    );
}
