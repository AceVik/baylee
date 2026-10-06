//! `cards/lands/shock/breeding_pool.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Breeding Pool` enters untapped for 2 life or tapped for free under `Coverage::Implemented`.
/// In the first game, declining the payment leaves the land tapped with no life lost.
/// In the second game, paying 2 life allows it to enter untapped at the cost of 2 life.
#[test]
fn breeding_pool_enters_untapped_for_two_life_or_tapped_for_free() {
    let p0 = PlayerId::new(0);

    // Arm 1: decline -> enters tapped, no life lost.
    {
        let mut engine = Duel::new(2312, forest())
            .hand(0, &[breeding_pool()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let life_before = engine.state().players[0].life;
        let land = play_land(&mut engine, p0, breeding_pool());

        let Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
            ..
        } = engine.pending().clone()
        else {
            panic!("expected PayLifeOrEnterTapped, got {:?}", engine.pending());
        };
        assert_eq!(player, p0);
        assert_eq!(amount, 2);

        engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        assert!(is_tapped(&engine, land));
        assert_eq!(engine.state().players[0].life, life_before);
    }

    // Arm 2: accept -> enters untapped, costs 2 life.
    {
        let mut engine = Duel::new(2313, forest())
            .hand(0, &[breeding_pool()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let life_before = engine.state().players[0].life;
        let land = play_land(&mut engine, p0, breeding_pool());

        let Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
            ..
        } = engine.pending().clone()
        else {
            panic!("expected PayLifeOrEnterTapped, got {:?}", engine.pending());
        };
        assert_eq!(player, p0);
        assert_eq!(amount, 2);

        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        assert!(!is_tapped(&engine, land));
        assert_eq!(engine.state().players[0].life, life_before - 2);
    }
}

/// Breeding Pool under Chromatic Lantern taps for either of its colours
/// through either door the offer lists.
///
/// The land prints "Add {G} or {U}" (`legal.abilities`), and the Lantern
/// grants it "{T}: Add one mana of any color" (`legal.mana_abilities`, the
/// door with no index). The offer listed the second door and `apply` refused
/// it: it asked only whether a land with basic types could be tapped, took
/// the CR 305.6 shortcut, and found no colour there, because the card's own
/// ability already makes both. 63 of 10,000 fuzzed games pressed that refused
/// door (Breeding Pool 39, Stomping Ground 14, Canopy Vista 10).
#[test]
fn breeding_pool_under_chromatic_lantern_taps_for_either_colour_through_either_door() {
    let p0 = PlayerId::new(0);
    for colour in [ManaColor::Green, ManaColor::Blue] {
        for granted in [true, false] {
            let mut engine = Duel::new(2314, forest())
                .battlefield(0, &[chromatic_lantern()])
                .hand(0, &[breeding_pool()])
                .start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, p0);
            let land = play_land(&mut engine, p0, breeding_pool());
            engine
                .apply(p0, PlayerAction::YesNo(true))
                .expect("2 life for an untapped land");
            pass_until(&mut engine, stack_is_empty);
            assert!(!is_tapped(&engine, land), "it entered untapped");

            let Pending::Priority { legal, .. } = engine.pending().clone() else {
                panic!("expected priority, got {:?}", engine.pending())
            };
            let press = if granted {
                assert!(
                    legal.mana_abilities.contains(&land),
                    "the Lantern's grant is offered: {legal:?}"
                );
                PlayerAction::ActivateManaAbility { source: land }
            } else {
                assert!(
                    legal.abilities.contains(&(land, 0)),
                    "the printed ability is offered: {legal:?}"
                );
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 0,
                }
            };
            engine
                .apply(p0, press)
                .expect("a press the offer listed is taken");

            let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
                panic!("the colour is asked: {:?}", engine.pending())
            };
            assert!(options.contains(&colour), "{colour:?} is among {options:?}");
            assert_eq!(
                options.len(),
                if granted { 5 } else { 2 },
                "any colour through the grant, the printed two through the card"
            );
            engine
                .apply(p0, PlayerAction::ChooseColor(colour))
                .expect("a colour the engine offered");

            assert!(is_tapped(&engine, land), "the land paid its {{T}}");
            assert_eq!(
                engine.state().players[0].mana_pool.available(colour),
                1,
                "one {colour:?} floats (granted: {granted})"
            );
        }
    }
}
