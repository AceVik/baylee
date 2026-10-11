//! Token-making lands, paid and resolved through the printed abilities.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn float(e: &mut Engine<RegistryLookup>, color: ManaColor, amount: u32) {
    e.dev_state_mut(PlayerId::new(0)).unwrap().players[0]
        .mana_pool
        .add(color, amount);
    e.refresh_offer();
}

#[test]
fn tapped_mana_tokens_are_doubled_and_preserve_their_own_characteristics() {
    let p = PlayerId::new(0);
    for (land, cost, definition, indestructible) in [
        (
            index::HALL_OF_TAGSIN,
            4,
            &baylee_cards::tokens::POWERSTONE,
            false,
        ),
        (
            index::THE_GREAT_MOUND,
            3,
            &baylee_cards::tokens::VIBRANIUM,
            true,
        ),
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[land, index::DOUBLING_SEASON])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p);
        float(&mut e, ManaColor::Colorless, cost);
        activate(&mut e, p, land, 2);
        assert!(tokens_of(&e, p).is_empty(), "creation uses the stack");
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
        pass_until(&mut e, stack_is_empty);
        let tokens = tokens_of(&e, p);
        assert_eq!(tokens.len(), 2);
        for id in tokens {
            let object = e.state().object(id).unwrap();
            assert!(object.status.contains(crate::object::Status::TAPPED));
            assert!(std::ptr::eq(object.token.unwrap(), definition));
            assert_eq!(
                object
                    .characteristics()
                    .keywords
                    .contains(KeywordSet::INDESTRUCTIBLE),
                indestructible
            );
            assert!(
                e.apply(
                    p,
                    PlayerAction::ActivateAbility {
                        source: id,
                        ability_index: 0
                    }
                )
                .is_err()
            );
            e.dev_state_mut(p).unwrap().set_tapped(id, false);
            e.refresh_offer();
            e.apply(
                p,
                PlayerAction::ActivateAbility {
                    source: id,
                    ability_index: 0,
                },
            )
            .unwrap();
            assert!(stack_is_empty(&e));
        }
        assert_eq!(e.state().players[0].mana_pool.total(), 2);
        assert_eq!(
            e.state().players[0]
                .mana_pool
                .available(ManaColor::Colorless),
            0
        );
    }
}

#[test]
fn junktown_sacrifices_as_a_cost_and_its_six_junk_tokens_allow_paid_plays() {
    let p = PlayerId::new(0);
    for top in [forest(), index::MIND_STONE] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::JUNKTOWN, index::DOUBLING_SEASON])
            .hand(0, &[top])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p);
        let exiled = hand_to_library_top(&mut e, p, top);
        let town = on_battlefield(&e, p, index::JUNKTOWN).unwrap();
        float(&mut e, ManaColor::Red, 5);
        activate(&mut e, p, index::JUNKTOWN, 1);
        assert_eq!(e.state().object(town).unwrap().zone, Zone::Graveyard);
        assert!(tokens_of(&e, p).is_empty());
        pass_until(&mut e, stack_is_empty);
        let tokens = tokens_of(&e, p);
        assert_eq!(tokens.len(), 6);
        e.apply(
            p,
            PlayerAction::ActivateAbility {
                source: tokens[0],
                ability_index: 0,
            },
        )
        .unwrap();
        assert!(e.state().object(tokens[0]).is_none());
        assert!(
            e.apply(
                p,
                PlayerAction::ActivateAbility {
                    source: tokens[1],
                    ability_index: 0
                }
            )
            .is_err(),
            "only as a sorcery"
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(exiled).unwrap().zone, Zone::Exile);
        if top == forest() {
            e.apply(p, PlayerAction::PlayLand { card: exiled }).unwrap();
        } else {
            assert!(
                e.apply(p, PlayerAction::CastSpell { card: exiled })
                    .is_err(),
                "no free casting"
            );
            float(&mut e, ManaColor::Colorless, 2);
            e.apply(p, PlayerAction::CastSpell { card: exiled })
                .unwrap();
            pass_until(&mut e, stack_is_empty);
        }
        assert_eq!(e.state().object(exiled).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn junk_permissions_expire_at_cleanup_without_moving_the_card() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::JUNKTOWN])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p);
    let card = hand_to_library_top(&mut e, p, forest());
    float(&mut e, ManaColor::Red, 5);
    activate(&mut e, p, index::JUNKTOWN, 1);
    pass_until(&mut e, stack_is_empty);
    let source = tokens_of(&e, p)[0];
    e.apply(
        p,
        PlayerAction::ActivateAbility {
            source,
            ability_index: 0,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    reach_their_main_phase(&mut e, q);
    reach_their_main_phase(&mut e, p);
    assert_eq!(e.state().object(card).unwrap().zone, Zone::Exile);
    assert!(e.apply(p, PlayerAction::PlayLand { card }).is_err());
}

#[test]
fn sokenzan_discounts_only_owned_legends_and_grants_temporary_haste_to_every_created_token() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    let legends = [
        index::HAPATRA_VIZIER_OF_POISONS,
        index::ARCHANGEL_AVACYN,
        index::BIRGI_GOD_OF_STORYTELLING,
    ];
    for count in [0, 1, 3] {
        let mut board = vec![index::DOUBLING_SEASON, index::GRIZZLY_BEARS];
        board.extend_from_slice(&legends[..count]);
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(1, &[index::ARCHANGEL_AVACYN])
            .hand(0, &[index::SOKENZAN_CRUCIBLE_OF_DEFIANCE])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p);
        let source = in_hand(&e, p, index::SOKENZAN_CRUCIBLE_OF_DEFIANCE).unwrap();
        float(&mut e, ManaColor::Red, u32::try_from(3 - count).unwrap());
        assert!(
            e.apply(
                p,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 1
                }
            )
            .is_err(),
            "one red pip remains"
        );
        float(&mut e, ManaColor::Red, 1);
        e.apply(
            p,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 1,
            },
        )
        .unwrap();
        assert_eq!(e.state().object(source).unwrap().zone, Zone::Graveyard);
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
        pass_until(&mut e, stack_is_empty);
        let tokens = tokens_of(&e, p);
        assert_eq!(tokens.len(), 4);
        for id in &tokens {
            let object = e.state().object(*id).unwrap();
            assert!(!object.status.contains(crate::object::Status::TAPPED));
            assert_eq!(pt(&e, *id), (1, 1));
            assert!(
                object
                    .characteristics()
                    .keywords
                    .contains(KeywordSet::HASTE)
            );
            assert!(
                !object.base.keywords.contains(KeywordSet::HASTE),
                "haste is not copiable"
            );
        }
        assert!(
            !e.state()
                .object(on_battlefield(&e, p, index::GRIZZLY_BEARS).unwrap())
                .unwrap()
                .characteristics()
                .keywords
                .contains(KeywordSet::HASTE)
        );
        reach_their_main_phase(&mut e, q);
        for id in tokens {
            assert!(
                !e.state()
                    .object(id)
                    .unwrap()
                    .characteristics()
                    .keywords
                    .contains(KeywordSet::HASTE)
            );
        }
    }
}
