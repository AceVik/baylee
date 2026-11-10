//! `cards/creatures/mv_5/kenrith_the_returned_king.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kenrith's black line: "{4}{B}: Put target creature card from a graveyard
/// onto the battlefield under its owner's control."
///
/// Both halves are read on one board. "A graveyard" names no player, so an
/// Elf in each graveyard is offered; the one taken is the opponent's, and
/// "its owner's control" puts it on their side of the table, where the
/// ordinary reanimation sentence would have put it on Kenrith's.
#[test]
fn kenrith_returns_a_creature_card_from_any_graveyard_to_its_owner() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(
            0,
            &[
                kenrith_the_returned_king(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let mine = in_graveyard(&engine, p0, llanowar_elves()).expect("an Elf in p0's graveyard");
    let theirs = in_graveyard(&engine, p1, llanowar_elves()).expect("an Elf in p1's graveyard");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, kenrith_the_returned_king(), 4);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the graveyard target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"from a graveyard\": both graveyards are offered — {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("a creature card in a graveyard is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "\"under its owner's control\": the Elf came back on p1's side"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and not on Kenrith's"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "p0's own Elf was not the target and stayed where it was"
    );
}

/// Kenrith is a 5/5 for {4}{W} whose red and green lines are both written,
/// and this scenario plays them off one first main phase. The red one is the
/// reason the test exists: "{R}: All creatures gain trample and haste until
/// end of turn" names no controller, so the pump lands on the Elves across
/// the table exactly as it lands on Kenrith's own side — the same crossing
/// the Liquimetal Coating test guards, one activation cheaper. The `0/0` in
/// the card is asserted with the keywords so a pump that had also moved P/T
/// could not pass. The green one is the contrast: "{1}{G}: Put a +1/+1
/// counter on **target** creature" reaches either side of the table and is
/// answered with one, so the counter has to appear on the creature that was
/// named and on none of the others it was offered.
#[allow(clippy::too_many_lines)] // one ability, the whole board read before and after it
#[test]
fn kenrith_pumps_every_creature_and_marks_only_the_one_he_aimed_at() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                forest(),
                forest(),
                forest(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[kenrith_the_returned_king()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main"
    );

    // The King is cast off the Plains and one Forest alone, leaving the
    // Mountains and the other Forests in reserve: a mana pool empties when a
    // step ends (CR 500.5), and the colours left floating are the colours
    // the two abilities below are paid with.
    for land in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
    }
    let grove = all_on_battlefield(&engine, p0, forest());
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: grove[0] })
        .unwrap();
    let king_card = in_hand(&engine, p0, kenrith_the_returned_king()).expect("the King is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: king_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let king = on_battlefield(&engine, p0, kenrith_the_returned_king()).expect("the King resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, king), (5, 5), "the body the card prints");

    // {R}: the Mountains pay it, and "{R}" is the whole cost.
    for mountain in all_on_battlefield(&engine, p0, mountain()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: mountain })
            .unwrap();
    }
    // Ability 0 is the red team pump; 1 is the counter, and 2 and 3 are the
    // life and the draw.
    activate(&mut engine, p0, kenrith_the_returned_king(), 0);
    pass_until(&mut engine, stack_is_empty);

    for (who, creature) in [
        ("the King", king),
        ("my Elves", mine),
        ("their Elves", theirs),
    ] {
        let granted = keywords(&engine, creature);
        assert!(
            granted.contains(KeywordSet::TRAMPLE) && granted.contains(KeywordSet::HASTE),
            "\"all creatures\" reached {who}: {granted:?}"
        );
    }
    assert_eq!(
        (pt(&engine, king), pt(&engine, mine)),
        ((5, 5), (1, 1)),
        "the red ability grants keywords and no body: 0/0 is still 0/0"
    );

    // {1}{G}: the two Forests still standing pay it, and the ability that
    // prints a target asks for one.
    for forest_land in &grove[1..] {
        engine
            .apply(
                p0,
                PlayerAction::ActivateManaAbility {
                    source: *forest_land,
                },
            )
            .unwrap();
    }
    activate(&mut engine, p0, kenrith_the_returned_king(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the counter targets a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs) && options.contains(&king),
        "\"target creature\" names no controller, so every creature is offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, mine, CounterKind::P1P1),
        1,
        "the creature that was named takes the counter"
    );
    assert_eq!(
        counters_on(&engine, theirs, CounterKind::P1P1),
        0,
        "and the creature that was only offered does not"
    );
    assert_eq!(
        counters_on(&engine, king, CounterKind::P1P1),
        0,
        "Kenrith is a legal target for his own green line and still not the one picked"
    );
    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "a 1/1 with one +1/+1 counter on it"
    );
}
