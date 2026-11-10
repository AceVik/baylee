//! `cards/sorceries/mv_2/sinkhole.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sinkhole costs {B}{B} and prints one sentence: "Destroy target land."
///
/// The word worth playing is "land", so the board carries one of each thing
/// the filter could have widened into: a Forest across the table with an Elf
/// standing beside it, which a bare `Filter::Any` would have offered as a
/// target too. Both lands *are* offered — "target land" reaches either side of
/// the table — and the permanent that is not named has to be exactly where it
/// was afterwards, which is what separates a destroy aimed at one target from
/// a spell that wrecked the board it landed on.
#[test]
fn sinkhole_destroys_the_land_it_names_and_leaves_the_rest_of_the_table_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(0, &[sinkhole()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_land = on_battlefield(&engine, p0, swamp()).expect("my Swamp is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // The two Swamps pay the {B}{B} before the offer is read, because
    // `legal`/the target list is what the pool funds and not what the
    // untapped lands could have funded.
    cast_from_hand(&mut engine, p0, sinkhole());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert!(
        options.contains(&my_land) && options.contains(&their_land),
        "\"target land\" reaches either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "an Elf is a creature and no land, so it may not be named: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("the Forest was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "the land the spell named left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "and a destroyed land goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, swamp()).is_some(),
        "the land the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "nor did the creature standing beside it"
    );
    assert!(
        in_graveyard(&engine, p0, sinkhole()).is_some(),
        "and the sorcery itself is in its caster's graveyard"
    );
}
