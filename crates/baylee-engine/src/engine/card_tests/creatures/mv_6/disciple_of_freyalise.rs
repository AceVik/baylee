//! `cards/creatures/mv_6/disciple_of_freyalise.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Disciple of Freyalise, played as its back face: the land pays 3 life to
/// arrive untapped and then makes green.
///
/// The back face uses the modal double-faced land entry rule: `EnterModifier::TappedOrPayLife(3)` asks, and
/// the answer decides whether the mana is available this turn.
#[test]
fn garden_of_freyalise_pays_three_life_to_arrive_untapped() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(disciple_of_freyalise(), 1).expect("the back face is a land");
    assert!(matches!(engine.pending(), Pending::YesNo {
        player, prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount: 3 }, ..
    } if *player == p0));
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(engine.state().object(land).unwrap().face_index, 1);

    assert!(
        !is_tapped(&engine, land),
        "3 life was paid, so it entered untapped"
    );
    assert_eq!(engine.state().players[0].life, 17, "and the 3 life is gone");
    // A *printed* mana ability is enumerated into `LegalActions::abilities`
    // like any other activated ability; `mana_abilities` is the CR 305.6 land
    // shortcut and what a continuous effect granted.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the land taps for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green in the pool"
    );
}

#[test]
fn disciple_gains_and_draws_the_sacrificed_creatures_last_power_only_if_paid() {
    let p0 = PlayerId::new(0);
    for pay in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .hand(0, &[disciple_of_freyalise()])
            .battlefield(0, &[baylee_core::generated::index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bear = on_battlefield(&e, p0, baylee_core::generated::index::GRIZZLY_BEARS).unwrap();
        crate::replacement::put_counters(e.dev_state_mut(p0).unwrap(), bear, CounterKind::P1P1, 2);
        give_mana(&mut e, p0, ManaColor::Green, 6);
        cast_from_hand(&mut e, p0, disciple_of_freyalise());
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::CostSacrifice,
                    ..
                }
            )
        });
        let disciple = on_battlefield(&e, p0, disciple_of_freyalise()).unwrap();
        let Pending::ChooseCards { options, .. } = e.pending() else {
            unreachable!()
        };
        assert!(options.contains(&bear));
        assert!(!options.contains(&disciple));
        let before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: if pay { vec![bear] } else { vec![] },
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, if pay { 24 } else { 20 });
        assert_eq!(
            e.state().zones.list(ZoneLocation::Hand(p0)).len(),
            before + if pay { 4 } else { 0 }
        );
        assert_eq!(
            e.state().object(bear).unwrap().zone,
            if pay {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            }
        );
    }
}
