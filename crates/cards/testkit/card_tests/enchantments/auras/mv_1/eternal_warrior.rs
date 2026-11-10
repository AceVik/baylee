//! `cards/enchantments/auras/mv_1/eternal_warrior.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eternal Warrior — {R} Aura: "Enchant creature. Enchanted creature has
/// vigilance."
///
/// A keyword grant is only worth playing if the keyword's own rule gets read,
/// so the game is walked as far as the attack declaration: vigilance says
/// attacking doesn't cause the creature to tap (CR 702.20b), which is the one
/// reading that separates the enchanted Elf from the bare Elf beside it once
/// both are declared. The bare Elf, the Elf across the table and the Aura
/// itself are the controls — "enchanted creature" is not "creatures you
/// control" and not "every permanent in this game", and the Aura grants the
/// keyword rather than keeping it. The Mountain is the only source tapped:
/// the Elves are the attackers below, so both are named as kept back, and
/// casting off `cast_with_floating` proves the {R} was really paid.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn eternal_warrior_grants_vigilance_to_the_creature_it_enchants() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[eternal_warrior()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::VIGILANCE),
        "nothing is enchanted yet"
    );

    // {R} off the Mountain alone: the Elves are kept untapped because they
    // are the attackers below, and a creature tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one red, and the Elves are still standing"
    );
    cast_with_floating(&mut engine, p0, eternal_warrior());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("an Aura targets as it is cast, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"enchant creature\" reaches either Elf: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and a creature across the table too — the filter is `Filter::CREATURE`: {options:?}"
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

    let aura = on_battlefield(&engine, p0, eternal_warrior()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and it is attached to the Elf it was aimed at"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::VIGILANCE),
        "enchanted creature has vigilance"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::VIGILANCE),
        "the static reaches the enchanted creature and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::VIGILANCE),
        "nor across the table"
    );
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::VIGILANCE),
        "the Aura grants the keyword, it does not keep it"
    );

    // The keyword's whole meaning, which is a combat-step fact: both Elves
    // may attack (both are untapped and past summoning sickness), and only
    // the bare one pays for it by tapping.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&host) && attackers.contains(&bystander),
        "both untapped Elves are offered as attackers: {attackers:?}"
    );
    assert!(
        !attackers.contains(&theirs),
        "and the Elf across the table is not mine to send: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (host, Defender::Player(p1)),
                    (bystander, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    assert!(
        !is_tapped(&engine, host),
        "CR 702.20b: vigilance, so attacking does not tap the enchanted creature"
    );
    assert!(
        is_tapped(&engine, bystander),
        "while the Elf beside it taps to attack like any other, so the \
         untapped one above is the grant and not a combat step that never came"
    );
}
