//! `cards/enchantments/mv_3/fervor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fervor — {2}{R} enchantment: "Creatures you control have haste."
///
/// Haste projects no characteristic at all; the only place it exists is in
/// the attackers the combat step is willing to offer, so the scenario is the
/// one play that needs it: a Llanowar Elves cast in the same main phase it
/// would have to attack in (CR 302.6). Three Mountains pay the enchantment
/// while the lone Forest is held back to pay for the Elf, and the fresh 1/1
/// is both offered as an attacker and connects for exactly its power.
///
/// The second game is the control, and it is the same board down to the last
/// card with Fervor seated *across* the table instead: the identical Elf is
/// not offered. That one difference reads both halves of
/// `Filter::YOUR_CREATURE` — nothing but the static put the first Elf in the
/// list, and it reaches its controller's creatures and no other seat's.
#[test]
fn fervor_haste_only_its_own_controllers_creatures_so_they_attack_as_they_arrive() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // Mine: the Mountains cast the enchantment and the Forest is kept back
    // for the creature it is about to make hasty.
    let mut mine = Duel::new(47, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), forest()])
        .hand(0, &[fervor(), llanowar_elves()])
        .start();
    keep_mulligans(&mut mine);
    assert!(walk_to_own_main(&mut mine, p0), "p0 reaches its own main");
    tap_all_mana_but(&mut mine, p0, Some(forest()));
    assert_eq!(
        mine.state().players[0].mana_pool.total(),
        3,
        "three Mountains, and the Forest kept back for the Elf"
    );
    cast_with_floating(&mut mine, p0, fervor());
    pass_until(&mut mine, stack_is_empty);
    assert!(
        on_battlefield(&mine, p0, fervor()).is_some(),
        "the enchantment resolved onto the battlefield"
    );

    cast_from_hand(&mut mine, p0, llanowar_elves());
    pass_until(&mut mine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = mine.pending().clone() else {
        unreachable!("pass_until stopped on nothing but the attack declaration");
    };
    let hasty = on_battlefield(&mine, p0, llanowar_elves()).expect("the Elf resolved");
    assert!(
        attackers.contains(&hasty),
        "\"creatures you control have haste\": the Elf entered this turn and \
         is offered anyway (CR 302.6): {attackers:?}"
    );

    // And it is a real attack, not merely a name on a list.
    mine.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(hasty, Defender::Player(p1))],
        },
    )
    .expect("the offer is what the permission looks like");
    pass_until(&mut mine, |e| matches!(e.state().turn.phase, Phase::Ending));
    assert_eq!(
        mine.state().players[1].life,
        19,
        "the 1/1 connected for exactly its power"
    );

    // Theirs: one card different, and the same Elf stays home.
    let mut theirs = Duel::new(47, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), forest()])
        .battlefield(1, &[fervor()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut theirs);
    assert!(walk_to_own_main(&mut theirs, p0), "p0 reaches its own main");
    cast_from_hand(&mut theirs, p0, llanowar_elves());
    pass_until(&mut theirs, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = theirs.pending().clone() else {
        unreachable!("pass_until stopped on nothing but the attack declaration");
    };
    let sick = on_battlefield(&theirs, p0, llanowar_elves()).expect("the Elf resolved");
    assert!(
        !attackers.contains(&sick),
        "an opponent's Fervor is not this seat's: the same freshly played Elf \
         is still summoning-sick and is not offered: {attackers:?}"
    );
}
