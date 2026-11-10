//! `cards/creatures/mv_2/serra_avenger.rs`, played.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("b91b65b3-b35c-460e-a49b-333000a9a519")
}

#[test]
fn serra_avenger_cannot_be_cast_before_its_controllers_fourth_turn() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let card = in_hand(&e, p0, subject()).unwrap();
    assert!(!priority_offer(&e).castable.contains(&card));
    for expected_turn in [3, 5] {
        pass_until(&mut e, |g| {
            g.state().turn.active == p0
                && g.state().turn.number == expected_turn
                && matches!(g.state().turn.phase, Phase::FirstMain)
        });
        assert!(!priority_offer(&e).castable.contains(&card));
    }
    pass_until(&mut e, |g| {
        g.state().turn.active == p0
            && g.state().turn.number == 7
            && matches!(g.state().turn.phase, Phase::FirstMain)
    });
    tap_all_mana(&mut e, p0);
    assert!(priority_offer(&e).castable.contains(&card));
    cast_with_floating(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let angel = on_battlefield(&e, p0, subject()).unwrap();
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    assert!(keywords(&e, angel).contains(KeywordSet::VIGILANCE));
}
