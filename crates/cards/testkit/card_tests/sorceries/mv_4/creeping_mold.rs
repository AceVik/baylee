//! `cards/sorceries/mv_4/creeping_mold.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Creeping Mold prints one sentence — "Destroy target artifact, enchantment,
/// or land" — and the word "or" is what the board has to be built around: an
/// artifact, an enchantment and a land stand on *both* sides of the table, so
/// the menu has to offer all three printed kinds and neither seat is
/// privileged by the filter, while a creature of mine and one across the table
/// must stay off it. The land is the target worth naming, because a card that
/// had lost its third disjunct would still offer the artifact beside it and
/// still leave every creature standing — which is exactly what an
/// artifact-only removal spell looks like from the outside.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn creeping_mold_offers_every_artifact_enchantment_and_land_and_never_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                quiet_artifact(),
                exploration(),
                llanowar_elves(),
            ],
        )
        .battlefield(
            1,
            &[quiet_artifact(), exploration(), forest(), llanowar_elves()],
        )
        .hand(0, &[creeping_mold()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_rock = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let my_chant = on_battlefield(&engine, p0, exploration()).expect("my Exploration is out");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let my_lands = lands_of(&engine, p0);
    assert_eq!(my_lands.len(), 4, "four Forests under p0");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_chant = on_battlefield(&engine, p1, exploration()).expect("their Exploration is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // The whole board is tapped for the {2}{G}{G}, so nothing here rests on a
    // mana the spell never paid.
    cast_from_hand(&mut engine, p0, creeping_mold());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    for (id, what) in [
        (my_rock, "my artifact"),
        (my_chant, "my enchantment"),
        (their_rock, "their artifact"),
        (their_chant, "their enchantment"),
        (their_land, "their land"),
    ] {
        assert!(
            options.contains(&id),
            "{what} is one of the three printed kinds: {options:?}"
        );
    }
    for land in &my_lands {
        assert!(
            options.contains(land),
            "a land of mine is a land, whichever side of the table it stands on: {options:?}"
        );
    }
    assert!(
        !options.contains(&my_elf),
        "a creature is none of the three: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "and neither is one across the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("the land was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "the land that was named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "and it left the battlefield, which is the destroy itself"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        4,
        "the four lands nobody named are all still standing"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some()
            && on_battlefield(&engine, p0, exploration()).is_some()
            && on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "nothing else under p0 moved"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some()
            && on_battlefield(&engine, p1, exploration()).is_some()
            && on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and nothing else under p1 either: one target, one permanent"
    );
    assert!(
        in_graveyard(&engine, p0, creeping_mold()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
}
