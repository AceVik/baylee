//! `cards/enchantments/auras/mv_1/imposing_visage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Imposing Visage — {R} Aura: "Enchant creature. Enchanted creature has
/// menace." The word "enchant creature" names no controller, so the target
/// question is read over both sides of the table and answered with the
/// caster's own Elf. The static is
/// `Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource])`, so the
/// bare Elf beside the host and the Elf across it are the two controls: a
/// filter that had widened to "creatures you control" (or dropped the
/// attachment) would have granted menace to one or both and passed every
/// assertion about the host.
#[test]
fn imposing_visage_enchants_any_creature_and_grants_menace_only_to_that_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, mountain())
        .battlefield(0, &[mountain(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[imposing_visage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::MENACE),
        "nothing is enchanted yet"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::MENACE),
        "and the Elf across the table is a printed 1/1"
    );

    // {R} off the Mountain. `tap_all_mana` also taps the two Elves for their
    // own mana ability, which costs this scenario nothing — nobody attacks.
    cast_from_hand(&mut engine, p0, imposing_visage());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "an Aura picks its host as it is cast (CR 601.2c), got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat casting the Aura is the one asked");
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    let visage = on_battlefield(&engine, p0, imposing_visage()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(visage).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::MENACE),
        "\"enchanted creature has menace\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::MENACE),
        "the static reaches the creature the Aura holds and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::MENACE),
        "nor across the table"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "menace is a keyword and no body: the Elf is still a printed 1/1"
    );
}
