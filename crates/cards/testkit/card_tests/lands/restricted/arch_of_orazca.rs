//! Printed rules exercised through the real game driver.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn arch_of_orazca_requires_ten_permanents_then_keeps_the_blessing() {
    let p0 = PlayerId::new(0);
    for lands in [8, 9] {
        let mut board = vec![arch_of_orazca()];
        board.extend(std::iter::repeat_n(forest(), lands));
        let mut e = Duel::new(SEED, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let arch = on_battlefield(&e, p0, arch_of_orazca()).unwrap();
        tap_all_mana_but(&mut e, p0, Some(arch_of_orazca()));
        if lands == 8 {
            assert!(
                e.apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: arch,
                        ability_index: 1
                    }
                )
                .is_err()
            );
        } else {
            let other = on_battlefield(&e, p0, forest()).unwrap();
            bury(&mut e, &[other]);
            e.refresh_offer();
            let before = library_size(&e, p0);
            activate(&mut e, p0, arch_of_orazca(), 1);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(library_size(&e, p0), before - 1);
        }
    }
}
