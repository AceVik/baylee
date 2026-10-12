#[allow(clippy::wildcard_imports)]
use super::*;

static FREERUNNING: baylee_cards_dsl::CardDef = baylee_cards_dsl::CardDef {
    faces: &[baylee_cards_dsl::FaceDef {
        freerunning: Some(ManaCost::parse("{R}")),
        ..baylee_cards_dsl::FaceDef::DEFAULT
    }],
    ..baylee_cards_dsl::CardDef::DEFAULT
};

#[test]
fn batch46_hero_and_assassin_mana_pay_cycling_of_a_typed_source_in_hand() {
    for land in [index::AVENGERS_TOWER, index::BROTHERHOOD_HEADQUARTERS] {
        let mut e = prepared(
            &[index::SANDBAR_MERFOLK],
            &[land, index::MASKWOOD_NEXUS],
            &[],
        );
        let land = source(&e, land);
        let card = in_hand(&e, P0, index::SANDBAR_MERFOLK).unwrap();
        pool(&mut e, &[(ManaColor::Colorless, 1)]);
        ability(&mut e, land, 1);
        color(&mut e, ManaColor::Green);
        let before = e.state.zones.list(ZoneLocation::Hand(P0)).len();
        ability(&mut e, card, 0);
        assert_eq!(e.state.players[0].mana_pool.total(), 0);
        assert_eq!(e.state.object(card).unwrap().zone, Zone::Graveyard);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.zones.list(ZoneLocation::Hand(P0)).len(), before);
    }
}

#[test]
fn batch46_brotherhood_mana_pays_an_assassin_activation_after_its_land_leaves() {
    let mut e = prepared(
        &[],
        &[index::BROTHERHOOD_HEADQUARTERS, index::STALKING_ASSASSIN],
        &[index::GRIZZLY_BEARS],
    );
    let headquarters = source(&e, index::BROTHERHOOD_HEADQUARTERS);
    let assassin = source(&e, index::STALKING_ASSASSIN);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    pool(&mut e, &[(ManaColor::Colorless, 3)]);
    ability(&mut e, headquarters, 1);
    color(&mut e, ManaColor::Blue);
    e.state
        .move_object(
            headquarters,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    ability(&mut e, assassin, 0);
    choose(&mut e, vec![bear]);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, bear));
}

#[test]
fn batch46_brotherhood_rejects_other_spells_and_other_sources_even_with_freerunning() {
    let mut e = prepared(
        &[index::RAGING_GOBLIN],
        &[index::BROTHERHOOD_HEADQUARTERS, index::SHIVAN_DRAGON],
        &[],
    );
    let headquarters = source(&e, index::BROTHERHOOD_HEADQUARTERS);
    let dragon = source(&e, index::SHIVAN_DRAGON);
    let goblin = in_hand(&e, P0, index::RAGING_GOBLIN).unwrap();
    pool(&mut e, &[]);
    ability(&mut e, headquarters, 1);
    color(&mut e, ManaColor::Red);
    no_mana_to_cast(&e, goblin);
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: dragon,
                ability_index: 0
            }
        )
        .is_err()
    );

    // No compiled pool card currently has freerunning. Derive its presence
    // from the actual cost-bearing FaceDef rather than directly setting a bit.
    let presence = FREERUNNING.keywords_for_face(0);
    assert!(presence.contains(KeywordSet::FREERUNNING));
    for id in [goblin, dragon] {
        let base = e.state.object_mut(id).unwrap().base_mut();
        base.keywords = base.keywords.union(presence);
    }
    e.refresh_offer();
    assert!(crate::casting::can_cast(e.state(), &RegistryLookup, P0, goblin).is_ok());
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: dragon,
                ability_index: 0
            }
        )
        .is_err()
    );
    e.apply(P0, PlayerAction::CastSpell { card: goblin })
        .unwrap();
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(goblin).unwrap().zone, Zone::Battlefield);
}

#[test]
fn batch46_hero_and_assassin_mana_pay_a_typed_creature_ability_from_its_graveyard() {
    for land in [index::AVENGERS_TOWER, index::BROTHERHOOD_HEADQUARTERS] {
        for nexus in [false, true] {
            let mut board = vec![land];
            if nexus {
                board.push(index::MASKWOOD_NEXUS);
            }
            let mut e = prepared(&[index::FANATIC_OF_RHONAS], &board, &[]);
            let fanatic = hand_to_graveyard(&mut e, P0, index::FANATIC_OF_RHONAS);
            let land = source(&e, land);
            pool(&mut e, &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)]);
            ability(&mut e, land, 1);
            color(&mut e, ManaColor::Green);
            let before = e.snapshot_hash();
            let action = PlayerAction::ActivateAbility {
                source: fanatic,
                ability_index: 2,
            };
            if nexus {
                e.apply(P0, action).unwrap();
                assert_eq!(e.state.players[0].mana_pool.total(), 0);
                assert_eq!(e.state.object(fanatic).unwrap().zone, Zone::Exile);
                let before = e.state.zones.list(ZoneLocation::Battlefield).len();
                pass_until(&mut e, stack_is_empty);
                assert_eq!(
                    e.state.zones.list(ZoneLocation::Battlefield).len(),
                    before + 1
                );
            } else {
                assert!(e.apply(P0, action).is_err());
                assert_eq!(e.snapshot_hash(), before);
                assert_eq!(e.state.object(fanatic).unwrap().zone, Zone::Graveyard);
                assert_eq!(e.state.players[0].mana_pool.total(), 4);
            }
        }
    }
}
