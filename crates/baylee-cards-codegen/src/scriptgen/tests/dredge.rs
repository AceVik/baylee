//! Unit tests for reading `K:Dredge:<N>` (CR 702.52a), on made-up scripts.

use super::*;

#[test]
fn dredge_is_the_replacement_rule_with_its_count() {
    let body = read(
        "Name:Dredger\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ GainLife | LifeAmount$ 2\n\
         K:Dredge:3",
    );
    assert!(
        body.abilities
            .iter()
            .any(|a| a == "AbilityDef::Replacement(ReplacementRule::Dredge(3))"),
        "{:?}",
        body.abilities
    );
}

#[test]
fn a_dredge_without_a_count_is_refused_by_name() {
    for (keyword, expected) in [
        ("Dredge:X", "a `Dredge` count `X`"),
        ("Dredge:0", "a `Dredge` count `0`"),
    ] {
        let parsed = parse(&format!(
            "Name:Dredger\nManaCost:1 G\nTypes:Sorcery\nA:SP$ GainLife | LifeAmount$ 2\nK:{keyword}"
        ));
        assert!(
            transcode(&parsed, &cats(), None).is_none(),
            "silently accepted: {keyword}"
        );
        let reason = refusal_reason(&parsed, &cats(), None).unwrap_or_default();
        assert!(
            reason.contains(expected),
            "{keyword}: expected `{expected}`, got `{reason}`"
        );
    }
}
