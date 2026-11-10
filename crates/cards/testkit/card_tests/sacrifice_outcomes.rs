//! Counted and conditional sacrifices through the real card's stack abilities.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

#[test]
fn mold_demon_pays_two_distinct_swamps_together_and_can_decline() {
    for accept in [false, true] {
        let mut e = prepared(&[index::MOLD_DEMON], &[index::SWAMP; 2], &[index::SWAMP]);
        let demon = in_hand(&e, P0, index::MOLD_DEMON).unwrap();
        cast_with_floating(&mut e, P0, index::MOLD_DEMON);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        assert_eq!(
            e.state().object(demon).unwrap().zone,
            Zone::Battlefield,
            "Mold Demon has an ETB trigger, not an entry replacement"
        );
        e.apply(P0, PlayerAction::YesNo(accept)).unwrap();
        if accept {
            let Pending::ChooseCards {
                options,
                min: 2,
                max: 2,
                ..
            } = e.pending()
            else {
                panic!("two Swamps required: {:?}", e.pending());
            };
            let options = options.clone();
            assert_eq!(options.len(), 2, "opponent's Swamp cannot pay");
            let hash = e.state().snapshot_hash();
            for objects in [vec![options[0]], vec![options[0]; 2]] {
                assert!(
                    e.apply(P0, PlayerAction::ChooseObjects { objects })
                        .is_err()
                );
                assert_eq!(e.state().snapshot_hash(), hash);
            }
            e.apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: options.clone(),
                },
            )
            .unwrap();
            for id in options {
                assert_eq!(e.state().object(id).unwrap().zone, Zone::Graveyard);
            }
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(demon).unwrap().zone,
            if accept {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
        assert!(on_battlefield(&e, P1, index::SWAMP).is_some());
    }
}

#[test]
fn mold_demon_cannot_make_a_partial_payment_and_swamps_may_be_tapped() {
    for available in 0..=2 {
        let mut e = prepared(&[index::MOLD_DEMON], &vec![index::SWAMP; available], &[]);
        let demon = in_hand(&e, P0, index::MOLD_DEMON).unwrap();
        let swamps: Vec<_> = e.state().battlefield_seen().collect();
        for id in &swamps {
            e.dev_state_mut(P0).unwrap().set_tapped(*id, true);
        }
        e.refresh_offer();
        cast_with_floating(&mut e, P0, index::MOLD_DEMON);
        if available == 2 {
            pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
            e.apply(P0, PlayerAction::YesNo(true)).unwrap();
            e.apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: swamps.clone(),
                },
            )
            .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(demon).unwrap().zone,
            if available == 2 {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
        if available < 2 {
            for id in swamps {
                assert_eq!(e.state().object(id).unwrap().zone, Zone::Battlefield);
            }
        }
    }
}

#[test]
fn serendib_djinn_checks_the_sacrificed_land_and_sacrifices_itself_when_none_remain() {
    for (land, damage, exile) in [
        (index::ISLAND, 3, false),
        (index::FOREST, 0, false),
        (index::ISLAND, 3, true),
    ] {
        let theirs = if exile {
            vec![index::DAUTHI_VOIDWALKER]
        } else {
            vec![]
        };
        let mut e = prepared(&[index::SERENDIB_DJINN], &[land], &theirs);
        let djinn = in_hand(&e, P0, index::SERENDIB_DJINN).unwrap();
        let victim = on_battlefield(&e, P0, land).unwrap();
        cast_with_floating(&mut e, P0, index::SERENDIB_DJINN);
        pass_until(&mut e, stack_is_empty);
        assert!(keywords(&e, djinn).contains(KeywordSet::FLYING));
        let before = e.state().players[0].life;
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, before - damage);
        assert_eq!(
            e.state().object(victim).unwrap().zone,
            if exile { Zone::Exile } else { Zone::Graveyard }
        );
        assert_eq!(
            e.state().object(djinn).unwrap().zone,
            if exile { Zone::Exile } else { Zone::Graveyard }
        );
    }
}

#[test]
fn serendib_djinn_reads_departing_characteristics_before_a_land_copy_reverts() {
    let mut e = prepared(
        &[index::SERENDIB_DJINN],
        &[index::FOREST, index::ISLAND],
        &[],
    );
    cast_with_floating(&mut e, P0, index::SERENDIB_DJINN);
    pass_until(&mut e, stack_is_empty);
    let victim = on_battlefield(&e, P0, index::FOREST).unwrap();
    let island = on_battlefield(&e, P0, index::ISLAND).unwrap();
    let island_base = e.state().object(island).unwrap().base.clone();
    let object = e.dev_state_mut(P0).unwrap().object_mut(victim).unwrap();
    object.original_base = Some(object.base.clone());
    object.base = island_base;
    e.dev_state_mut(P0).unwrap().invalidate_projections();
    e.refresh_offer();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let before = e.state().players[0].life;
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![victim],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, before - 3);
    assert!(on_battlefield(&e, P0, index::SERENDIB_DJINN).is_some());
    assert!(
        !e.state()
            .object(victim)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::land::ISLAND)
    );
}

mod retained_arm;
