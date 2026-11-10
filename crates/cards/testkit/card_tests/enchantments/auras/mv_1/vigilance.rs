//! `cards/enchantments/auras/mv_1/vigilance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vigilance is a {W} Aura — "Enchant creature / Enchanted creature has
/// vigilance" — and vigilance's whole printed meaning is that attacking does
/// not tap the creature (CR 702.20b). So the card is played onto one of two
/// otherwise identical Llanowar Elves and both attack in the same combat:
/// the one wearing the Aura is still standing afterwards, the one without it
/// is tapped, and the only difference between them is the Aura. The
/// projection is read before combat too, so a static that never landed would
/// be caught before the tapped check could blame the declare-attackers step.
#[test]
fn vigilance_enchants_a_creature_and_that_creature_attacks_without_tapping() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[vigilance()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::VIGILANCE),
        "an Aura that has not resolved grants nothing"
    );

    // {W} off the Plains, and both Elves kept back: the creature the Aura is
    // about to arm has to be able to attack, and one tapped for mana could not.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, vigilance());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "enchant creature asks which creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "either creature on this board may be enchanted: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered is the one enchanted");

    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, vigilance()).is_some()
    });
    let aura = on_battlefield(&engine, p0, vigilance()).expect("the Aura resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "and it entered attached to the creature it was cast on"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::VIGILANCE),
        "enchanted creature has vigilance"
    );
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::VIGILANCE),
        "the Aura grants the keyword, it does not keep it"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::VIGILANCE),
        "and it reaches the enchanted creature and no other"
    );

    // Both attack. The bare Elf is the control: it proves the combat step
    // really ran, so the standing attacker below is vigilance and not a
    // declaration that never happened.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&host) && attackers.contains(&bystander),
        "both untapped Elves may be declared: {attackers:?}"
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

    // Not `stack_is_empty`: the stack is empty the moment attackers are
    // declared, so that predicate stops the walk before the combat damage
    // step. The end step is past damage (CR 510.2) and still before p0's
    // untap step, which is the only place a tapped attacker can be read.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        !is_tapped(&engine, host),
        "\"attacking doesn't cause it to tap\": the enchanted Elf attacked and is \
         still standing"
    );
    assert!(
        is_tapped(&engine, bystander),
        "the identical Elf nobody enchanted attacked and tapped, so the \
         difference is the Aura and not a combat step that never ran"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "and both of them connected, so neither was quietly dropped from the \
         declaration"
    );
}
