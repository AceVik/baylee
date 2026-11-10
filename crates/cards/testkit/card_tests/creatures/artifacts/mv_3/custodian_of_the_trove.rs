//! `cards/creatures/artifacts/mv_3/custodian_of_the_trove.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn custodian_of_the_trove() -> CardIndex {
    card_index("0d060d19-9c8b-4831-a9be-aa2c98b6858a")
}

/// Custodian of the Trove — 2/5 defender that enters tapped: cast, it is on
/// the battlefield tapped; untapped next turn it still is not offered as an
/// attacker while the plain creature beside it is.
#[test]
fn custodian_of_the_trove_enters_tapped_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(
        &[custodian_of_the_trove()],
        &[forest(), forest(), forest(), quiet_creature()],
    );
    cast_and_settle(&mut engine, custodian_of_the_trove());
    let custodian = on_battlefield(&engine, p0, custodian_of_the_trove()).expect("it resolved");
    assert_eq!(pt(&engine, custodian), (2, 5));
    assert!(keywords(&engine, custodian).contains(KeywordSet::DEFENDER));
    assert!(is_tapped(&engine, custodian), "it enters tapped");

    // Through this turn's combat (it is tapped and not offered), then on to
    // the next turn of seat 0, where the untap step has freed it.
    let first = attackers_offered(&mut engine);
    assert!(!first.contains(&custodian));
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number > 1
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0)
    });
    assert!(!is_tapped(&engine, custodian), "it untapped");
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    let plain = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    assert!(attackers.contains(&plain));
    assert!(
        !attackers.contains(&custodian),
        "untapped and unsick, defender still keeps it home"
    );
}
