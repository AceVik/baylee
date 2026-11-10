//! `cards/lands/deserts/desert_of_the_fervent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Desert of the Fervent prints three lines and one first main phase plays all
/// three. "This land enters tapped" is read off a real land drop, with the
/// copy the harness *placed* on the battlefield as the control: that one comes
/// through `starting_battlefield`, which is a placement and not an entry, so
/// the same card stands untapped there and tapped here. `{T}: Add {R}` is read
/// off a pool of exactly three red — two Mountains and that Desert, all three
/// of whose whole price is their own tap — and Cycling `{1}{R}` is a real
/// payment: two mana leave the pool, the card goes from hand to graveyard and
/// the draw takes one card off the top of the library.
#[test]
fn desert_of_the_fervent_enters_tapped_taps_for_red_and_cycles_for_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), desert_of_the_fervent()])
        .hand(0, &[desert_of_the_fervent(), desert_of_the_fervent()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The same card arriving two ways. `SeatSpec::starting_battlefield` places
    // a permanent rather than having it enter, so it is standing untapped; the
    // land drop below is an entry and is not.
    let placed =
        on_battlefield(&engine, p0, desert_of_the_fervent()).expect("the seeded copy is out");
    assert!(
        !is_tapped(&engine, placed),
        "placed by the harness, which is not entering"
    );
    let played = play_land(&mut engine, p0, desert_of_the_fervent());
    assert!(
        is_tapped(&engine, played),
        "\"this land enters tapped\" — a real land drop and not a placement"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Two Mountains and the Desert, each of whose whole price is its own
    // `{T}`, so the kit takes all three. Three red is the Desert's own {R}
    // counted in: the pair of Mountains alone could only ever be two.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        3,
        "two Mountains and the Desert's own {{T}}: Add {{R}}"
    );
    assert_eq!(pool.total(), 3, "and no source made anything else");

    // Cycling is offered on the *card in hand*, which is the object this
    // names — the seeded Desert on the table has the same card index and its
    // own ability 0, which is why the lookup is by object and not by card.
    let card = in_hand(&engine, p0, desert_of_the_fervent()).expect("the second copy is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == card)
        .expect("cycling {1}{R} is offered from hand once the mana is floating");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the mana the Mountains and the Desert made pays {1}{R}");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{1}}{{R}} came out of the pool the three red went into"
    );
    assert!(
        in_graveyard(&engine, p0, desert_of_the_fervent()).is_some(),
        "\"discard this card\": the cycled copy is in its owner's graveyard"
    );
    assert!(
        in_hand(&engine, p0, desert_of_the_fervent()).is_none(),
        "and it left the hand to get there"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" took one card off the top"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one discarded and one drawn, so the hand is the size it was"
    );
}
