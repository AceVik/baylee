//! `cards/enchantments/mv_2/serra_s_blessing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Serra's Blessing — {1}{W} — "Creatures you control have vigilance."
///
/// The keyword cannot be read off a projection: what vigilance *does* only
/// shows up in combat, so the Elf takes the attack declaration and is still
/// standing afterwards, which is the one thing the keyword changes. The
/// control is the Elf across the table attacking on its own turn and tapping
/// normally, so the untapped Elf says something about a granted rule rather
/// than about a declaration that never happened. Before the cast and across
/// the table the keyword is asserted absent, because a static without
/// `Filter::YOUR_CREATURE` would have armed the whole table.
#[test]
fn serras_blessing_keeps_your_attacker_standing_and_not_theirs() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[serras_blessing()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::VIGILANCE),
        "nothing is granted before the enchantment resolves"
    );

    // Two Plains pay the {1}{W}; the Elf is named as the printing kept back,
    // because it is the creature this test attacks with and a creature tapped
    // for mana is tapped for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, serras_blessing());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, serras_blessing()).is_some(),
        "the enchantment resolved"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::VIGILANCE),
        "\"creatures you control have vigilance\""
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::VIGILANCE),
        "\"you control\" — the Elf across the table is not granted the keyword"
    );

    // Attacking is where the keyword is worth anything: the Elf is declared as
    // an attacker and stays untapped, which a printed 1/1 without vigilance
    // could not do.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mine, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert!(
        !is_tapped(&engine, mine),
        "\"attacking doesn't cause them to tap\" — the Elf that just attacked \
         is still standing"
    );

    // The control: the same declaration one turn later, by the seat with no
    // Serra's Blessing, does tap its attacker.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(theirs, Defender::Player(p0))],
            },
        )
        .unwrap();
    assert!(
        is_tapped(&engine, theirs),
        "an attacker with no vigilance taps as it attacks"
    );
}
