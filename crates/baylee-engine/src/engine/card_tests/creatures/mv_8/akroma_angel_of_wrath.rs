//! `cards/creatures/mv_8/akroma_angel_of_wrath.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn akroma() -> CardIndex {
    card_index("107f204c-9fd3-490d-8887-be63d14fc6a4")
}

/// Akroma has flying, first strike, vigilance, trample and haste, and
/// protection from black and from red; it is a 6/6 for {5}{W}{W}{W}.
#[test]
fn akroma_is_cast_for_eight_with_her_five_keywords() {
    let mut engine = at_main(&[plains(); 8], &[akroma()]);
    let angel = cast_and_settle(&mut engine, akroma());
    assert_eq!(pt(&engine, angel), (6, 6));
    let k = keywords(&engine, angel);
    for (kw, name) in [
        (KeywordSet::FLYING, "flying"),
        (KeywordSet::FIRST_STRIKE, "first strike"),
        (KeywordSet::VIGILANCE, "vigilance"),
        (KeywordSet::TRAMPLE, "trample"),
        (KeywordSet::HASTE, "haste"),
    ] {
        assert!(k.contains(kw), "{name}");
    }
}

/// Haste and vigilance together: cast this turn, Akroma is offered as an
/// attacker and is still untapped once the attack is declared.
#[test]
fn akroma_attacks_the_turn_she_is_cast_and_stays_untapped() {
    let mut engine = at_main(&[plains(); 8], &[akroma()]);
    let angel = cast_and_settle(&mut engine, akroma());
    attack_player(&mut engine, angel);
    assert!(!is_tapped(&engine, angel), "vigilance: attacking did not tap her");
}

/// Protection from red and from black: the opponent's Lightning Bolt and
/// Vindicate cannot name her, while an unprotected Bear is a target of both.
#[test]
fn akroma_cannot_be_targeted_by_red_or_black_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for spell in [lightning_bolt(), vindicate()] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[akroma(), grizzly_bears()])
            .battlefield(1, &[mountain(), plains(), swamp()])
            .hand(1, &[spell])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        reach_their_main_phase(&mut engine, p1);
        let angel = on_battlefield(&engine, p0, akroma()).expect("Akroma");
        let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("the bear");
        cast_from_hand(&mut engine, p1, spell);
        let options = pass_until_targets(&mut engine, p1);
        assert!(options.contains(&bear), "the bear is a target: {options:?}");
        assert!(
            !options.contains(&angel),
            "protection from the spell's colour: {options:?}"
        );
    }
}
