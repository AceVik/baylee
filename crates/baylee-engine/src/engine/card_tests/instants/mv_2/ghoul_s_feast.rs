//! `cards/instants/mv_2/ghoul_s_feast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ghoul's Feast (`Coverage::Implemented`): "Target creature gets +X/+0 until
/// end of turn, where X is the number of creature cards in your graveyard."
///
/// The library filler is `quiet_creature()` (Llanowar Elves), so one card
/// seeded from the library lands as a creature card in p0's graveyard, giving
/// X = 1. A land card seeded into p1's graveyard confirms "your graveyard"
/// is read and the opponent's creatures are not counted. Power rises by 1,
/// toughness stays unchanged.
#[test]
fn ghouls_feast_pumps_a_creature_by_the_count_of_your_graveyard_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // Using quiet_creature() as the library filler so seeded graveyard cards
    // are creature cards, making X = 1 when one is seeded for p0.
    let mut engine = Duel::new(43, quiet_creature())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[ghoul_s_feast()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    assert_eq!(pt(&engine, target), (1, 1), "a 1/1 before the feast");

    // Put one creature card into p0's graveyard so X = 1.
    seed_graveyard(&mut engine, p0, 1);
    // Put a card into p1's graveyard too; it is also a creature card but it
    // is not in *p0's* graveyard, so the spell must not count it.
    seed_graveyard(&mut engine, p1, 1);

    cast_from_hand(&mut engine, p0, ghoul_s_feast());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Feast asks for a target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&target),
        "the Elf is a legal creature target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, target),
        (2, 1),
        "+1/+0 for the one creature card in p0's graveyard; \
         the card in p1's graveyard is not counted"
    );
}
