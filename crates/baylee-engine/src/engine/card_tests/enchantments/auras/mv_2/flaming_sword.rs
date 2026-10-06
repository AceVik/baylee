//! `cards/enchantments/auras/mv_2/flaming_sword.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flaming Sword prints flash, "Enchant creature", and one static: the
/// enchanted creature gets +1/+0 and has first strike. Both grants hang on
/// `Filter::AttachedToBySource`, so the board has to separate the creature
/// the Aura holds from every other creature on the table — an unenchanted
/// Elf beside the host and an Elf across it both have to stay `(1, 1)` and
/// keywordless, or the static is only being read, not tested. The flash
/// line is played rather than noted: the Aura is cast in the *opponent's*
/// main phase off two Mountains that were never tapped for anything, which
/// is a cast no sorcery-speed Aura could make.
#[test]
fn flaming_sword_flashes_in_to_arm_only_the_creature_it_enchants() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), llanowar_elves(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flaming_sword()])
        .start();
    keep_mulligans(&mut engine);

    // Flash (CR 702.8a) is a cast at instant speed, so the window is p1's
    // own first main phase, stack empty, with p0 holding priority.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays unenchanted");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");

    // Neither Elf may be tapped for its own mana: the two Mountains are the
    // whole of the {1}{R}, and the host has to still be what it was.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, flaming_sword());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the caster chooses what their own Aura enchants"
    );
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "either creature under your control may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"enchant creature\" is any creature: an Aura is not a spell that \
         has to be aimed at your own side: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let sword = on_battlefield(&engine, p0, flaming_sword()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(sword).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it was cast at"
    );

    assert_eq!(pt(&engine, host), (2, 1), "+1/+0 on the enchanted creature");
    assert!(
        keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "and first strike with it"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FIRST_STRIKE),
        "the static reaches the enchanted creature and no other"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "nor does the keyword travel with the Aura's colour"
    );
    assert!(
        !keywords(&engine, sword).contains(KeywordSet::FIRST_STRIKE),
        "the Aura grants the keyword, it does not keep it"
    );
}
