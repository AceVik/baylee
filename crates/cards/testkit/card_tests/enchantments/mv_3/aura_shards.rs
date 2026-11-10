//! `cards/enchantments/mv_3/aura_shards.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aura Shards — {1}{G}{W} enchantment: "Whenever a creature you control
/// enters, you may destroy target artifact or enchantment."
///
/// `pass_until` answers the "you may" with yes on the way past, so what is
/// left to read is the target half: the menu has to hold every artifact and
/// enchantment on the table — the Shards themselves, the Sol Ring beside them
/// and the Luminarch Ascension across it — while the Llanowar Elves whose
/// entry caused the trigger is a creature and must be absent from it. The
/// destruction is then read in the graveyard rather than off the battlefield,
/// because "destroy" is the word the card prints, and an effect that merely
/// stopped being a permanent would satisfy a board check.
#[test]
fn aura_shards_destroys_the_artifact_or_enchantment_its_controller_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                aura_shards(),
                forest(),
                forest(),
                plains(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[their_enchantment()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let shards = on_battlefield(&engine, p0, aura_shards()).expect("the Shards are out");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    let theirs =
        on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");

    // The creature enters and the trigger asks; the target question is what
    // the walk stops on, with the "you may" already answered yes behind it.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves landed");
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the target question")
    };
    assert_eq!(
        player, p0,
        "the Shards' controller answers their own trigger"
    );
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        options.contains(&shards) && options.contains(&rock) && options.contains(&theirs),
        "\"target artifact or enchantment\" is any of the three on the table — \
         the Shards are an enchantment themselves and the Ascension stands \
         across it: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "the creature whose entry caused the trigger is a creature and not \
         something the filter names: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "and those three are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the enchantment across the table was one of the options");
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, their_enchantment()).is_some()
    });

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the chosen permanent left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "\"destroy\" puts it in its owner's graveyard, which is p1's and not \
         the Shards' controller's"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the permanent the ability did not name is untouched"
    );
    assert!(
        on_battlefield(&engine, p0, aura_shards()).is_some(),
        "and the Shards outlive their own trigger"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "one artifact or enchantment, one destruction: the creature that \
         started all of this is still standing"
    );
}
