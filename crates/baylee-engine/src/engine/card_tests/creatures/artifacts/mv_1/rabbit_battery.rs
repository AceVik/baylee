//! `cards/creatures/artifacts/mv_1/rabbit_battery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rabbit Battery — {R}, a 1/1 artifact creature — Equipment Rabbit printing
/// haste for itself, "Equipped creature gets +1/+1 and has haste", and
/// Reconfigure {R}, which attaches to another creature or detaches (CR 702.151a).
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
/// earlier. The separate test below exercises the detach ability.
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

#[test]
fn battery_can_unattach_only_while_attached_and_recovers_its_creature_type() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[rabbit_battery(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let battery = on_battlefield(&e, p0, rabbit_battery()).unwrap();
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 2);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.abilities.contains(&(battery, 4)));
    activate(&mut e, p0, rabbit_battery(), 2);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    assert!(!options.contains(&battery));
    aim_at(&mut e, p0, bear);
    pass_until(&mut e, stack_is_empty);
    assert!(!types(&e, battery).contains(TypeSet::CREATURE));
    assert_eq!(pt(&e, bear), (3, 3));
    activate(&mut e, p0, rabbit_battery(), 4);
    assert!(!stack_is_empty(&e));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(battery).unwrap().attached_to, None);
    assert!(types(&e, battery).contains(TypeSet::CREATURE));
    assert_eq!(pt(&e, bear), (2, 2));
    assert!(!keywords(&e, bear).contains(baylee_cards_dsl::KeywordSet::HASTE));
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

/// Whether printed ability `index` of `source` is on the offer right now.
fn battery_offered(e: &Engine<RegistryLookup>, source: ObjectId, index: u32) -> bool {
    matches!(e.pending(), Pending::Priority { legal, .. } if legal.abilities.contains(&(source, index)))
}

/// "Attach to target creature **you control**": the menu holds p0's other
/// creatures and neither the Battery itself nor an opponent's Ogre.
#[test]
fn battery_reconfigure_offers_only_other_creatures_its_controller_has() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[rabbit_battery(), grizzly_bears(), llanowar_elves()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    assert!(
        walk_to_own_main(&mut e, p0),
        "p0 reaches its own main phase"
    );
    let battery = on_battlefield(&e, p0, rabbit_battery()).unwrap();
    let bears = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let elves = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let ogre = on_battlefield(&e, p1, gray_ogre()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 1);
    e.refresh_offer();
    activate(&mut e, p0, rabbit_battery(), 2);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("reconfigure asks for a target: {:?}", e.pending())
    };
    assert!(
        options.contains(&bears) && options.contains(&elves),
        "both of p0's other creatures are legal: {options:?}"
    );
    assert!(
        !options.contains(&ogre),
        "\"target creature you control\": an opponent's creature is not: {options:?}"
    );
    assert!(
        !options.contains(&battery),
        "and it is another creature, not the Battery: {options:?}"
    );
}

/// "Reconfigure only as a sorcery": with the Battery loose (attach) or worn
/// (unattach), `{R}` floating and the same board, the ability is on the offer
/// in p0's own main phase with an empty stack, and off it while a creature
/// spell sits on the stack and on the opponent's turn.
#[test]
#[allow(clippy::too_many_lines)] // one board, three moments, two abilities
fn battery_reconfigure_and_unattach_are_sorcery_speed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for attached in [false, true] {
        let index = if attached { 4 } else { 2 };
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[rabbit_battery(), grizzly_bears()])
            .hand(0, &[llanowar_elves()])
            .start();
        keep_mulligans(&mut e);
        assert!(
            walk_to_own_main(&mut e, p0),
            "p0 reaches its own main phase"
        );
        let battery = on_battlefield(&e, p0, rabbit_battery()).unwrap();
        let bears = on_battlefield(&e, p0, grizzly_bears()).unwrap();
        let float = |e: &mut Engine<RegistryLookup>| {
            let pool = &mut e.dev_state_mut(p0).unwrap().players[0].mana_pool;
            pool.add(ManaColor::Red, 3);
            pool.add(ManaColor::Green, 1);
            e.refresh_offer();
        };
        float(&mut e);
        if attached {
            activate(&mut e, p0, rabbit_battery(), 2);
            aim_at(&mut e, p0, bears);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(
                e.state().object(battery).unwrap().attached_to,
                Some(bears),
                "the Battery is worn before the timing is probed"
            );
            e.refresh_offer();
        }
        assert!(
            battery_offered(&e, battery, index),
            "control: in p0's main phase with an empty stack and {{R}} floating, ability {index} \
             (attached = {attached}) is on the offer"
        );

        // A creature spell on the stack: the same mana, the same board.
        cast_with_floating(&mut e, p0, llanowar_elves());
        assert!(!stack_is_empty(&e), "the Elves spell is waiting to resolve");
        assert!(
            e.state().players[0].mana_pool.available(ManaColor::Red) >= 1,
            "red is still floating, so mana is not what withholds the ability"
        );
        assert!(
            !battery_offered(&e, battery, index),
            "\"only as a sorcery\": not while a spell is on the stack (attached = {attached})"
        );
        let hash = e.state().snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: battery,
                    ability_index: index
                }
            )
            .is_err()
        );
        assert_eq!(
            e.state().snapshot_hash(),
            hash,
            "the refusal changed nothing"
        );
        pass_until(&mut e, stack_is_empty);

        // The opponent's turn, with p0 holding priority and red floating.
        pass_until(&mut e, |e| {
            e.state().turn.active == p1
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
        });
        float(&mut e);
        assert!(
            e.state().players[0].mana_pool.available(ManaColor::Red) >= 1,
            "red is floating on the opponent's turn"
        );
        assert!(
            !battery_offered(&e, battery, index),
            "\"only as a sorcery\": not on the opponent's turn (attached = {attached})"
        );
        let hash = e.state().snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: battery,
                    ability_index: index
                }
            )
            .is_err()
        );
        assert_eq!(
            e.state().snapshot_hash(),
            hash,
            "the refusal changed nothing"
        );
    }
}
