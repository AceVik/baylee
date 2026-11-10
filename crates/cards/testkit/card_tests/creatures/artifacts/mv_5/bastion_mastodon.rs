//! `cards/creatures/artifacts/mv_5/bastion_mastodon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn bastion_mastodon() -> CardIndex {
    card_index("48a01671-da1f-4875-bcd5-ca99566ff399")
}

/// Bastion Mastodon — "{W}: This creature gains vigilance until end of turn."
/// White mana buys it, and it wears off at the next turn.
#[test]
fn bastion_mastodon_gains_vigilance_for_white_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bastion_mastodon(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mastodon = on_battlefield(&engine, p0, bastion_mastodon()).expect("seated");
    assert_eq!(pt(&engine, mastodon), (4, 5));
    assert!(!keywords(&engine, mastodon).contains(KeywordSet::VIGILANCE));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, bastion_mastodon(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, mastodon).contains(KeywordSet::VIGILANCE));

    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert!(
        !keywords(&engine, mastodon).contains(KeywordSet::VIGILANCE),
        "\"until end of turn\""
    );
}

/// The cost is {W}: with only a Forest the ability is not offered.
#[test]
fn bastion_mastodon_is_not_offered_without_white_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[bastion_mastodon(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mastodon = on_battlefield(&engine, p0, bastion_mastodon()).expect("seated");
    assert!(!priority_offer(&engine).abilities.contains(&(mastodon, 0)));
}
