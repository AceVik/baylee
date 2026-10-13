//! `cards/creatures/mv_7/drowner_of_truth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The printed mana or land-face ability is exercised through player actions.
#[test]
fn drowned_jungle_enters_tapped_and_taps_for_either_colour() {
    let (engine, land) = play_land_face(drowner_of_truth(), 1).expect("the back face is a land");
    assert!(
        is_tapped(&engine, land),
        "the printed \"This land enters tapped\" is an enter modifier and not a \
         line the harness can place around"
    );
}

#[test]
fn drowner_only_triggers_for_colorless_actually_spent_and_tokens_survive_the_spell_being_countered()
{
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for (colorless, countered) in [(false, false), (true, false), (true, true)] {
        let mut e = Duel::new(SEED, forest())
            .hand(0, &[drowner_of_truth()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let card = in_hand(&e, p0, drowner_of_truth()).unwrap();
        assert!(
            e.state()
                .object(card)
                .unwrap()
                .characteristics()
                .colors
                .is_empty()
        );
        {
            let s = e.dev_state_mut(p0).unwrap();
            s.players[0]
                .mana_pool
                .add(ManaColor::Green, if colorless { 6 } else { 7 });
            if colorless {
                s.players[0].mana_pool.add(ManaColor::Colorless, 1);
            }
        }
        e.refresh_offer();
        cast_with_floating(&mut e, p0, index::DROWNER_OF_TRUTH);
        assert_eq!(
            e.state().zones.list(ZoneLocation::Stack).len(),
            if colorless { 2 } else { 1 }
        );
        if countered {
            e.dev_state_mut(p0)
                .unwrap()
                .move_object(
                    card,
                    ZoneLocation::Graveyard(p0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        let tokens = tokens_of(&e, p0);
        assert_eq!(tokens.len(), if colorless { 2 } else { 0 });
        if colorless {
            let token = tokens[0];
            assert_eq!(pt(&e, token), (0, 1));
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: token,
                    ability_index: 0,
                },
            )
            .unwrap();
            assert!(stack_is_empty(&e));
            assert_eq!(
                e.state().players[0]
                    .mana_pool
                    .available(ManaColor::Colorless),
                1
            );
            assert_eq!(tokens_of(&e, p0).len(), 1);
        }
    }
}

#[test]
fn drowned_jungle_asks_green_or_blue_and_adds_the_one_chosen() {
    let p0 = PlayerId::new(0);
    for (chosen, other) in [
        (ManaColor::Blue, ManaColor::Green),
        (ManaColor::Green, ManaColor::Blue),
    ] {
        let (mut e, land) = play_land_face(drowner_of_truth(), 1).expect("the back face is a land");
        e.dev_state_mut(p0).unwrap().set_tapped(land, false);
        e.refresh_offer();
        activate(&mut e, p0, drowner_of_truth(), 0);
        let Pending::ChooseColor { player, options } = e.pending().clone() else {
            panic!("{{T}}: Add {{G}} or {{U}} asks which: {:?}", e.pending())
        };
        assert_eq!(player, p0);
        let mut offered = options.clone();
        offered.sort_by_key(|c| format!("{c:?}"));
        let mut want = vec![ManaColor::Green, ManaColor::Blue];
        want.sort_by_key(|c| format!("{c:?}"));
        assert_eq!(offered, want, "exactly green and blue are on offer");
        e.apply(p0, PlayerAction::ChooseColor(chosen)).unwrap();
        assert!(is_tapped(&e, land), "{{T}}: the land taps for the mana");
        let pool = &e.state().players[0].mana_pool;
        assert_eq!(pool.available(chosen), 1, "one mana of the chosen colour");
        assert_eq!(pool.available(other), 0, "and none of the other colour");
        assert_eq!(pool.available(ManaColor::Colorless), 0);
    }
}
