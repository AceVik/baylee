//! `cards/creatures/artifacts/mv_2/tidehollow_strix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tidehollow_strix() -> CardIndex {
    card_index("19a39cc5-2134-4dee-932c-81389806c57b")
}

/// Tidehollow Strix: a 2/1 with "Flying" and "Deathtouch", cast for `{U}{B}`.
#[test]
fn tidehollow_strix_is_a_flying_deathtouch_two_one() {
    let (engine, strix) = {
        let p0 = PlayerId::new(0);
        let mut engine = table(&[tidehollow_strix()], &[island(), swamp()], &[]);
        let strix = cast_and_land(&mut engine, tidehollow_strix());
        pass_until(&mut engine, stack_is_empty);
        assert!(in_hand(&engine, p0, tidehollow_strix()).is_none());
        (engine, strix)
    };
    assert_eq!(pt(&engine, strix), (2, 1));
    assert!(keywords(&engine, strix).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, strix).contains(KeywordSet::DEATHTOUCH));
}

/// Deathtouch: blocking a 2/3 Hurloon Minotaur, two damage is not lethal by
/// toughness, but from a deathtouch source it destroys the Minotaur; the
/// Strix dies to the Minotaur's two as well.
#[test]
fn tidehollow_strix_kills_what_it_blocks_with_two_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[tidehollow_strix()], &[hurloon_minotaur()]);
    let strix = on_battlefield(&engine, p0, tidehollow_strix()).expect("seated");
    reach_their_main_phase(&mut engine, p1);
    let minotaur = on_battlefield(&engine, p1, hurloon_minotaur()).expect("seated");
    assert_eq!(
        pt(&engine, minotaur),
        (2, 3),
        "two damage alone would not kill it"
    );
    attack_and_collect_blocks(&mut engine, minotaur, p0);
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(strix, minotaur)],
            },
        )
        .expect("the Strix blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        on_battlefield(&engine, p1, hurloon_minotaur()).is_none(),
        "deathtouch destroyed the 2/3"
    );
}

/// Flying: a ground creature cannot block the Strix, a flier can.
#[test]
fn tidehollow_strix_cannot_be_blocked_by_a_ground_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[tidehollow_strix()], &[grizzly_bears(), storm_crow()]);
    let strix = on_battlefield(&engine, p0, tidehollow_strix()).expect("seated");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let crow = on_battlefield(&engine, p1, storm_crow()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, strix, p1);
    assert!(options.iter().any(|o| o.blocker == crow));
    assert!(!options.iter().any(|o| o.blocker == bears));
}
