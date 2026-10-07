//! `cards/creatures/mv_4/wall_of_swords.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "eb098958-50d3-4476-ba74-382033703ff9"

/// Wall of Swords — {3}{W}, a 3/5 white Wall printing exactly two keywords:
/// flying and defender ("This creature can't attack.").
///
/// Both have to be read where they change something rather than where they sit
/// in the card file, so one game reads both: flying arrives on the projected
/// keyword set, and defender is read off the attack declaration a turn later,
/// where an untapped Elf of the same seat is offered and the Wall — untapped,
/// past summoning sickness, and otherwise a perfectly serviceable 3/5 flier —
/// is not. Reading "can't attack" on the arrival turn would prove nothing,
/// because CR 302.6 withholds every creature that just entered.
#[test]
fn wall_of_swords_flies_and_its_defender_keeps_it_out_of_the_attack_declaration() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .hand(0, &[wall_of_swords()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Plains pay {3}{W} exactly, so the pool is empty afterwards; the Elf
    // is named as the printing kept back because it is the control in the
    // attack declaration below, and a creature tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four tapped Plains and an untapped Elf"
    );
    cast_with_floating(&mut engine, p0, wall_of_swords());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, wall_of_swords()).is_some()
    });

    let wall = on_battlefield(&engine, p0, wall_of_swords()).expect("the Wall resolved");
    assert_eq!(pt(&engine, wall), (3, 5), "the body the card prints");
    assert!(
        types(&engine, wall).contains(TypeSet::CREATURE),
        "and it arrives as the creature it prints"
    );
    let granted = keywords(&engine, wall);
    assert!(
        granted.contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert!(
        granted.contains(KeywordSet::DEFENDER),
        "and so does the printed defender, both read off the projection: {granted:?}"
    );

    // A whole turn cycle, so CR 302.6 cannot be the reason it stays home.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the Wall's controller is the one being asked");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still out");
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 with no text of its own may attack, so the offer is \
         not an empty list: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "\"Defender (This creature can't attack.)\": the 3/5 flier is the same \
         untapped, unsick permanent the Elf beside it is, and the one printed \
         word is what leaves it off the offer: {attackers:?}"
    );
}

/// Nettling Imp: "Choose target non-Wall creature the active player has
/// controlled continuously since the beginning of the turn." A Wall, a
/// creature p0 cast this turn, and the Imp's own controller's creature are
/// left off the menu; a creature p0 has held since the turn began is on
/// it. "That creature attacks this turn if able": a declaration leaving it
/// out is refused, and the creature that attacked survives the end step.
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn nettling_imp_targets_only_a_held_since_the_turn_began_non_wall_creature_and_forces_its_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves(), wall_of_swords()])
        .hand(0, &[grizzly_bears()])
        .battlefield(1, &[nettling_imp(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let wall = on_battlefield(&engine, p0, wall_of_swords()).expect("the Wall is seated");
    let imp = on_battlefield(&engine, p1, nettling_imp()).expect("the Imp is seated");
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).expect("p1's own Bear is seated");

    // Not `cast_from_hand`: the Elf prints its own "{T}: Add {G}" and would
    // otherwise pay for the fresh Bear, tapping itself out of the very
    // targeting question this test is about.
    tap_mana_except(&mut engine, p0, elf);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    let fresh_bear = on_battlefield(&engine, p0, grizzly_bears()).expect("the fresh Bear resolved");

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: imp,
                ability_index: 0,
            },
        )
        .expect("offered in p0's beginning of combat, before attackers are declared");
    let options = aim_at(&mut engine, p1, elf);
    assert!(
        options.contains(&elf),
        "held since the turn began: on the menu"
    );
    assert!(!options.contains(&wall), "a Wall is never a legal target");
    assert!(
        !options.contains(&fresh_bear),
        "cast this turn: not controlled since the turn began"
    );
    assert!(
        !options.contains(&their_bear),
        "the Imp's own controller's creature, not the active player's"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, imp), "the Imp paid its own {{T}} cost");
    assert!(
        !priority_offer(&engine).abilities.contains(&(imp, 0)),
        "tapped, the Imp cannot pay {{T}} again to activate a second time \
         this turn"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, required, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0);
    assert_eq!(required, vec![elf], "\"attacks this turn if able\"");
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "leaving out the Elf disobeys the Imp's requirement"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .expect("attacking with the Elf obeys it");

    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "it attacked this turn, so the delayed destruction spares it"
    );
}
