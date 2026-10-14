//! `cards/enchantments/mv_1/thassa_s_ire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thassa_s_ire() -> CardIndex {
    card_index("fb3b365b-eb8c-4695-bfd6-88060ad20741")
}

/// "{3}{U}: You may tap or untap target creature." It toggles when taken, and
/// does nothing when declined.
#[test]
fn thassa_s_ire_toggles_the_target_when_taken() {
    for yes in [true, false] {
        let mut e = table(7271, &[], &[thassa_s_ire()], &[index::GRIZZLY_BEARS]);
        let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
        activate(&mut e, P0, thassa_s_ire(), 0);
        drive(&mut e, &[bears], &[], &[], yes);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        drive(&mut e, &[], &[], &[], yes);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(is_tapped(&e, bears), yes, "yes = {yes}");
    }
}
