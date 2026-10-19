//! `cards/creatures/mv_2/elvish_hunter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn fill_pools(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn is_tapped(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .expect("the permanent is out")
        .status
        .contains(Status::TAPPED)
}

/// The Hunter aimed at player 1's tapped Bears, resolved.
fn hunted() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card_index("49b351d8-8b63-4f9e-a323-344ed35fdfff")])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let hunter = on_battlefield(&e, P0, card_index("49b351d8-8b63-4f9e-a323-344ed35fdfff"))
        .expect("the Hunter is out");
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("the Bears are out");
    e.state.set_tapped(bears, true);

    fill_pools(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: hunter,
            ability_index: 0,
        },
    )
    .expect("{1}{G}, {T} is paid");
    let menu = aim_at(&mut e, P0, bears);
    assert!(
        menu.contains(&bears) && menu.contains(&hunter),
        "any creature"
    );
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, hunter), "the Hunter paid {{T}}");
    (e, hunter, bears)
}

/// Elvish Hunter (49b351d8-8b63-4f9e-a323-344ed35fdfff) — "{1}{G}, {T}: Target
/// creature doesn't untap during its controller's next untap step."
#[test]
fn elvish_hunter_keeps_the_creature_tapped_through_its_next_untap_step() {
    let (mut e, _hunter, bears) = hunted();
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    assert!(is_tapped(&e, bears), "it did not untap");
}

#[test]
fn elvish_hunter_only_holds_one_untap_step() {
    let (mut e, _hunter, bears) = hunted();
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    assert!(is_tapped(&e, bears), "still tapped on my turn");
    pass_until(&mut e, |e| e.state().turn.active == P1);
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Main
    });
    assert!(
        !is_tapped(&e, bears),
        "the hold covered one untap step only, so it untaps in the next"
    );
}

#[test]
fn elvish_hunter_a_creature_not_hunted_untaps_normally() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card_index("49b351d8-8b63-4f9e-a323-344ed35fdfff")])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("the Bears are out");
    e.state.set_tapped(bears, true);
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    assert!(!is_tapped(&e, bears), "the control case untaps");
}
