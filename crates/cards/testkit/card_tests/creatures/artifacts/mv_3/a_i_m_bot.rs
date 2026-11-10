//! `cards/creatures/artifacts/mv_3/a_i_m_bot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn a_i_m_bot() -> CardIndex {
    card_index("bd269d87-5f72-4559-bf2b-270e2ebe6154")
}

/// A.I.M. Bot — {2}{U}, 2/3 artifact creature with flying: cast off three
/// Islands it resolves as exactly that body.
#[test]
fn a_i_m_bot_resolves_as_a_two_three_flying_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[a_i_m_bot()], &[island(), island(), island()]);
    assert!(in_hand(&engine, p0, a_i_m_bot()).is_some());

    cast_and_settle(&mut engine, a_i_m_bot());

    let bot = on_battlefield(&engine, p0, a_i_m_bot()).expect("it resolved");
    assert_eq!(pt(&engine, bot), (2, 3));
    let t = types(&engine, bot);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(keywords(&engine, bot).contains(KeywordSet::FLYING));
}

/// Flying means "can't be blocked except by creatures with flying or reach":
/// seat 1's ground creature may block the plain attacker beside the Bot and
/// not the Bot itself.
#[test]
fn a_i_m_bot_cannot_be_blocked_by_a_ground_creature() {
    let (engine, [bot, ground], options) = evasion_probe(a_i_m_bot(), &[quiet_creature()]);
    let blocker = on_battlefield(&engine, PlayerId::new(1), quiet_creature()).expect("seated");
    assert!(
        may_block(&options, blocker, ground),
        "the same blocker may block a ground attacker"
    );
    assert!(!may_block(&options, blocker, bot));
}
