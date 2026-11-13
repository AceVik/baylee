//! `cards/creatures/mv_3/glissa_the_traitor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

#[test]
fn glissa_returns_only_an_own_artifact_when_an_opposing_creature_dies() {
    for accept in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::GLISSA_THE_TRAITOR, swamp(), swamp(), swamp()])
            .hand(0, &[hero_s_downfall(), sol_ring(), grizzly_bears()])
            .battlefield(1, &[index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        let artifact = hand_to_graveyard(&mut e, P0, sol_ring());
        let nonartifact = hand_to_graveyard(&mut e, P0, grizzly_bears());
        let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();

        cast_from_hand(&mut e, P0, hero_s_downfall());
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { player: P0, .. })
        });
        let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
            unreachable!()
        };
        assert!(options.contains(&artifact));
        assert!(!options.contains(&nonartifact), "only an artifact card");
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![artifact],
                players: vec![],
            },
        )
        .unwrap();
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::YesNo { player: P0, .. })
        });
        e.apply(P0, PlayerAction::YesNo(accept)).unwrap();
        pass_until(&mut e, stack_is_empty);

        assert_eq!(e.state().object(bears).unwrap().zone, Zone::Graveyard);
        assert_eq!(e.state().object(nonartifact).unwrap().zone, Zone::Graveyard);
        assert_eq!(
            e.state().object(artifact).unwrap().zone,
            if accept { Zone::Hand } else { Zone::Graveyard }
        );
    }
}
