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

fn charge(e: &Engine<RegistryLookup>) -> u16 {
    let coils = on_battlefield(e, PlayerId::new(0), lightning_coils()).unwrap();
    e.state()
        .object(coils)
        .unwrap()
        .counters
        .get(CounterKind::Charge)
}

fn set_charge(e: &mut Engine<RegistryLookup>, n: u16) {
    let p0 = PlayerId::new(0);
    let coils = on_battlefield(e, p0, lightning_coils()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(coils)
        .unwrap()
        .counters
        .set(CounterKind::Charge, n);
    e.refresh_offer();
}

/// "A nontoken creature you control": the opponent's creature dying adds
/// nothing, and the same death on our side, on the same board, adds one.
#[test]
fn lightning_coils_ignores_an_opponent_s_creature_dying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[lightning_coils(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let theirs = on_battlefield(&e, p1, gray_ogre()).unwrap();
    kill(&mut e, theirs);
    assert!(in_graveyard(&e, p1, gray_ogre()).is_some());
    assert_eq!(charge(&e), 0, "not a creature we control");
    let ours = on_battlefield(&e, p0, gray_ogre()).unwrap();
    kill(&mut e, ours);
    assert_eq!(charge(&e), 1, "the control: our own Ogre counts");
}

/// "Nontoken": one of the Coils' own Elementals dying adds no counter.
#[test]
fn lightning_coils_ignores_our_token_dying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[lightning_coils()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    set_charge(&mut e, 5);
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert_eq!(elementals(&e), 5);
    assert_eq!(charge(&e), 0);
    let token = e
        .state
        .battlefield_seen()
        .find(|id| {
            e.state.object(*id).is_some_and(|object| {
                object
                    .characteristics()
                    .subtypes
                    .contains(subtypes::creature::ELEMENTAL)
            })
        })
        .unwrap();
    kill(&mut e, token);
    assert_eq!(elementals(&e), 4);
    assert_eq!(charge(&e), 0, "a token is not a nontoken creature");
}

/// "Five or more": four counters make nothing at the upkeep and stay.
#[test]
fn lightning_coils_with_four_counters_makes_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[lightning_coils()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    set_charge(&mut e, 4);
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert_eq!(elementals(&e), 0);
    assert_eq!(charge(&e), 4);
}

/// The Elementals it makes have haste.
#[test]
fn lightning_coils_elementals_have_haste() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[lightning_coils()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    set_charge(&mut e, 5);
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    let tokens: Vec<ObjectId> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|object| {
                object
                    .characteristics()
                    .subtypes
                    .contains(subtypes::creature::ELEMENTAL)
            })
        })
        .collect();
    assert_eq!(tokens.len(), 5);
    assert!(
        tokens
            .iter()
            .all(|&token| keywords(&e, token).contains(KeywordSet::HASTE))
    );
}
