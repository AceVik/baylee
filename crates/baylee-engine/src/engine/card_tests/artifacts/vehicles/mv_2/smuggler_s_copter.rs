//! `cards/artifacts/vehicles/mv_2/smuggler_s_copter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crew lasts one turn. Both attacking and blocking trigger optional draw/discard.
#[test]
fn smugglers_copter_crews_and_loots_on_attack_or_block() {
    use crate::engine::card_tests::legends_kit as kit;
    for blocking in [false, true] {
        let mut e = kit::game(
            980,
            &[smugglers_copter(), llanowar_elves()],
            &[serra_angel()],
            &[forest()],
            &[],
        );
        let copter = kit::obj(&e, kit::P0, smugglers_copter());
        let elf = kit::obj(&e, kit::P0, llanowar_elves());
        if blocking {
            pass_until(&mut e, |e| {
                e.state().turn.active == kit::P1
                    && matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P0)
            });
        }
        assert!(!types(&e, copter).contains(TypeSet::CREATURE));
        activate(&mut e, kit::P0, smugglers_copter(), 2);
        e.apply(kit::P0, PlayerAction::ChooseObjects { objects: vec![elf] })
            .unwrap();
        assert!(is_tapped(&e, elf));
        pass_until(&mut e, stack_is_empty);
        assert!(types(&e, copter).contains(TypeSet::CREATURE));
        assert_eq!(pt(&e, copter), (3, 3));
        let before = library_size(&e, kit::P0);
        if blocking {
            let angel = kit::obj(&e, kit::P1, serra_angel());
            kit::attack(&mut e, &[angel]);
            kit::block(&mut e, &[(copter, angel)]);
        } else {
            kit::attackers_offered(&mut e);
            e.apply(
                kit::P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(copter, Defender::Player(kit::P1))],
                },
            )
            .unwrap();
        }
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        kit::yes_no(&mut e, true);
        assert_eq!(library_size(&e, kit::P0), before - 1);
        let Pending::ChooseCards { options, .. } = e.pending().clone() else {
            panic!("discard after draw")
        };
        e.apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
    }
}
