//! `cards/artifacts/mv_2/genesis_chamber.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn genesis_chamber() -> CardIndex {
    card_index("150ab025-5cc3-4468-a724-bbba8838445d")
}

fn myr_tokens_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == seat
                    && o.card.is_none()
                    && o.characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::creature::MYR)
            })
        })
        .collect()
}

/// Genesis Chamber (150ab025-5cc3-4468-a724-bbba8838445d) — {2} Artifact:
/// "Whenever a nontoken creature enters, if this artifact is untapped, that
///  creature's controller creates a 1/1 colorless Myr artifact creature token."
///
/// Creates a 1/1 colorless Myr artifact creature token for whichever player
/// had a nontoken creature enter (never re-triggering on the Myr token itself),
/// and does not trigger while Genesis Chamber is tapped.
#[test]
fn genesis_chamber_makes_myr_for_nontoken_creatures_only_when_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[genesis_chamber(), relic_barrier(), forest(), forest()])
        .hand(0, &[llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let chamber = on_battlefield(&engine, p0, genesis_chamber()).unwrap();

    // 1. p0 casts a nontoken creature while Genesis Chamber is untapped -> p0 gets 1 Myr.
    let my_forests = forests(&engine, p0, 2);
    tap_mana_where(&mut engine, p0, |id| id == my_forests[0]);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let p0_myrs = myr_tokens_of(&engine, p0);
    assert_eq!(
        p0_myrs.len(),
        1,
        "exactly 1 Myr token (Myr token ETB did not re-trigger Genesis Chamber)"
    );
    let myr = engine.state().object(p0_myrs[0]).unwrap();
    assert_eq!(pt(&engine, p0_myrs[0]), (1, 1));
    assert!(myr.characteristics().types.contains(
        baylee_core::types::TypeSet::ARTIFACT.union(baylee_core::types::TypeSet::CREATURE)
    ));
    assert!(
        myr.characteristics().colors.is_empty(),
        "Myr token is colorless"
    );

    // 2. Tap Genesis Chamber with Relic Barrier, then p0 casts another nontoken creature -> no Myr!
    activate(&mut engine, p0, relic_barrier(), 0);
    aim_at(&mut engine, p0, chamber);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, chamber));

    tap_mana_where(&mut engine, p0, |id| id == my_forests[1]);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        myr_tokens_of(&engine, p0).len(),
        1,
        "tapped Genesis Chamber does not create a Myr"
    );

    // 3. Untap Genesis Chamber on p0's next turn, then on p1's turn p1 casts Llanowar Elves -> p1 gets 1 Myr!
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_their_main_phase(&mut engine, p1);
    assert!(!is_tapped(&engine, chamber));

    cast_from_hand(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        myr_tokens_of(&engine, p1).len(),
        1,
        "opponent receives a 1/1 Myr when their nontoken creature enters"
    );
    assert_eq!(myr_tokens_of(&engine, p0).len(), 1);
}
