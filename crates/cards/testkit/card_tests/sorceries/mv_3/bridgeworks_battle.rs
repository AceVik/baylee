//! `cards/sorceries/mv_3/bridgeworks_battle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Bridgeworks Battle` // `Tanglespan Bridgeworks`: "Target creature you
/// control gets +2/+2 until end of turn. It fights up to one target creature
/// you don't control."
///
/// The first menu is my creatures only, the second theirs only and "up to
/// one" (`min` 0). A 1/1 of mine is pumped to 3/3 and fights their 1/1: theirs
/// dies, mine keeps one damage — at the pumped size, which is the resolution
/// seeing its own pump. The declined and the gone-in-response halves are in
/// `fight_tests`.
#[test]
fn bridgeworks_battle_pumps_my_creature_and_it_fights_theirs() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(474, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[bridgeworks_battle()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 has an elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("p1 has an elf");

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_front_face(&mut engine, p0, bridgeworks_battle());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&my_elf),
        "controlled creature is a legal target"
    );
    assert!(
        !options.contains(&their_elf),
        "opponent creature cannot be pumped"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![my_elf],
                players: vec![],
            },
        )
        .unwrap();

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the fight's target prompt, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![their_elf],
        "only a creature I don't control is fought"
    );
    assert_eq!((min, max), (0, 1), "\"up to one\"");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_elf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, my_elf), (3, 3), "+2/+2 until end of turn");
    assert_eq!(
        engine.state().object(my_elf).map(|o| o.damage),
        Some(1),
        "the 1/1 across the table dealt its one back"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "three damage from the pumped Elves kill theirs"
    );
    assert!(
        in_graveyard(&engine, p0, bridgeworks_battle()).is_some(),
        "Bridgeworks Battle moves to graveyard after resolution"
    );
}
