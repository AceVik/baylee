//! `cards/enchantments/mv_1/hopeless_nightmare.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hopeless_nightmare() -> CardIndex {
    card_index("5e63b69c-d901-4a01-8dfa-88e044856239")
}

/// "When this enchantment enters, each opponent discards a card and loses 2
/// life." and "{2}{B}: Sacrifice this enchantment." with its scry 2.
#[test]
fn hopeless_nightmare_drains_on_entry_and_scries_when_sacrificed() {
    let mut e = Duel::new(7201, forest())
        .hand(0, &[hopeless_nightmare()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    fill_pool(&mut e);
    let hand_before = hand_size(&e, P1);
    cast_with_floating(&mut e, P0, hopeless_nightmare());
    drive_through(&mut e);
    assert_eq!(life(&e, P1), 18);
    assert_eq!(hand_size(&e, P1), hand_before - 1);
    assert!(in_graveyard(&e, P1, index::GRIZZLY_BEARS).is_some());
    assert_eq!(life(&e, P0), 20);

    fill_pool(&mut e);
    activate(&mut e, P0, hopeless_nightmare(), 2);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Arrange { .. }) || stack_is_empty(e)
    });
    assert!(in_graveyard(&e, P0, hopeless_nightmare()).is_some());
    let Pending::Arrange { player, .. } = e.pending().clone() else {
        panic!("scry expected, got {:?}", e.pending())
    };
    assert_eq!(player, P0);
}

/// Answers whatever the trigger asks and lets the stack empty.
fn drive_through(e: &mut Engine<RegistryLookup>) {
    for _ in 0..8 {
        drive(e, &[], &[], &[], true);
        if stack_is_empty(e) {
            return;
        }
        pass_until(e, |e| {
            !matches!(e.pending(), Pending::Priority { .. }) || stack_is_empty(e)
        });
    }
}
