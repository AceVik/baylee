//! `cards/creatures/artifacts/mv_1/rabbit_battery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rabbit Battery — {R}, a 1/1 artifact creature — Equipment Rabbit printing
/// haste for itself, "Equipped creature gets +1/+1 and has haste", and
/// Reconfigure {R}, whose attach half is the equip ability CR 702.151 words
/// exactly as CR 702.6 does.
///
/// The Elves is the control the whole claim rests on: a printed 1/1 with no
/// haste of its own before the reconfigure, a 2/2 with haste after it — so the
/// bonus and the keyword are read as changes to a *bystander* and not as
/// anything the Battery says about itself. The Battery standing at its printed
/// 1/1 while it holds the Elves is the other half of
/// `Filter::AttachedToBySource`: the grant reaches the creature it is attached
/// to and never its own source. Reconfiguring is pressed rather than read,
/// because "target creature you control" is a question the engine asks, and the
/// {R} that pays for it is the red the three Mountains left floating one spell
/// earlier; the unattach mode is the `Coverage::Partial` gap and is
/// deliberately left alone.
///
/// "While attached, this isn't a creature" (CR 702.151b) is read last, once
/// the state-based actions have had their look: a creature attached to
/// anything becomes unattached (CR 704.5p), so a Battery that stayed a
/// creature would fall straight off the Elves. It was a creature there until
/// the engine read that sentence, and stayed on only because the
/// state-based action read the second sentence of CR 704.5p and not the first.
#[test]
#[allow(clippy::too_many_lines)] // one game, from the cast to the settled attachment
fn rabbit_battery_reconfigures_onto_the_elves_and_hands_it_a_bonus_and_haste() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4211, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[rabbit_battery()])
        .start();
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, elves), (1, 1), "a printed 1/1");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::HASTE),
        "and nothing has handed it haste yet"
    );

    cast_from_hand(&mut engine, p0, rabbit_battery());
    pass_until(&mut engine, stack_is_empty);
    let battery = on_battlefield(&engine, p0, rabbit_battery()).expect("the Battery resolved");
    assert_eq!(pt(&engine, battery), (1, 1), "a printed 1/1 of its own");
    assert!(
        keywords(&engine, battery).contains(KeywordSet::HASTE),
        "and the haste the card prints for itself"
    );
    assert!(
        engine
            .state()
            .object(battery)
            .is_some_and(|o| o.attached_to.is_none()),
        "it arrives holding nobody"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "{{R}} of the three Mountains paid for the Battery and the rest is \
         still floating, so Reconfigure's {{R}} is payable"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == battery)
        .expect("the floating red pays for Reconfigure, so its one line is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("reconfigure activates");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "reconfigure asks which creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves),
        "\"attach to target creature you control\": the Elves is one of them: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves was among the options the question enumerated");
    pass_until(&mut engine, |e| {
        e.state()
            .object(battery)
            .is_some_and(|o| o.attached_to == Some(elves))
    });

    assert_eq!(
        pt(&engine, elves),
        (2, 2),
        "\"equipped creature gets +1/+1\": the 1/1 it was printed as, plus one"
    );
    assert!(
        keywords(&engine, elves).contains(KeywordSet::HASTE),
        "\"and has haste\" — the Elves prints none of its own"
    );
    assert_eq!(
        pt(&engine, battery),
        (1, 1),
        "and the grant belongs to the creature it is attached to, not to its own source"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the board has settled, state-based actions and all: {:?}",
        engine.pending()
    );
    let worn = engine.state().object(battery).expect("still on the table");
    assert!(
        !worn.characteristics().types.contains(TypeSet::CREATURE),
        "\"While attached, this isn't a creature\" (CR 702.151b)"
    );
    assert!(
        worn.characteristics().types.contains(TypeSet::ARTIFACT),
        "and an artifact still — only the creature type goes"
    );
    assert_eq!(
        worn.attached_to,
        Some(elves),
        "so the state-based action that takes a creature off what it is \
         attached to (CR 704.5p) leaves it on the Elves"
    );
}
