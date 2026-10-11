//! Each Road creates the same printed Pilot, at sorcery speed and with real costs.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn every_road_sacrifices_for_two_pilots_under_doubling_season() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    for (land, color) in [
        (index::COUNTRY_ROADS, ManaColor::White),
        (index::REEF_ROADS, ManaColor::Blue),
        (index::FOUL_ROADS, ManaColor::Black),
        (index::ROCKY_ROADS, ManaColor::Red),
        (index::WILD_ROADS, ManaColor::Green),
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[land, index::DOUBLING_SEASON])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p);
        let source = on_battlefield(&e, p, land).unwrap();
        e.dev_state_mut(p).unwrap().players[0]
            .mana_pool
            .add(color, 2);
        e.refresh_offer();
        e.apply(p, PlayerAction::PassPriority).unwrap();
        e.apply(q, PlayerAction::PassPriority).unwrap();
        // This advances out of main; the activation is no longer a sorcery action.
        assert!(
            e.apply(
                p,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 1
                }
            )
            .is_err()
        );
        reach_their_main_phase(&mut e, q);
        reach_their_main_phase(&mut e, p);
        e.dev_state_mut(p).unwrap().players[0]
            .mana_pool
            .add(color, 2);
        e.refresh_offer();
        e.apply(
            p,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 1,
            },
        )
        .unwrap();
        assert_eq!(e.state().object(source).unwrap().zone, Zone::Graveyard);
        assert!(tokens_of(&e, p).is_empty());
        pass_until(&mut e, stack_is_empty);
        let made = tokens_of(&e, p);
        assert_eq!(made.len(), 2);
        for id in made {
            assert_eq!(pt(&e, id), (1, 1));
            assert!(std::ptr::eq(
                e.state().object(id).unwrap().token.unwrap(),
                &raw const baylee_cards::tokens::PILOT_1_1_COLORLESS
            ));
        }
    }
}
