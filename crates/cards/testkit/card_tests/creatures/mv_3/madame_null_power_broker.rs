//! `cards/creatures/mv_3/madame_null_power_broker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn madame() -> CardIndex {
    card_index("73413bd5-8c40-4d45-8f05-c20c39ee4017")
}

#[track_caller]
fn cast_bears_to_payment(e: &mut Engine<RegistryLookup>) -> ObjectId {
    lk::float(e, P0, &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)]);
    cast_with_floating(e, P0, grizzly_bears());
    // A real target choice here would be wrong: the entering creature is an
    // implicit event object, not a target of Madame's triggered ability.
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player: P0,
                prompt: YesNoPrompt::PayLife { .. },
                ..
            }
        )
    });
    lk::obj(e, P0, grizzly_bears())
}

#[test]
fn madame_null_plays_deathtouch_in_combat() {
    let mut e = lk::game(SEED, &[madame()], &[rootbreaker_wurm()], &[], &[]);
    let attacker = lk::obj(&e, P0, madame());
    let blocker = lk::obj(&e, P1, rootbreaker_wurm());
    assert!(keywords(&e, attacker).contains(KeywordSet::DEATHTOUCH));
    lk::attack(&mut e, &[attacker]);
    lk::block(&mut e, &[(blocker, attacker)]);
    lk::through_combat(&mut e);
    assert_eq!(
        lk::zone_of(&e, blocker),
        Zone::Graveyard,
        "one damage was lethal"
    );
}

#[test]
fn madame_null_charges_current_power_and_puts_that_many_counters_on_the_entrant() {
    for (anthem, season, price, expected) in [(true, false, 3, 3), (false, true, 2, 4)] {
        let mut board = vec![madame()];
        if anthem {
            board.push(baylee_core::generated::index::GLORIOUS_ANTHEM);
        }
        if season {
            board.push(doubling_season());
        }
        let mut e = lk::game(SEED, &board, &[], &[grizzly_bears()], &[]);
        let bears = cast_bears_to_payment(&mut e);
        let Pending::YesNo {
            prompt: YesNoPrompt::PayLife { amount, .. },
            ..
        } = e.pending().clone()
        else {
            unreachable!()
        };
        assert_eq!(amount, price, "current power, after static bonuses");
        e.apply(P0, PlayerAction::YesNo(true)).unwrap();
        lk::settle(&mut e);

        assert_eq!(lk::life(&e, P0), 20 - i32::try_from(price).unwrap());
        assert_eq!(
            counters_on(&e, bears, CounterKind::P1P1),
            expected,
            "Doubling Season doubles counters, never the life price"
        );
        assert_eq!(lk::zone_of(&e, bears), Zone::Battlefield);
    }
}

#[test]
fn madame_null_declining_the_payment_leaves_life_and_counters_untouched() {
    let mut e = lk::game(SEED, &[madame()], &[], &[grizzly_bears()], &[]);
    let bears = cast_bears_to_payment(&mut e);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, P0), 20);
    assert_eq!(counters_on(&e, bears, CounterKind::P1P1), 0);
}

#[test]
fn madame_null_does_not_offer_an_unpayable_price() {
    let mut e = lk::game(SEED, &[madame()], &[], &[grizzly_bears()], &[]);
    e.dev_state_mut(P0).unwrap().players[0].life = 1;
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    cast_with_floating(&mut e, P0, grizzly_bears());
    lk::settle(&mut e);

    let bears = lk::obj(&e, P0, grizzly_bears());
    assert_eq!(lk::life(&e, P0), 1);
    assert_eq!(counters_on(&e, bears, CounterKind::P1P1), 0);
}

#[test]
fn madame_null_ignores_herself_and_creatures_entering_for_an_opponent() {
    let mut own = lk::game(SEED, &[], &[], &[madame()], &[]);
    lk::float(
        &mut own,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut own, P0, madame());
    lk::settle(&mut own);
    assert_eq!(lk::life(&own, P0), 20);
    assert_eq!(
        counters_on(&own, lk::obj(&own, P0, madame()), CounterKind::P1P1),
        0
    );

    let mut theirs = lk::game(SEED, &[madame()], &[], &[], &[grizzly_bears()]);
    reach_their_main_phase(&mut theirs, P1);
    lk::float(
        &mut theirs,
        P1,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    cast_with_floating(&mut theirs, P1, grizzly_bears());
    lk::settle(&mut theirs);
    assert_eq!(lk::life(&theirs, P0), 20);
    assert_eq!(
        counters_on(
            &theirs,
            lk::obj(&theirs, P1, grizzly_bears()),
            CounterKind::P1P1
        ),
        0
    );
}
