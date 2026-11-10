//! `cards/creatures/mv_9/spirit_of_the_night.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn spirit() -> CardIndex {
    card_index("73016220-2436-4072-96dd-65c0cb05fff4")
}

/// The Spirit is a 6/5 for {6}{B}{B}{B} with flying, trample and haste.
#[test]
fn spirit_of_the_night_is_cast_for_nine_with_three_keywords() {
    let mut engine = at_main(&[swamp(); 9], &[spirit()]);
    let demon = cast_and_settle(&mut engine, spirit());
    assert_eq!(pt(&engine, demon), (6, 5));
    let k = keywords(&engine, demon);
    assert!(k.contains(KeywordSet::FLYING));
    assert!(k.contains(KeywordSet::TRAMPLE));
    assert!(k.contains(KeywordSet::HASTE));
    assert!(
        !k.contains(KeywordSet::FIRST_STRIKE),
        "not while it is not attacking"
    );
}

/// "Spirit of the Night has first strike as long as it's attacking": none
/// before the attack is declared, first strike once it is.
#[test]
fn spirit_of_the_night_has_first_strike_only_while_attacking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = at_main(&[spirit()], &[]);
    let demon = on_battlefield(&engine, p0, spirit()).expect("the Spirit");
    assert!(!keywords(&engine, demon).contains(KeywordSet::FIRST_STRIKE));
    attack_and_collect_blocks(&mut engine, demon, p1);
    assert!(keywords(&engine, demon).contains(KeywordSet::FIRST_STRIKE));
}

/// Protection from black: the opponent's Vindicate (white and black) cannot
/// name it, while a red Lightning Bolt can.
#[test]
fn spirit_of_the_night_has_protection_from_black_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (spell, can_target) in [(vindicate(), false), (lightning_bolt(), true)] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[spirit()])
            .battlefield(1, &[mountain(), plains(), swamp()])
            .hand(1, &[spell])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        reach_their_main_phase(&mut engine, p1);
        let demon = on_battlefield(&engine, p0, spirit()).expect("the Spirit");
        cast_from_hand(&mut engine, p1, spell);
        let options = pass_until_targets(&mut engine, p1);
        assert_eq!(options.contains(&demon), can_target, "{options:?}");
    }
}
