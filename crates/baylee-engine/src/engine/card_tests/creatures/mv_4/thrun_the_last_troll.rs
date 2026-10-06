//! `cards/creatures/mv_4/thrun_the_last_troll.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thrun, the Last Troll: "this spell can't be countered". A Counterspell
/// may point at it and resolves, and the Troll arrives anyway (#243; the
/// Gatherer ruling on Abrupt Decay reads the same clause).
///
/// This test used to pin the opposite: that the Counterspell was never
/// offered, because the engine left an uncounterable spell out of
/// `TargetSpec::Spell`. A Counterspell that resolves into its owner's
/// graveyard is what proves it was cast and did nothing, and the positive
/// needs no control beside it.
#[test]
fn thrun_the_last_troll_arrives_through_a_counterspell_pointed_at_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(382, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[thrun_the_last_troll()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let counter = in_hand(&engine, p1, counterspell()).expect("the counterspell is in hand");
    cast_from_hand(&mut engine, p0, thrun_the_last_troll());
    let troll = on_stack(&engine, thrun_the_last_troll()).expect("the troll is cast");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: counter })
        .expect("the troll is a legal target for the counterspell");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![troll],
            },
        )
        .expect("the counterspell points at the troll");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, counterspell()).is_some(),
        "the counterspell resolved"
    );
    assert!(
        on_battlefield(&engine, p0, thrun_the_last_troll()).is_some(),
        "and the troll arrived"
    );
}

/// Thrun's third line — "{1}{G}: Regenerate this creature" — and the only
/// one of the three that survives something.
///
/// A wrath its own controller casts, which is the cheapest way to put two
/// creatures under one destruction and read the difference: the Elves and
/// the Troll are destroyed by the same resolution, the Elves are in the
/// graveyard and the Troll is not. Supreme Verdict prints no "can't be
/// regenerated" clause, which is the whole reason it is the spell here —
/// the eight cards in this pool that do print it would kill the Troll
/// shield and all (CR 701.19c).
///
/// The shield is bought before the spell is cast rather than in response to
/// it, because Thrun has hexproof and the wrath has no target: there is no
/// window this test needs that the precombat main does not already give it.
#[test]
fn thrun_the_last_troll_regenerates_out_of_a_wrath_the_elves_die_to() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(384, forest())
        .battlefield(
            0,
            &[
                thrun_the_last_troll(),
                llanowar_elves(),
                forest(),
                forest(),
                // Three, not two. `mana_pay::pay_any` settles a generic
                // symbol out of `ManaColor::ALL` in order, so the `{1}` of
                // the regeneration eats a white before it reaches the green
                // sitting right there -- with two Plains the wrath below is
                // then one white short and refused as uncastable.
                plains(),
                plains(),
                plains(),
                island(),
                island(),
            ],
        )
        .hand(0, &[supreme_verdict()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let troll = on_battlefield(&engine, p0, thrun_the_last_troll()).expect("the Troll is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: troll,
                ability_index: 0,
            },
        )
        .expect("{1}{G} is floating and the ability needs no target");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine
            .state()
            .object(troll)
            .expect("nothing has happened to it yet")
            .regeneration_shields,
        1,
        "one shield, bought and standing"
    );
    assert_eq!(
        engine
            .state()
            .object(elf)
            .expect("nor to the Elves")
            .regeneration_shields,
        0,
        "and the Elves have none, which is what makes them the control"
    );

    cast_with_floating(&mut engine, p0, supreme_verdict());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the wrath destroyed an unshielded creature"
    );
    let survivor = engine
        .state()
        .object(troll)
        .expect("and did not destroy the shielded one");
    assert!(
        on_battlefield(&engine, p0, thrun_the_last_troll()).is_some(),
        "the Troll is still on the battlefield"
    );
    assert_eq!(
        survivor.regeneration_shields, 0,
        "the shield was spent on that destruction"
    );
    assert!(
        is_tapped(&engine, troll),
        "and regenerating taps what it saves (CR 701.19a)"
    );
}
