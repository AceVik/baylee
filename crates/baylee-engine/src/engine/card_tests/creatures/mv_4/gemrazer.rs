//! `cards/creatures/mv_4/gemrazer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gemrazer ({3}{G}) cast normally arrives as a 4/4 Beast with reach and trample.
#[test]
fn gemrazer_arrives_as_a_four_four_beast_with_reach_and_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[gemrazer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        in_hand(&engine, p0, gemrazer()).is_some(),
        "the card is in hand before anything is cast"
    );
    cast_from_hand(&mut engine, p0, gemrazer());
    pass_until(&mut engine, stack_is_empty);

    let beast = on_battlefield(&engine, p0, gemrazer()).expect("Gemrazer resolved");
    assert!(
        in_hand(&engine, p0, gemrazer()).is_none(),
        "and it left the hand to get there"
    );
    assert!(
        types(&engine, beast).contains(TypeSet::CREATURE),
        "it is a creature permanent: {:?}",
        types(&engine, beast)
    );
    assert_eq!(pt(&engine, beast), (4, 4), "the body the card prints");
    let kw = keywords(&engine, beast);
    assert!(kw.contains(KeywordSet::REACH), "Gemrazer has reach");
    assert!(kw.contains(KeywordSet::TRAMPLE), "Gemrazer has trample");
}

#[test]
fn gemrazer_mutates_onto_non_human_creature_and_destroys_opponent_artifact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(84, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[sol_ring()])
        .hand(0, &[gemrazer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    let ring = on_battlefield(&engine, p1, sol_ring()).expect("Sol Ring on opponent battlefield");
    let card = in_hand(&engine, p0, gemrazer()).expect("Gemrazer in hand");

    // Tap the 3 Forests (leaving Llanowar Elves untapped) so {G}{G}{G} is in the pool:
    // only Mutate ({1}{G}{G}) is affordable, not the normal {3}{G} cost.
    tap_mana_except(&mut engine, p0, elf);
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("begin casting Gemrazer via mutate");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected mutate target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&elf),
        "non-Human creature owned by p0 is a legal mutate target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("choose Llanowar Elves as mutate target");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseMode(0))
        .expect("put Gemrazer on top");

    // Resolve the mutating spell, which triggers "Whenever this creature mutates,
    // destroy target artifact or enchantment an opponent controls."
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected mutate trigger target choice, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&ring),
        "opponent's Sol Ring is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .expect("target opponent's Sol Ring");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, sol_ring()).is_none(),
        "opponent's Sol Ring was destroyed by the mutate trigger"
    );
    assert_eq!(
        pt(&engine, elf),
        (4, 4),
        "merged creature has Gemrazer's 4/4 characteristics"
    );
    let kw = keywords(&engine, elf);
    assert!(kw.contains(KeywordSet::REACH));
    assert!(kw.contains(KeywordSet::TRAMPLE));
}

#[test]
fn gemrazer_underneath_preserves_body_and_both_cards_leave_together() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(85, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .hand(0, &[gemrazer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let card = in_hand(&engine, p0, gemrazer()).unwrap();
    let version = engine.state().object(elf).unwrap().version;
    tap_mana_except(&mut engine, p0, elf);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elf), (1, 1));
    assert_eq!(
        engine.state().object(elf).unwrap().version,
        version,
        "no zone change or new permanent"
    );
    assert!(keywords(&engine, elf).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        engine.state().merged_components(elf).as_slice(),
        &[elf, card]
    );
    assert_eq!(
        engine
            .state()
            .merged_top(engine.state().object(elf).unwrap())
            .id,
        elf
    );
    let _ = engine.state.move_object(
        elf,
        ZoneLocation::Hand(p0),
        crate::zone::ZonePosition::Top,
        crate::event::Cause::Effect,
    );
    for id in [elf, card] {
        let obj = engine.state().object(id).unwrap();
        assert_eq!(obj.zone, Zone::Hand);
        assert!(!obj.riders.iter().any(|r| matches!(
            r,
            crate::object::Rider::MergedInto(_) | crate::object::Rider::MergedComponent(_)
        )));
    }
    assert!(
        !engine
            .state()
            .object(elf)
            .unwrap()
            .base
            .keywords
            .contains(KeywordSet::TRAMPLE)
    );
    assert_eq!(engine.state().object(card).unwrap().base.power, Some(4));
}

#[test]
fn mutate_target_ownership_follows_spell_owner_even_when_control_is_borrowed() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(86, forest())
        .hand(1, &[gemrazer()])
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let spell = in_hand(&engine, p1, gemrazer()).unwrap();
    engine.state.object_mut(spell).unwrap().set_controller(p0);
    for (owner, expected) in [(p0, false), (p1, true)] {
        let elf = on_battlefield(&engine, owner, llanowar_elves()).unwrap();
        assert_eq!(
            crate::eval::matches(
                &baylee_cards_dsl::Filter::OwnedBySourceOwner,
                &engine.state,
                engine.state.object(elf).unwrap(),
                p0,
                spell
            ),
            expected
        );
    }
}
