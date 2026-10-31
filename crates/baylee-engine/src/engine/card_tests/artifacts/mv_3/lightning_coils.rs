//! Lightning Coils counts nontoken deaths and turns five counters into tokens.

use super::*;
use baylee_core::generated::subtypes;

fn lightning_coils() -> CardIndex {
    card_index("bbb16f23-fdb1-444b-bef8-ce3e177c2678")
}

fn elementals(e: &Engine<RegistryLookup>) -> usize {
    e.state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|object| {
                object
                    .characteristics()
                    .subtypes
                    .contains(subtypes::creature::ELEMENTAL)
            })
        })
        .count()
}

#[test]
fn lightning_coils_gains_a_counter_when_our_nontoken_creature_dies() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[lightning_coils(), gray_ogre(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut e, p0, lightning_bolt());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let coils = on_battlefield(&e, p0, lightning_coils()).unwrap();
    assert_eq!(
        e.state()
            .object(coils)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );
}

#[test]
fn lightning_coils_removes_five_counters_makes_tokens_and_exiles_them_at_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[lightning_coils()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let coils = on_battlefield(&e, p0, lightning_coils()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(coils)
        .unwrap()
        .counters
        .set(CounterKind::Charge, 5);
    e.refresh_offer();
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert_eq!(elementals(&e), 5);
    assert_eq!(
        e.state()
            .object(coils)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );

    pass_until(&mut e, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::End
            && stack_is_empty(e)
    });
    assert_eq!(elementals(&e), 0);
}
