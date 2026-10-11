//! `cards/creatures/mv_6/ojer_taq_deepest_foundation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn ojer_taq_deepest_foundation_attacks_with_vigilance_and_remains_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(311, plains())
        .battlefield(0, &[ojer_taq_deepest_foundation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let god = on_battlefield(&engine, p0, ojer_taq_deepest_foundation())
        .expect("Ojer Taq on battlefield");
    assert_eq!(pt(&engine, god), (6, 6), "Ojer Taq is a 6/6 God");
    assert!(
        keywords(&engine, god).contains(KeywordSet::VIGILANCE),
        "Ojer Taq has vigilance"
    );
    assert!(!is_tapped(&engine, god), "starts untapped");

    // Advance to combat and declare Ojer Taq as an attacker against p1.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(god, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Not `stack_is_empty`: the stack is already empty the moment attackers
    // are declared, so that predicate stops the walk *before* the combat
    // damage step and every life total still reads 20. The end step is past
    // damage (CR 510.2) and is what the assertion below needs.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        14,
        "opponent took 6 combat damage"
    );
    assert!(
        !is_tapped(&engine, god),
        "vigilance keeps Ojer Taq untapped after attacking"
    );
}

#[test]
fn ojer_taq_triples_own_creature_tokens_and_multiplies_with_doubling_season() {
    use crate::engine::card_tests::token_helpers::named;
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for own in [false, true] {
        for doubled in [false, true] {
            let mut board = vec![index::MIRREX];
            if own {
                board.push(ojer_taq_deepest_foundation());
            }
            if doubled {
                board.push(index::DOUBLING_SEASON);
            }
            let opponent = if own {
                vec![]
            } else {
                vec![ojer_taq_deepest_foundation()]
            };
            let mut e = Duel::new(SEED, forest())
                .battlefield(0, &board)
                .battlefield(1, &opponent)
                .start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, p0);
            e.dev_state_mut(p0).unwrap().players[0]
                .mana_pool
                .add(ManaColor::Colorless, 3);
            e.refresh_offer();
            activate(&mut e, p0, index::MIRREX, 2);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(
                named(&e, "Phyrexian Mite").len(),
                (if own { 3 } else { 1 }) * (if doubled { 2 } else { 1 })
            );
        }
    }
}
#[test]
fn ojer_taq_returns_tapped_as_land_and_can_transform_after_three_attackers() {
    let p0 = PlayerId::new(0);
    for attackers in [2, 3] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    ojer_taq_deepest_foundation(),
                    grizzly_bears(),
                    grizzly_bears(),
                    grizzly_bears(),
                ],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let god = on_battlefield(&e, p0, ojer_taq_deepest_foundation()).unwrap();
        kill(&mut e, god);
        assert_eq!(e.state().object(god).unwrap().face_index, 1);
        assert!(is_tapped(&e, god));
        assert!(types(&e, god).contains(TypeSet::LAND));
        assert!(!e.state().journal.entries().iter().any(|entry|matches!(entry.event,crate::event::GameEvent::Transformed{object,..}if object==god)));
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let chosen = e
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| types(&e, *id).contains(TypeSet::CREATURE))
            .take(attackers)
            .map(|id| (id, Defender::Player(PlayerId::new(1))))
            .collect();
        e.apply(p0, PlayerAction::DeclareAttackers { attackers: chosen })
            .unwrap();
        pass_until(&mut e, |e| {
            e.state().turn.phase == Phase::SecondMain
                && matches!(e.pending(),Pending::Priority{player,..}if *player==p0)
        });
        let state = e.dev_state_mut(p0).unwrap();
        state.set_tapped(god, false);
        state.players[0].mana_pool.add(ManaColor::White, 3);
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert_eq!(legal.abilities.contains(&(god, 1)), attackers == 3);
        if attackers == 3 {
            activate(&mut e, p0, ojer_taq_deepest_foundation(), 1);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().object(god).unwrap().face_index, 0);
            assert_eq!(pt(&e, god), (6, 6));
        }
    }
}

#[test]
fn copied_god_returns_a_double_faced_card_but_not_an_ordinary_card() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for (card, returns) in [
        (index::BLACKBLOOM_ROGUE, true),
        (index::GRIZZLY_BEARS, false),
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[card])
            .battlefield(1, &[ojer_taq_deepest_foundation()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, card).unwrap();
        let target = on_battlefield(&e, PlayerId::new(1), ojer_taq_deepest_foundation()).unwrap();
        assert!(crate::copiable_abilities::apply_copy(
            e.dev_state_mut(p0).unwrap(),
            crate::copiable_abilities::CopyApplication {
                source,
                target,
                mods: &[],
                copy_index: 0,
                until_eot: false,
                resolving: None,
                text: crate::text_changes::TextChangeMap::IDENTITY,
            }
        ));
        kill(&mut e, source);
        let object = e.state().object(source).unwrap();
        assert_eq!(
            object.zone,
            if returns {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
        if returns {
            assert_eq!(object.face_index, 1);
            assert!(is_tapped(&e, source));
            assert!(types(&e, source).contains(TypeSet::LAND));
            e.dev_state_mut(p0)
                .unwrap()
                .move_object(
                    source,
                    ZoneLocation::Hand(p0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            assert_eq!(e.state().object(source).unwrap().face_index, 0);
            assert!(
                e.state()
                    .object(source)
                    .unwrap()
                    .characteristics()
                    .types
                    .contains(TypeSet::CREATURE)
            );
        }
    }
}
