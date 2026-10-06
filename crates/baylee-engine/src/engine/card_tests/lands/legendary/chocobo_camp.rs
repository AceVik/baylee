//! `cards/lands/legendary/chocobo_camp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Chocobo Camp: "This land enters tapped unless you control a legendary creature." / "{T}: Add {G}." / "{2}{G}{G}, {T}: Create a 2/2 green Bird creature token..."
/// Under `Coverage::Partial`, the delayed counter rider is omitted.
/// Controlling a legendary creature allows Chocobo Camp to enter untapped and immediately tap for green mana.
#[test]
fn chocobo_camp_enters_untapped_with_legendary_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(213, forest())
        .battlefield(0, &[katara_the_fearless()])
        .hand(0, &[chocobo_camp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, chocobo_camp());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, chocobo_camp(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// Chocobo Camp's `{{2}}{{G}}{{G}}, {{T}}`: one 2/2 green Bird creature token.
#[test]
fn chocobo_camp_makes_a_bird_for_four() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(213, forest())
        .battlefield(0, &[chocobo_camp(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let camp = on_battlefield(&engine, p0, chocobo_camp()).expect("Chocobo Camp deployed");
    tap_mana_except(&mut engine, p0, camp);
    activate(&mut engine, p0, chocobo_camp(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, camp));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Bird");
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
    assert!(types(&engine, tokens[0]).contains(TypeSet::CREATURE));
}
