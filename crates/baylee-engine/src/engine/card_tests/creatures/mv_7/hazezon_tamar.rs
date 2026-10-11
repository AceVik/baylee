//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn hazezon_counts_lands_at_next_own_upkeep_even_after_leaving() {
    use crate::engine::card_tests::token_helpers::named;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for departed in [false, true] {
        for doubled in [false, true] {
            let mut board = vec![forest(), forest()];
            if doubled {
                board.push(index::DOUBLING_SEASON);
            }
            let mut e = Duel::new(SEED, forest())
                .battlefield(0, &board)
                .hand(0, &[index::HAZEZON_TAMAR, forest()])
                .start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, p0);
            let pool = &mut e.dev_state_mut(p0).unwrap().players[0].mana_pool;
            pool.add(ManaColor::White, 5);
            pool.add(ManaColor::Red, 1);
            pool.add(ManaColor::Green, 1);
            e.refresh_offer();
            cast_with_floating(&mut e, p0, index::HAZEZON_TAMAR);
            pass_until(&mut e, stack_is_empty);
            let source = on_battlefield(&e, p0, index::HAZEZON_TAMAR).unwrap();
            assert!(named(&e, "Sand Warrior").is_empty());
            if departed {
                kill(&mut e, source);
            }
            play_land(&mut e, p0, forest());
            reach_their_main_phase(&mut e, p1);
            assert!(
                named(&e, "Sand Warrior").is_empty(),
                "not the opponent's upkeep"
            );
            reach_their_main_phase(&mut e, p0);
            let tokens = named(&e, "Sand Warrior");
            assert_eq!(
                tokens.len(),
                if doubled { 6 } else { 3 },
                "counted the third land added later"
            );
            for token in &tokens {
                assert_eq!(pt(&e, *token), (1, 1));
                assert_eq!(
                    e.state().object(*token).unwrap().characteristics().colors,
                    baylee_core::color::ColorSet::from_slice(&[
                        baylee_core::color::Color::Red,
                        baylee_core::color::Color::Green,
                        baylee_core::color::Color::White
                    ])
                );
            }
            if !departed {
                e.dev_state_mut(p0)
                    .unwrap()
                    .object_mut(tokens[0])
                    .unwrap()
                    .controller = p1;
                kill(&mut e, source);
                assert!(
                    named(&e, "Sand Warrior").is_empty(),
                    "exiles every player's Sand Warriors"
                );
            }
            reach_their_main_phase(&mut e, p1);
            reach_their_main_phase(&mut e, p0);
            assert_eq!(
                named(&e, "Sand Warrior").len(),
                if departed { tokens.len() } else { 0 },
                "delayed trigger fires only once"
            );
        }
    }
}
