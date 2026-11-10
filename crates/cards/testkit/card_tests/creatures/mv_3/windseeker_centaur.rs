//! `cards/creatures/mv_3/windseeker_centaur.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Windseeker Centaur — {1}{R}{R}, a 2/2 Centaur whose entire printed text is
/// "Vigilance". The body alone says nothing: vigilance is a rule about the
/// attack declaration (CR 702.20b), so the only reading worth playing is a
/// combat step in which the Centaur attacks and is *still untapped* afterwards.
/// The Llanowar Elves beside it is the control — the same declaration, the same
/// defender, and the Elves are tapped by it — so an untapped Centaur is the
/// keyword and not an attack that never happened. Three Mountains pay the
/// {1}{R}{R} and the Elves is kept untapped, because a creature tapped for mana
/// could not attack at all; the turn is walked forward once since a creature
/// cast this turn has summoning sickness (CR 302.6).
#[test]
fn windseeker_centaur_attacks_and_stays_untapped() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[windseeker_centaur()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elves is named as the printing kept back: `tap_all_mana` would have
    // spent its own `{T}` as well (#159), and the Elf is half of what makes
    // the untapped Centaur below readable.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains for the {{1}}{{R}}{{R}}, and the Elves left standing"
    );
    cast_with_floating(&mut engine, p0, windseeker_centaur());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let centaur = on_battlefield(&engine, p0, windseeker_centaur()).expect("the Centaur resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, centaur), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, centaur).contains(KeywordSet::VIGILANCE),
        "and the one line of text it prints"
    );
    assert!(
        !is_tapped(&engine, centaur) && !is_tapped(&engine, elves),
        "neither creature was tapped for mana, so both may attack"
    );

    // A creature cast this turn has summoning sickness (CR 302.6), so the game
    // has to come back around before the attack declaration is worth anything.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    // The same declaration for both, aimed at the same seat: whatever the
    // engine does to one it does to the other unless a keyword says otherwise.
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (centaur, Defender::Player(p1)),
                    (elves, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    assert!(
        is_tapped(&engine, elves),
        "CR 508.1f: attacking taps the Elves, which is what makes the \
         declaration a real one and not a step that never happened"
    );
    assert!(
        !is_tapped(&engine, centaur),
        "\"Vigilance\" (CR 702.20b): the Centaur attacked and is still untapped"
    );

    pass_until(&mut engine, |e| e.state().players[1].life < 20);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "2 from the Centaur and 1 from the Elves, so both attacked"
    );
    assert!(
        !is_tapped(&engine, centaur),
        "and the Centaur is untapped after the damage too"
    );
}
