//! `cards/sorceries/mv_2/reshape.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The artifact is paid before the search; X limits what the search can find.
#[test]
fn reshape_pays_its_artifact_cost_and_searches_with_the_announced_x() {
    use crate::engine::card_tests::legends_kit as kit;
    for x in 0..=1 {
        let mut e = Duel::new(970, quiet_artifact())
            .battlefield(0, &[chromatic_lantern(), llanowar_elves()])
            .hand(0, &[reshape()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, kit::P0);
        let artifact = kit::obj(&e, kit::P0, chromatic_lantern());
        let elf = kit::obj(&e, kit::P0, llanowar_elves());
        kit::float(
            &mut e,
            kit::P0,
            &[(ManaColor::Blue, 2), (ManaColor::Colorless, x)],
        );
        cast_with_floating(&mut e, kit::P0, reshape());
        e.apply(kit::P0, PlayerAction::ChooseNumber(x)).unwrap();
        let Pending::ChooseCards { options, .. } = e.pending() else {
            panic!("expected sacrifice")
        };
        assert_eq!(options, &[artifact]);
        assert!(
            e.apply(kit::P0, PlayerAction::ChooseObjects { objects: vec![elf] })
                .is_err()
        );
        e.apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: vec![artifact],
            },
        )
        .unwrap();
        assert_eq!(e.state().object(artifact).unwrap().zone, Zone::Graveyard);
        assert!(!stack_is_empty(&e));
        assert_eq!(kit::pool_total(&e, kit::P0), 0);
        let before = library_size(&e, kit::P0);
        if x == 1 {
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseCards { .. })
            });
            let Pending::ChooseCards { options, .. } = e.pending().clone() else {
                unreachable!()
            };
            assert!(!options.is_empty());
            e.apply(
                kit::P0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library_size(&e, kit::P0), before - x as usize);
        let found = on_battlefield(&e, kit::P0, quiet_artifact());
        assert_eq!(found.is_some(), x == 1);
        if let Some(found) = found {
            assert!(!is_tapped(&e, found));
        }
    }
}

#[test]
fn reshape_without_an_artifact_is_not_castable_despite_enough_mana() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut e = kit::game(971, &[], &[chromatic_lantern()], &[reshape()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Blue, 5)]);
    let card = in_hand(&e, kit::P0, reshape()).unwrap();
    assert!(!priority_offer(&e).castable.contains(&card));
    assert!(e.apply(kit::P0, PlayerAction::CastSpell { card }).is_err());
}
