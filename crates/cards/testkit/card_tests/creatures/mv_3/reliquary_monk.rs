//! `cards/creatures/mv_3/reliquary_monk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reliquary Monk is a {2}{W} 2/2 whose entire text is one dies trigger:
/// "When this creature dies, destroy target artifact or enchantment." The
/// filter names no controller, so the board carries an artifact *and* an
/// enchantment under each seat, plus a creature and a land to be declined —
/// the offer is then exactly those four permanents and nothing else. The Monk
/// dies to Vindicate aimed at itself, so one resolution both kills it and puts
/// the trigger on the stack (CR 603.3b), and the card the trigger names is the
/// one the graveyard finds afterwards.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn reliquary_monk_dies_and_destroys_a_target_artifact_or_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                reliquary_monk(),
                quiet_artifact(),
                exploration(),
                plains(),
                swamp(),
            ],
        )
        // An artifact, an enchantment, a creature and a land across the table:
        // "target artifact or enchantment" is not "a permanent you don't
        // control" and the two declinations are the proof.
        .battlefield(
            1,
            &[quiet_artifact(), exploration(), llanowar_elves(), forest()],
        )
        .hand(0, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let monk = on_battlefield(&engine, p0, reliquary_monk()).expect("the Monk is out");
    let my_rock = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let my_glide = on_battlefield(&engine, p0, exploration()).expect("my enchantment is out");
    let their_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_glide = on_battlefield(&engine, p1, exploration()).expect("their enchantment is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    cast_from_hand(&mut engine, p0, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        options.contains(&monk),
        "the first target question is Vindicate's own, and the Monk is a \
         permanent it may name: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![monk],
            },
        )
        .expect("the Monk was one of the options it enumerated");

    // Vindicate resolves, the Monk dies, and the death trigger is put on the
    // stack with its own target question (CR 603.3b).
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
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Monk's controller answers its own trigger");
    assert!(
        in_graveyard(&engine, p0, reliquary_monk()).is_some(),
        "the Monk died before the trigger was put on the stack"
    );
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&my_rock) && options.contains(&their_rock),
        "\"target artifact\" is either seat's artifact: {options:?}"
    );
    assert!(
        options.contains(&my_glide) && options.contains(&their_glide),
        "and \"or enchantment\" is either seat's enchantment: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "a creature that is not an artifact is no legal target: {options:?}"
    );
    assert!(
        !options.contains(&their_land),
        "a land is neither an artifact nor an enchantment: {options:?}"
    );
    assert_eq!(options.len(), 4, "and those four are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_rock],
            },
        )
        .expect("the artifact the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the targeted artifact left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its owner's graveyard, not merely gone"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some()
            && on_battlefield(&engine, p0, exploration()).is_some()
            && on_battlefield(&engine, p1, exploration()).is_some(),
        "one target, one destruction: the other three permanents never moved"
    );
    assert!(
        on_battlefield(&engine, p0, reliquary_monk()).is_none(),
        "and the Monk itself is still dead, so the trigger fired once for one death"
    );
}
