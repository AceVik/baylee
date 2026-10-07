//! `cards/instants/mv_1/berserk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Berserk: "Cast this spell only before the combat damage step" (CR 506.7).
/// Offered in the upkeep, the first main phase, the beginning of combat,
/// the declare attackers step (both before and after the attacker is
/// declared — that step is not skipped, CR 508.8, since one is) and through
/// the declare blockers step, and refused everywhere the combat damage step
/// has already begun or passed: that step itself, end of combat, the second
/// main phase, the end step. A Forest is tapped fresh at every checkpoint,
/// open or shut, so a "not castable" reading is never merely "no floating
/// mana".
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn berserk_is_castable_before_combat_damage_and_refused_from_it_on() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut lands = vec![forest(); 9];
    lands.push(llanowar_elves());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &lands)
        .hand(0, &[berserk()])
        .start();
    keep_mulligans(&mut engine);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let castable = |e: &Engine<RegistryLookup>| {
        let spell = in_hand(e, p0, berserk()).expect("Berserk is still in hand");
        priority_offer(e).castable.contains(&spell)
    };

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_one_mana(&mut engine, p0);
    assert!(
        castable(&engine),
        "the upkeep step is well before the combat damage step"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_one_mana(&mut engine, p0);
    assert!(
        castable(&engine),
        "the first main phase is before the combat damage step"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_one_mana(&mut engine, p0);
    assert!(
        castable(&engine),
        "beginning of combat, still before attackers are even declared"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(attackers.contains(&elf));
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .expect("the Elf came out of the list that offered it");

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_one_mana(&mut engine, p0);
    assert!(
        castable(&engine),
        "the declare attackers step, with an attacker now named, is still \
         before the combat damage step"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_one_mana(&mut engine, p0);
    assert!(
        castable(&engine),
        "the declare blockers step is still before the combat damage step"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatDamage
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_one_mana(&mut engine, p0);
    assert!(
        !castable(&engine),
        "the combat damage step has begun — \"only before\" it is over, \
         and the mana just floated proves this is not about affording it"
    );
    let refused = in_hand(&engine, p0, berserk()).expect("Berserk is still in hand");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: refused })
            .is_err(),
        "the window is the engine's rule, not only the offer's"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatEnd
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_one_mana(&mut engine, p0);
    assert!(!castable(&engine), "end of combat, later still");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_one_mana(&mut engine, p0);
    assert!(!castable(&engine), "the second main phase");

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    float_one_mana(&mut engine, p0);
    assert!(!castable(&engine), "the end step");
}

/// Berserk carries no clause about whose turn it is, so p1 casts it freely
/// during p0's turn. And when nothing is declared this combat, the declare
/// blockers and combat damage steps are skipped outright (CR 508.8), so the
/// stated point — the combat damage step — never exists this combat; CR
/// 506.7e then closes the window at the end of the declare attackers step
/// instead. A Forest is tapped fresh at every checkpoint, open or shut, so
/// a "not castable" reading is never merely "no floating mana".
#[test]
fn berserk_is_castable_on_an_opponents_turn_and_closes_at_the_declare_attackers_step_when_nothing_attacked()
 {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            1,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                grizzly_bears(),
            ],
        )
        .hand(1, &[berserk()])
        .start();
    keep_mulligans(&mut engine);

    // A legal target throughout: without one Berserk is withheld for that
    // reason alone (CR 601.2c), which would say nothing about the window
    // this test is about.
    let castable = |e: &Engine<RegistryLookup>| {
        let spell = in_hand(e, p1, berserk()).expect("Berserk is still in hand");
        priority_offer(e).castable.contains(&spell)
    };

    // Turn 1 is p0's; p0 has nothing to attack with, so the declare
    // attackers turn-based action declares nobody on its own.
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(
        castable(&engine),
        "an opponent's turn is no obstacle: Berserk names none"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(castable(&engine), "p0's main phase, still before combat");

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(castable(&engine), "beginning of combat");

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(
        castable(&engine),
        "the declare attackers step, before it ends, is open even though \
         nothing was named this combat (CR 506.7a: the rule reads the step, \
         not the declaration)"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatEnd
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    float_one_mana(&mut engine, p1);
    assert!(
        !castable(&engine),
        "declare blockers and combat damage were both skipped (CR 508.8), \
         so the stated point never existed this combat; CR 506.7e closes \
         the window at the declare attackers step's end instead, and the \
         mana just floated proves this is not about affording it"
    );
}

/// Berserk's delayed destruction: "At the beginning of the next end step,
/// destroy that creature if it attacked this turn." Two Berserks are cast
/// in the same main phase, one on a creature that then attacks unblocked
/// and survives combat, one on a creature that stays home. Both gain the
/// same trample and +X/+0; combat over, neither has been claimed yet — the
/// trigger fires at the *beginning* of the end step, not during combat —
/// and the attacker is untapped again before that step begins, so what
/// destroys it afterward can only be that it attacked this turn, not that
/// it is still tapped from doing so.
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn berserk_destroys_the_creature_it_pumped_only_if_it_attacked_this_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), llanowar_elves(), festering_goblin()],
        )
        .hand(0, &[berserk(), berserk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let goblin = on_battlefield(&engine, p0, festering_goblin()).expect("the Goblin is seated");

    // Not `tap_all_mana`: the Elf prints its own "{T}: Add {G}" and would
    // otherwise pay for its own Berserk, leaving it unable to attack for a
    // reason that has nothing to do with what this test is about.
    tap_mana_except(&mut engine, p0, elf);
    cast_with_floating(&mut engine, p0, berserk());
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, p0, berserk());
    aim_at(&mut engine, p0, goblin);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::TRAMPLE)
            && keywords(&engine, goblin).contains(KeywordSet::TRAMPLE),
        "both creatures were pumped by their own Berserk the same way"
    );

    let blocks = attack_and_collect_blocks(&mut engine, elf, p1);
    assert!(
        blocks.is_empty(),
        "p1 has nothing on the battlefield to block with"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("p1 declares no blocks");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "combat is over and the end step has not begun yet: the delayed \
         destruction has not fired"
    );
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some(),
        "same reading for the creature the trigger will end up sparing"
    );

    // Untapped here, before the end step begins: what the delayed trigger
    // reads is whether the Elf attacked this turn (`Filter::AttackedThisTurn`),
    // a fact recorded at the declaration and never erased by an untap, not
    // whether it is still tapped from having done so.
    engine
        .dev_state_mut(p0)
        .expect("a test seat has dev commands")
        .set_tapped(elf, false);
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the Elf attacked this turn, so its own Berserk's delayed \
         destruction claims it as the end step begins, untapped or not"
    );
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some(),
        "the Goblin never attacked, so the same delayed destruction from \
         its own Berserk spares it"
    );
}
