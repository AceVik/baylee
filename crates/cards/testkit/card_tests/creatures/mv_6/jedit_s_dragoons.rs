//! `cards/creatures/mv_6/jedit_s_dragoons.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jedit's Dragoons — {5}{W} — a 2/5 Cat Soldier with vigilance and "When
/// this creature enters, you gain 4 life." Both printed sentences are played
/// in one game rather than read: the entry trigger moves the caster's life by
/// exactly four while the opponent's stays where it was, and the vigilance is
/// proved the only way a keyword can be — the Dragoons attack a turn later,
/// deal their two damage, and are still untapped when it lands.
#[test]
fn jedits_dragoons_gains_four_life_on_entry_and_attacks_without_tapping() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 6])
        .hand(0, &[jedit_s_dragoons()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Six Plains are exactly {5}{W}, and `can_afford` reads the pool rather
    // than the untapped lands — so the mana is made first and the cost is
    // read off what is floating.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 6, "six Plains, six white");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "and exactly the {{5}}{{W}} the card prints"
    );
    cast_with_floating(&mut engine, p0, jedit_s_dragoons());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, jedit_s_dragoons()).is_some()
    });

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{5}}{{W}} came out of the pool"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "\"When this creature enters, you gain 4 life\" — four, and once"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the seat that cast it, not to the opponent"
    );
    let dragoons = on_battlefield(&engine, p0, jedit_s_dragoons()).expect("the Dragoons resolved");
    assert_eq!(pt(&engine, dragoons), (2, 5), "the body the card prints");
    assert!(
        keywords(&engine, dragoons).contains(KeywordSet::VIGILANCE),
        "the printed vigilance reaches the permanent, not just the card file"
    );

    // A creature that entered this turn is summoning sick (CR 302.6), so the
    // keyword needs a full turn before it can be played rather than read.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, jedit_s_dragoons()).is_some(),
        "the Dragoons survived the turn they arrived in"
    );
    assert!(
        !is_tapped(&engine, dragoons),
        "and their controller's untap step stood them back up"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the Dragoons' controller attacks");
    assert!(
        attackers.contains(&dragoons),
        "an untapped 2/5 with a printed keyword and no drawback is a legal \
         attacker: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dragoons, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    // The stack is already empty the moment attackers are declared, so the
    // end step is what "past the combat damage step" (CR 510.2) means here.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "two combat damage from the Dragoons"
    );
    assert!(
        !is_tapped(&engine, dragoons),
        "vigilance: \"attacking doesn't cause this creature to tap\" — the \
         damage is on the opponent and the Dragoons are still standing"
    );
}
