//! `cards/instants/mv_3/hero_s_downfall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hero's Downfall — {1}{B}{B} instant: "Destroy target creature or
/// planeswalker." That filter *is* the whole card, so the scenario puts one
/// creature and one planeswalker across the table with four Islands beside
/// them as the control: the offer is exactly those two permanents and never a
/// land, and what leaves the battlefield is the one that was named. Karn
/// arrives by being *cast* rather than seeded — a permanent put down by
/// `starting_battlefield` is a placement and not an entry, so no replacement
/// effect would hand a planeswalker the loyalty it needs to survive the first
/// state-based check.
#[test]
fn heroes_downfall_destroys_the_creature_or_planeswalker_it_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[heroes_downfall()])
        .battlefield(
            1,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(1, &[karn_the_great_creator()])
        .start();
    keep_mulligans(&mut engine);

    // The planeswalker the Downfall is for arrives first, off four Islands,
    // and the walk stops the moment p0 may answer it.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, karn_the_great_creator());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, karn_the_great_creator()).is_some()
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let karn = on_battlefield(&engine, p1, karn_the_great_creator()).expect("Karn resolved");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elves are out");
    let islands = all_on_battlefield(&engine, p1, island());
    assert_eq!(islands.len(), 4, "four Islands paid for the planeswalker");

    // {1}{B}{B} off three Swamps. The target is named before the mana is
    // spent (CR 601.2c before CR 601.2h), so the question comes back first.
    cast_from_hand(&mut engine, p0, heroes_downfall());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature or planeswalker\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster names the target");
    assert_eq!(
        (min, max),
        (1, 1),
        "exactly one permanent, as the card reads"
    );
    assert!(
        player_options.is_empty(),
        "a destroy spell targets no player: {player_options:?}"
    );
    assert!(
        options.contains(&karn),
        "the planeswalker half of the filter: {options:?}"
    );
    assert!(options.contains(&elves), "the creature half: {options:?}");
    assert!(
        islands.iter().all(|land| !options.contains(land)),
        "\"creature or planeswalker\" is read and not skipped: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![karn],
            },
        )
        .expect("the planeswalker the question offered");
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, karn_the_great_creator()).is_some()
    });

    assert!(
        on_battlefield(&engine, p1, karn_the_great_creator()).is_none(),
        "the named planeswalker left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, karn_the_great_creator()).is_some(),
        "and is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and only what was named: the creature beside it still stands"
    );
    assert!(
        in_graveyard(&engine, p0, heroes_downfall()).is_some(),
        "the instant itself resolved into its owner's graveyard"
    );
}
