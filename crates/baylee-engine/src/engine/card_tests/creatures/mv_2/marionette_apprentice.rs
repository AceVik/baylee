//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn fabricate_is_chosen_on_resolution_and_doubles_either_result() {
    use crate::engine::card_tests::token_helpers::named;
    let p0 = PlayerId::new(0);
    for case in 0..3 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::DOUBLING_SEASON])
            .hand(0, &[index::MARIONETTE_APPRENTICE])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Black, 2);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, index::MARIONETTE_APPRENTICE);
        pass_until(&mut e, |e| {
            on_battlefield(e, p0, index::MARIONETTE_APPRENTICE).is_some()
        });
        let apprentice = on_battlefield(&e, p0, index::MARIONETTE_APPRENTICE).unwrap();
        if case == 2 {
            bury(&mut e, &[apprentice]);
        } else {
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseCards { .. })
            });
            e.apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: if case == 1 { vec![apprentice] } else { vec![] },
                },
            )
            .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            named(&e, "Servo").len(),
            if case == 1 { 0 } else { 2 },
            "case {case}"
        );
        if case != 2 {
            assert_eq!(
                e.state()
                    .object(apprentice)
                    .unwrap()
                    .counters
                    .get(baylee_cards_dsl::CounterKind::P1P1),
                if case == 1 { 2 } else { 0 }
            );
        }
        if case == 0 {
            let life = e.state().players[1].life;
            let servo = named(&e, "Servo")[0];
            bury(&mut e, &[servo]);
            e.apply(p0, PlayerAction::PassPriority).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(
                e.state().players[1].life,
                life - 1,
                "the new artifact creature triggers the drain once"
            );
        }
    }
}
