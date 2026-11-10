//! `cards/enchantments/mv_4/blood_mist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn blood_mist() -> CardIndex {
    card_index("22188333-69b6-4685-b96b-b8a102ad5531")
}

/// "At the beginning of combat on your turn, target creature you control
/// gains double strike until end of turn.": the menu is my creatures only,
/// the target has double strike, the one I did not pick has not.
#[test]
fn blood_mist_gives_double_strike_to_the_chosen_creature_of_mine() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blood_mist(), grizzly_bears(), llanowar_elves()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let double = baylee_cards_dsl::KeywordSet::DOUBLE_STRIKE;
    assert!(
        !keywords(&engine, bears).contains(double),
        "not before combat"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(engine.state().turn.step, Step::CombatBegin);
    let menu = aim_at(&mut engine, p0, bears);
    assert!(menu.contains(&bears) && menu.contains(&elves));
    assert!(!menu.contains(&theirs), "only creatures I control");
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bears).contains(double), "the target");
    assert!(
        !keywords(&engine, elves).contains(double),
        "only the target"
    );
    assert!(!keywords(&engine, theirs).contains(double));
}
