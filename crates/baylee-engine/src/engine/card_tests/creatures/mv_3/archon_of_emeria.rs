//! `cards/creatures/mv_3/archon_of_emeria.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Archon of Emeria is `Coverage::Partial`: its flying is a keyword bit,
/// while both printed statics — the one-spell-per-turn limit and the tapped
/// nonbasic lands — have no vocabulary in the DSL at all. So the card is
/// played the only way it can be, cast off three Plains and read back off the
/// battlefield through the layer system rather than off the card file: a 2/3
/// with flying. The second half is the gap struck rather than left implied —
/// a nonbasic land the *opponent* plays comes in untapped, which is exactly
/// the sentence the card does not carry. The land has to be one that enters
/// untapped by its own text, which Rogue's Passage is and Irrigated
/// Farmland — the first one tried here — is not: a land that taps itself on
/// the way in would have passed this assertion for a reason that has
/// nothing to do with the Archon.
#[test]
fn archon_of_emeria_lands_as_a_two_three_flier_and_does_not_tap_the_opponents_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(83, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[archon_of_emeria()])
        .hand(1, &[rogue_s_passage()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, archon_of_emeria());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, archon_of_emeria()).is_some() && stack_is_empty(e)
    });
    let archon = on_battlefield(&engine, p0, archon_of_emeria()).expect("the Archon resolved");
    assert_eq!(pt(&engine, archon), (2, 3), "the body the card prints");
    assert!(
        keywords(&engine, archon).contains(KeywordSet::FLYING),
        "the one implemented line, as the layers project it"
    );

    reach_their_main_phase(&mut engine, p1);
    let land = play_land(&mut engine, p1, rogue_s_passage());
    assert!(
        !entered_tapped(&engine, land),
        "\"nonbasic lands your opponents control enter tapped\" is the \
         Coverage::Partial gap: the Archon stands and the land still comes in untapped"
    );
}
