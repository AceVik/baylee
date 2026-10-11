//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn gabriel_offers_all_four_abilities_until_the_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (mode, keyword) in [
        (0, KeywordSet::FLYING),
        (1, KeywordSet::FIRST_STRIKE),
        (2, KeywordSet::TRAMPLE),
        (3, KeywordSet::EMPTY),
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::GABRIEL_ANGELFIRE])
            .battlefield(1, &[giant_spider(), giant_spider()])
            .start();
        keep_mulligans(&mut e);
        assert!(
            !stack_is_empty(&e),
            "the trigger is on the stack before choosing a keyword"
        );
        assert!(
            matches!(e.pending(), Pending::Priority { .. }),
            "players can respond before the choice"
        );
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        let Pending::ChooseCastMode { options, .. } = e.pending() else {
            unreachable!()
        };
        assert_eq!(options.len(), 4);
        let hash = e.state().snapshot_hash();
        assert!(e.apply(p0, PlayerAction::ChooseMode(4)).is_err());
        assert_eq!(e.state().snapshot_hash(), hash);
        e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
        pass_until(&mut e, stack_is_empty);
        let gabriel = on_battlefield(&e, p0, index::GABRIEL_ANGELFIRE).unwrap();
        if mode < 3 {
            assert!(keywords(&e, gabriel).contains(keyword));
        }
        assert_eq!(
            crate::eval::matches(
                &baylee_cards_dsl::Filter::HasRampage,
                e.state(),
                e.state().object(gabriel).unwrap(),
                p0,
                gabriel
            ),
            mode == 3
        );
        if mode == 3 {
            let blockers: Vec<_> = e
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .copied()
                .filter(|id| e.state().object(*id).unwrap().controller == p1)
                .collect();
            attack_and_collect_blocks(&mut e, gabriel, p1);
            e.apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: blockers.iter().map(|id| (*id, gabriel)).collect(),
                },
            )
            .unwrap();
            assert_eq!(e.state().zones.list(ZoneLocation::Stack).len(), 1);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(pt(&e, gabriel), (7, 7));
        }
        for _ in 0..100 {
            if e.state().turn.active == p1 && e.state().turn.phase == crate::turn::Phase::FirstMain
            {
                break;
            }
            let (player, action) = crate::engine::testkit::answer_one(&e).unwrap();
            e.apply(player, action).unwrap();
        }
        assert_eq!(e.state().turn.active, p1);
        assert_eq!(e.state().turn.phase, crate::turn::Phase::FirstMain);
        if mode < 3 {
            assert!(keywords(&e, gabriel).contains(keyword));
        }
        pass_until(&mut e, |e| {
            e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Upkeep
        });
        assert!(!crate::eval::matches(
            &baylee_cards_dsl::Filter::HasRampage,
            e.state(),
            e.state().object(gabriel).unwrap(),
            p0,
            gabriel
        ));
        if mode < 3 {
            assert!(!keywords(&e, gabriel).contains(keyword));
        }
    }
}
