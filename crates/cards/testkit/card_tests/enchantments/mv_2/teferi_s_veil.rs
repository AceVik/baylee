//! `cards/enchantments/mv_2/teferi_s_veil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn teferi_s_veil() -> CardIndex {
    card_index("c6db2ea4-954c-443b-9603-c41596555c31")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn is_phased_out(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .is_some_and(|o| o.status.contains(Status::PHASED_OUT))
}

/// Whenever a creature you control attacks, it phases out at end of combat (idle
/// creatures and opponent's attackers do not phase out) and phases back in
/// before you untap during your next untap step.
#[test]
fn teferi_s_veil_phases_out_your_attackers_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[teferi_s_veil(), grizzly_bears(), savannah_lions()],
        &[llanowar_elves()],
        &[],
        &[],
    );
    let bear = lk::obj(&e, p0, grizzly_bears());
    let lions = lk::obj(&e, p0, savannah_lions());
    let opp_elves = lk::obj(&e, p1, llanowar_elves());

    // Only Grizzly Bears attacks; Savannah Lions stays home.
    lk::attack(&mut e, &[bear]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    assert!(is_phased_out(&e, bear));
    assert!(!is_phased_out(&e, lions));

    // Opponent's Elves attacking on P1's turn does not phase out at end of combat.
    reach_their_main_phase(&mut e, p1);
    lk::attack(&mut e, &[opp_elves]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert!(!is_phased_out(&e, opp_elves));

    // Bear phases back in before P0 untaps on P0's next turn.
    reach_their_main_phase(&mut e, p0);
    assert!(!is_phased_out(&e, bear));
    assert!(!lk::tapped(&e, bear));
}
