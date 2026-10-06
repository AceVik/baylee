//! `cards/creatures/mv_5/ashaya_soul_of_the_wild.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ashaya, Soul of the Wild: the type change feeds the count that sizes it.
///
/// Both halves are one board. "Nontoken creatures you control are Forest
/// lands" makes Ashaya and the Elf beside it lands, and "power and toughness
/// each equal to the number of lands you control" then counts five: three
/// Forests, the Elf, and Ashaya itself. A reader applying the count before
/// the type change — or one that exempted the source from its own static —
/// would say three.
#[test]
fn ashaya_counts_the_creatures_its_own_static_made_into_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(384, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                ashaya_soul_of_the_wild(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ashaya = on_battlefield(&engine, p0, ashaya_soul_of_the_wild()).expect("Ashaya is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    assert!(
        engine
            .state()
            .object(elf)
            .expect("the Elf is still there")
            .characteristics()
            .types
            .intersects(TypeSet::LAND),
        "the Elf is a land in addition to its other types"
    );
    assert_eq!(
        pt(&engine, ashaya),
        (5, 5),
        "three Forests, the Elf and Ashaya itself are five lands, on a 0/0 body"
    );
}

/// A creature entering under Ashaya is counted as it arrives.
///
/// CR 613.1 applies continuous effects in a series of layers in order, so
/// by layer 7a (Ashaya's count) the Elf is already a Forest land from
/// layer 4. Ashaya was projected before the Elf, whose cache the move
/// had just cleared, and counted the Elf's printed types: 4/4 with five
/// lands out, until something else happened to refresh the board.
#[test]
fn ashaya_counts_a_creature_the_moment_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(384, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), ashaya_soul_of_the_wild()],
        )
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ashaya = on_battlefield(&engine, p0, ashaya_soul_of_the_wild()).expect("Ashaya is seated");
    assert_eq!(pt(&engine, ashaya), (4, 4), "three Forests and Ashaya");
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf resolved"
    );
    assert_eq!(
        pt(&engine, ashaya),
        (5, 5),
        "the Elf is a Forest land from the moment it is on the battlefield"
    );
    assert!(engine.projection_is_fresh(), "and nothing else is behind");
}

/// A count of a count: a creature that grows with the creatures you
/// control with power 6 or greater, ahead of Ashaya in the walk, when the
/// Elf entering behind both of them takes Ashaya from 5/5 to 6/6.
///
/// No card in the pool counts by power, so the count is registered by hand.
/// The refresh's first walk reads Ashaya as the last refresh left it and
/// the entering Elf as printed; projecting the counters again once puts
/// Ashaya at 6/6 but leaves the Elf ahead of it counting the 5/5 it read
/// a moment before. Only the repeat, while a counter moved, lets it see the
/// 6/6 (CR 613.1: a layer-7 count reads every object's layer 4 and the
/// counts before it). A refresh that stopped after one pass kept it 1/1.
#[test]
fn a_count_of_a_count_settles_in_the_refresh_that_moved_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(384, forest())
        .battlefield(
            0,
            &[
                quiet_creature(),
                ashaya_soul_of_the_wild(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let counter = engine.state.zones.list(ZoneLocation::Battlefield)[0];
    let ashaya = on_battlefield(&engine, p0, ashaya_soul_of_the_wild()).expect("Ashaya is seated");
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness sets boards up");
    let modifier = baylee_cards_dsl::Modifier::ModifyPTPerCount {
        filter: &baylee_cards_dsl::Filter::And(&[
            baylee_cards_dsl::Filter::YOUR_CREATURE,
            baylee_cards_dsl::Filter::PowerAtLeast(6),
        ]),
        p: 1,
        t: 1,
    };
    let filter = crate::effects::EffectFilter::object(state, counter);
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(counter),
        controller: p0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter,
        modifier,
    });
    state.refresh_characteristics();
    assert_eq!(
        pt(&engine, ashaya),
        (5, 5),
        "three Forests, the Elf and Ashaya"
    );
    assert_eq!(pt(&engine, counter), (1, 1), "nothing of yours has power 6");

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, ashaya), (6, 6), "the new Elf is a Forest too");
    assert_eq!(
        pt(&engine, counter),
        (2, 2),
        "and the counter counts the 6/6 Ashaya now is"
    );
    assert!(engine.projection_is_fresh(), "and nothing else is behind");
}

/// Ashaya's "power and toughness are each equal to the number of lands you
/// control" is a characteristic-defining ability, so it applies in layer
/// 7a (CR 613.4a) and an effect that **sets** power and toughness applies
/// after it (613.4b).
///
/// Living Lands makes every Forest a 1/1 creature, and Ashaya is a Forest by
/// its own second ability. Four lands define it as 4/4 and Living Lands then
/// sets it to 1/1. A count added on top at 7c, which is how the card was
/// written before, reads 5/5.
#[test]
fn ashaya_is_defined_before_living_lands_sets_it_to_one_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(384, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                ashaya_soul_of_the_wild(),
                living_lands(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ashaya = on_battlefield(&engine, p0, ashaya_soul_of_the_wild()).expect("Ashaya is seated");
    let forest_id = on_battlefield(&engine, p0, forest()).expect("a Forest");
    assert_eq!(pt(&engine, forest_id), (1, 1), "Living Lands' Forest");
    assert_eq!(
        pt(&engine, ashaya),
        (1, 1),
        "7a defines 4/4 from four lands, and 7b sets 1/1 over it"
    );
}

/// A card that defines its own size is that size the moment it is drawn.
/// The draw clears the card's projection, and nothing else on this board
/// moves to start another refresh, so without the draw asking for one the
/// Ashaya in hand read its printed 0/0 until a land was played.
#[test]
fn a_drawn_ashaya_is_its_size_in_hand_at_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[ashaya_soul_of_the_wild()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let card = in_hand(&engine, p0, ashaya_soul_of_the_wild()).expect("Ashaya starts in hand");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            card,
            ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("onto the top of the library");
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "p0's next main phase");
    let drawn = in_hand(&engine, p0, ashaya_soul_of_the_wild()).expect("drawn again");
    assert_eq!(pt(&engine, drawn), (2, 2), "as big as the two Plains");
}
