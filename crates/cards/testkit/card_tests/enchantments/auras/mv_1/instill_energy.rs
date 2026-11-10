//! `cards/enchantments/auras/mv_1/instill_energy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Instill Energy: "Enchanted creature can attack as though it had haste."
/// A freshly cast creature it enchants may attack the same turn; a second,
/// identical creature beside it — just as freshly cast, but unenchanted —
/// may not. The grant is narrow: it does not also let the enchanted
/// creature use its own {{T}} ability, which summoning sickness still
/// refuses that same turn — unlike a third, non-sick Elves beside them both
/// (seated since before the game began), whose identical ability is still
/// offered.
#[test]
fn instill_energy_lets_a_freshly_cast_creature_attack_but_not_tap_for_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[llanowar_elves(), llanowar_elves(), instill_energy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf_c =
        on_battlefield(&engine, p0, llanowar_elves()).expect("seated before the game began");

    tap_mana_except(&mut engine, p0, elf_c);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elf_a = all_on_battlefield(&engine, p0, llanowar_elves())
        .into_iter()
        .find(|&id| id != elf_c)
        .expect("the first freshly cast Elves resolved");

    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let elf_b = all_on_battlefield(&engine, p0, llanowar_elves())
        .into_iter()
        .find(|&id| id != elf_c && id != elf_a)
        .expect("the second freshly cast Elves resolved");

    cast_with_floating(&mut engine, p0, instill_energy());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Enchant creature asks for a target, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&elf_a));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elf_a],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, instill_energy()).is_some()
    });
    let aura = on_battlefield(&engine, p0, instill_energy()).expect("resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(elf_a),
        "the Aura attached to the Elves it targeted"
    );

    let Pending::Priority { legal, player } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(
        legal.abilities.iter().any(|(src, _)| *src == elf_c),
        "positive control: an Elves that didn't just enter offers its own \
         {{T}}: Add {{G}}: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == elf_a),
        "the enchanted, freshly cast Elves' own tap ability is still \
         refused this turn: {:?}",
        legal.abilities
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers")
    };
    assert!(
        attackers.contains(&elf_a),
        "Instill Energy lets its enchanted, freshly cast Elves attack"
    );
    assert!(
        !attackers.contains(&elf_b),
        "a second, unenchanted Elves that also entered this turn may not"
    );
}

/// Instill Energy: "{{0}}: Untap enchanted creature. Activate only during
/// your turn and only once each turn." Untaps its creature during your
/// turn; a second activation that same turn is refused, and on the
/// opponent's turn it is not offered at all.
#[test]
fn instill_energy_s_untap_ability_is_once_per_turn_and_only_on_your_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves(), forest()])
        .hand(0, &[instill_energy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");

    tap_mana_except(&mut engine, p0, elf);
    cast_with_floating(&mut engine, p0, instill_energy());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Enchant creature asks for a target, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&elf));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, instill_energy()).is_some()
    });
    let aura = on_battlefield(&engine, p0, instill_energy()).expect("resolved");

    activate(&mut engine, p0, llanowar_elves(), 0);
    assert!(is_tapped(&engine, elf), "tapped for its own mana ability");

    activate(&mut engine, p0, instill_energy(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, elf),
        "the {{0}} ability untapped the enchanted creature"
    );

    let Pending::Priority { legal, player } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "the active player holds priority once the stack empties"
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == aura),
        "already activated once this turn: not offered again: {:?}",
        legal.abilities
    );

    // Not on the opponent's turn — read *p0's* offer there, which needs p0
    // to hold priority. A positive control sits beside it: the Elves is
    // untapped and no longer summoning sick, so its own {T} ability stays
    // offered to p0 throughout; only the Aura's {0} disappears.
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { legal, player } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "p0 holds priority once the active player (p1) passes with an empty stack"
    );
    assert!(
        legal.abilities.iter().any(|(src, _)| *src == elf),
        "positive control: p0's untapped Elves still offers its own {{T}} \
         on the opponent's turn: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == aura),
        "\"only during your turn\": the {{0}} is not offered on the \
         opponent's turn: {:?}",
        legal.abilities
    );
}
