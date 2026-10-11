//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn winter_blast_taps_all_chosen_creatures_and_damages_only_the_flying_ones_among_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for lose_flying in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .hand(0, &[index::WINTER_BLAST])
            .battlefield(
                1,
                &[
                    index::SHIVAN_DRAGON,
                    index::SHIVAN_DRAGON,
                    index::GRIZZLY_BEARS,
                ],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let dragons = all_on_battlefield(&e, p1, index::SHIVAN_DRAGON);
        let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
        let s = e.dev_state_mut(p0).unwrap();
        s.players[0].mana_pool.add(ManaColor::Green, 1);
        s.players[0].mana_pool.add(ManaColor::Colorless, 2);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, index::WINTER_BLAST);
        e.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![dragons[0], bear],
                players: vec![],
            },
        )
        .unwrap();
        if lose_flying {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(dragons[0])
                .unwrap()
                .base_mut()
                .keywords = baylee_cards_dsl::KeywordSet::EMPTY;
        }
        pass_until(&mut e, stack_is_empty);
        assert!(is_tapped(&e, dragons[0]) && is_tapped(&e, bear));
        assert!(!is_tapped(&e, dragons[1]));
        assert_eq!(
            e.state().object(dragons[0]).unwrap().damage,
            if lose_flying { 0 } else { 2 }
        );
        assert_eq!(e.state().object(dragons[1]).unwrap().damage, 0);
        assert_eq!(e.state().object(bear).unwrap().damage, 0);
    }
}
