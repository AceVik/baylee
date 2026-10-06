//! `cards/creatures/mv_2/rats_of_rath.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rats of Rath prints one line — "{B}: Destroy target artifact, creature, or
/// land you control" — and the two things worth playing are the three-type
/// `Or` and the "you control" that closes it. The menu is read *before* it is
/// answered, because CR 601.2c names the target while the {B} is still
/// floating and the Sol Ring is still standing, so a board holding all three
/// types under this seat and both of them across the table says the filter is
/// read rather than assumed. The offer is read twice — absent on an empty pool
/// and present once the mana is floating — because `legal.abilities` is
/// filtered through `can_afford`, which reads the pool and not the untapped
/// lands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn rats_of_rath_destroys_a_permanent_you_control_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                forest(),
                quiet_artifact(),
                llanowar_elves(),
                rats_of_rath(),
            ],
        )
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rats = on_battlefield(&engine, p0, rats_of_rath()).expect("the Rats are on the table");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let my_land = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    assert_eq!(pt(&engine, rats), (2, 1), "and it is the body it prints");

    // An empty pool pays no {B}, so the line is absent rather than refused.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(rats, 0)),
        "nothing floats, so the {{B}} is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the Swamp's {{B}}; the Forest, the Elves and the Sol Ring paid in \
         beside it, which is why only the colour is asserted"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(rats, 0)),
        "with {{B}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, rats_of_rath(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact, creature, or land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        options.contains(&rock) && options.contains(&mine) && options.contains(&my_land),
        "all three printed types are on the menu — the artifact, the creature \
         and the land: {options:?}"
    );
    assert!(
        !options.contains(&their_elf) && !options.contains(&their_land),
        "\"you control\" is not \"a permanent\": neither of the opponent's is \
         a legal target: {options:?}"
    );
    assert_eq!(
        options.len(),
        5,
        "every permanent this seat has — Swamp and Forest as lands, Elves and \
         Rats as creatures, Sol Ring as an artifact — and nothing across the \
         table: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "CR 601.2c names the target and CR 601.2h pays afterwards, so nothing \
         has died while the question still stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("the Sol Ring was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the ability named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "and it left the battlefield to get there"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some()
            && on_battlefield(&engine, p0, rats_of_rath()).is_some(),
        "one target, one permanent: the creature and the Rats beside it were \
         not named and never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some()
            && on_battlefield(&engine, p1, forest()).is_some(),
        "and the opponent's board is exactly where it was"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0,
        "the {{B}} is the price and it came out of the pool"
    );
}
