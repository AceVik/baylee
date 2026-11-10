//! `cards/creatures/mv_3/archon_of_emeria.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cast the actual 2/3 flier, then verify its opponent-only nonbasic entry rule
/// on Rogue's Passage, which does not itself enter tapped.
#[test]
fn archon_of_emeria_lands_as_a_two_three_flier_and_taps_the_opponents_nonbasic() {
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
        "flying, as the layers project it"
    );

    reach_their_main_phase(&mut engine, p1);
    let land = play_land(&mut engine, p1, rogue_s_passage());
    assert!(
        entered_tapped(&engine, land),
        "the Archon makes the opponent's otherwise untapped nonbasic enter tapped"
    );
}
