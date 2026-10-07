//! `cards/instants/mv_3/scrap.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scrap is `{2}{R}` for "Destroy target artifact" and "Cycling {2} ({2},
/// Discard this card: Draw a card.)" — two halves that each consume the card
/// carrying them, so two copies are dealt and played in one main phase. The
/// first is aimed at the Sol Ring across the table, which is the game's only
/// artifact and therefore the whole of what `Filter::ARTIFACT` may offer: the
/// Llanowar Elves beside it is a permanent of another kind and has to stay off
/// that menu. The second is cycled out of **hand**, the zone
/// `ActivationZone::Hand` puts the ability in, and because `can_afford` reads
/// the mana already floating rather than the five untapped Mountains the pool
/// is taken first: `{2}{R}` leaves exactly the `{2}` the cycling then charges,
/// and the pool reads empty once both prices are paid.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn scrap_destroys_the_artifact_it_names_and_cycles_itself_away_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[scrap(), scrap()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // Five Mountains into the pool: `{2}{R}` for the cast and the `{2}` the
    // cycling charges are one payment inside one main phase (CR 500.5), and
    // `can_afford` reads the pool rather than the untapped lands. Nothing else
    // on this board makes mana — the Sol Ring belongs to the other seat, and
    // cycling's own price is not a tap, so the helper leaves the card alone.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains tapped, and five red in the pool"
    );

    // The first half: "Destroy target artifact." Targets are named before
    // costs are paid (CR 601.2c, then CR 601.2h), so the artifact is still on
    // the battlefield and the pool is still full while the question stands.
    let spell = in_hand(&engine, p0, scrap()).expect("the first Scrap is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "{{2}}{{R}} is payable out of the pool, so the instant is castable: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, scrap());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one artifact, and the spell asks once");
    assert!(
        options.contains(&ring),
        "the Sol Ring is an artifact and the spell may point at it: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and it is the only one in the game: the Elf beside it is a permanent \
         of another kind, and every other permanent on the table is a Mountain \
         — {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "`Filter::ARTIFACT` is read and not skipped: a creature is no artifact: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "CR 601.2c before CR 601.2h: the target is named while its price is \
         still unpaid"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the artifact was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy target artifact\": the named artifact is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and the permanent the spell never named is still standing"
    );
    assert!(
        in_graveyard(&engine, p0, scrap()).is_some(),
        "the instant resolved and went to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five mana less the {{2}}{{R}} the cast cost: exactly the {{2}} the \
         cycling charges is still floating"
    );

    // The second half. Cycling is an ability the card offers *from hand*
    // (`ActivationZone::Hand`), so the ability's source is the card in the
    // hand rather than a permanent, and its `{2}` is read off the pool the
    // cast left behind.
    let discard = in_hand(&engine, p0, scrap()).expect("the second Scrap is in hand");
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == discard)
        .expect("cycling is offered on the card in hand, which is where it lives");
    assert_eq!(
        source, discard,
        "the ability belongs to the very card that pays for it"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("{{2}} is in the pool and the card is in hand, so cycling is activatable");

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&discard),
        "\"Discard this card\" is half the cost and is paid on announcement \
         (CR 601.2h), which for a card is its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the cycling ability waits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    assert_eq!(
        hand_after.len(),
        hand_before.len(),
        "one card discarded and one card drawn: the hand is the length it was"
    );
    assert!(
        !hand_after.contains(&discard),
        "the card that paid the cost is not in the hand it left"
    );
    let drawn: Vec<ObjectId> = hand_after
        .iter()
        .copied()
        .filter(|id| !hand_before.contains(id))
        .collect();
    assert_eq!(
        drawn.len(),
        1,
        "exactly one new card, so the draw happened once and not twice"
    );
    assert!(
        library_before.contains(&drawn[0]),
        "and it came off the library rather than out of nowhere"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .contains(&drawn[0]),
        "\"Draw a card\": the card left the library for the hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "one card off the top of the library, which is the whole of the effect"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cycling's {{2}} came out of the pool the Mountains filled, so both \
         printed prices were really paid"
    );
}
