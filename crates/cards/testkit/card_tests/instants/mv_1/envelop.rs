//! `cards/instants/mv_1/envelop.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Envelop costs {U} and, according to its own text, counters a
/// sorcery spell. A counter can only be read from what *does not happen*
/// afterwards: Wheel of Fortune would have made each player discard their
/// hand and draw seven cards — so p0's unchanged library and hand are one
/// half of the claim and p1's graveyard the other. Dark Ritual lies as
/// an instant over the same sorcery on the same stack and must be absent
/// from the target menu; otherwise `Filter::HasType(SORCERY)` reads
/// nothing at all and the spell counters every spell. The {U} is paid only
/// after the target choice (CR 601.2c before CR 601.2h), which the pool
/// shows after the response.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn envelop_counters_the_sorcery_and_declines_the_instant_beside_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[envelop()])
        .battlefield(1, &[mountain(), mountain(), mountain(), swamp()])
        .hand(1, &[wheel_of_fortune(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    // Three Mountains pay {2}{R}; the Swamp stays untapped so that the
    // instant above it does not fail because of the color of the rest.
    let swamp_obj = on_battlefield(&engine, p1, swamp()).expect("the Swamp is untapped");
    tap_mana_where(&mut engine, p1, |id| id != swamp_obj);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        3,
        "three Mountains, three mana for the sorcery"
    );
    cast_with_floating(&mut engine, p1, wheel_of_fortune());
    assert!(
        on_stack(&engine, wheel_of_fortune()).is_some(),
        "the sorcery waits on the stack"
    );

    // The active player retains priority after casting (CR 117.3c) and
    // casts another instant in response to the same sorcery.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_mana_where(&mut engine, p1, |id| id == swamp_obj);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the Swamp, one black mana"
    );
    cast_with_floating(&mut engine, p1, dark_ritual());
    pass_until(&mut engine, |e| {
        on_stack(e, dark_ritual()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let wheel_spell = on_stack(&engine, wheel_of_fortune()).expect("the sorcery is still there");
    let ritual_spell = on_stack(&engine, dark_ritual()).expect("der Instant liegt darüber");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // First mana into the pool, then the assertion: `castable` reads the pool
    // and not the untapped Islands.
    tap_mana_where(&mut engine, p0, |_| true);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "two Islands, two blue mana"
    );
    let counterspell = in_hand(&engine, p0, envelop()).expect("Envelop is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("p0 holds priority, not {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&counterspell),
        "with {{U}} in the pool the counterspell is playable: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: counterspell })
        .expect("der Konter ist angekündigt");
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target sorcery spell\" is a target choice, not {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the casting player chooses the target (CR 601.2c)"
    );
    assert_eq!(
        options,
        vec![wheel_spell],
        "the sorcery and only it: the instant above is a spell, not a sorcery"
    );
    assert!(
        !options.contains(&ritual_spell),
        "HasType(SORCERY) is read, not skipped"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wheel_spell],
            },
        )
        .expect("the sorcery was one of the options");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the cost comes only after the target (CR 601.2h): the {{U}} is paid"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, envelop()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, wheel_of_fortune()).is_some(),
        "the countered spell is in p1's graveyard (CR 701.6a)"
    );
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "der Instant unter dem Konter ist unberührt verrechnet — er zeigt auf niemanden"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "the Wheel would have drawn seven cards each: none of that happened"
    );
    // One card fewer, and the one it lost is the counter it cast: the Wheel
    // would have taken the whole hand away and dealt seven back (CR 701.5a
    // is why none of it happened), so "the same size" would have been the
    // wrong claim as well as a false one.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the hand lost the Envelop and nothing else — no discard, no draw"
    );
}
