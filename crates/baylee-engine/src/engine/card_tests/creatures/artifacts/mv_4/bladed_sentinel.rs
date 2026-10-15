//! `cards/creatures/artifacts/mv_4/bladed_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn bladed_sentinel() -> CardIndex {
    card_index("00f579f4-2d18-4c7c-91b2-34726b384535")
}

/// Declares `sentinel` the lone attacker on seat 0's first turn and returns
/// whether it is tapped once declared.
fn tapped_after_attacking(engine: &mut Engine<RegistryLookup>, sentinel: ObjectId) -> bool {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sentinel, Defender::Player(p1))],
            },
        )
        .expect("the Sentinel attacks");
    is_tapped(engine, sentinel)
}

/// Bladed Sentinel — {4} 2/4: "{W}: This creature gains vigilance until end
/// of turn." Paid for, an attacking Sentinel stays untapped; unpaid, the same
/// attack taps it.
#[test]
fn bladed_sentinel_attacks_without_tapping_once_it_has_vigilance() {
    let p0 = PlayerId::new(0);

    let mut paid = Duel::new(SEED, forest())
        .battlefield(0, &[bladed_sentinel(), plains()])
        .start();
    keep_mulligans(&mut paid);
    reach_main_phase(&mut paid, p0);
    let sentinel = on_battlefield(&paid, p0, bladed_sentinel()).expect("seated");
    assert!(!keywords(&paid, sentinel).contains(KeywordSet::VIGILANCE));
    tap_all_mana(&mut paid, p0);
    activate(&mut paid, p0, bladed_sentinel(), 0);
    pass_until(&mut paid, stack_is_empty);
    assert!(keywords(&paid, sentinel).contains(KeywordSet::VIGILANCE));
    assert!(
        !tapped_after_attacking(&mut paid, sentinel),
        "vigilance: attacking does not tap it"
    );

    let mut unpaid = Duel::new(SEED, forest())
        .battlefield(0, &[bladed_sentinel(), plains()])
        .start();
    keep_mulligans(&mut unpaid);
    reach_main_phase(&mut unpaid, p0);
    let sentinel = on_battlefield(&unpaid, p0, bladed_sentinel()).expect("seated");
    assert!(
        tapped_after_attacking(&mut unpaid, sentinel),
        "the control: without {{W}} the attack taps it"
    );
}
