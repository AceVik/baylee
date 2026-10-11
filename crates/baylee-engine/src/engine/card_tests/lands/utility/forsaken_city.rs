//! `cards/lands/utility/forsaken_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forsaken City prints `This land doesn't untap during your untap step`, `At the beginning of your
/// upkeep, you may exile a card from your hand. If you do, untap this land`, and `{T}: Add one mana of any color.`
/// Forsaken City enters untapped, activates ability index 1 to add chosen mana, and `Modifier::DoesNotUntap`
/// keeps it tapped through subsequent untap steps.
#[test]
fn forsaken_city_adds_any_color_and_does_not_untap_during_untap_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[forsaken_city()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, forsaken_city());
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, forsaken_city(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice");
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
    });
    assert!(is_tapped(&engine, land));
}

#[test]
fn forsaken_city_exiles_a_card_from_its_controllers_hand_to_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for pay in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[forsaken_city()])
            .hand(0, &[forest()])
            .hand(1, &[forest()])
            .start();
        let city = on_battlefield(&e, p0, forsaken_city()).unwrap();
        e.dev_state_mut(p0).unwrap().set_tapped(city, true);
        keep_mulligans(&mut e);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::CostExile,
                    ..
                }
            )
        });
        assert!(is_tapped(&e, city));
        let card = in_hand(&e, p0, forest()).unwrap();
        let theirs = in_hand(&e, p1, forest()).unwrap();
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = e.pending()
        else {
            unreachable!()
        };
        assert_eq!(*player, p0);
        assert_eq!((*min, *max), (0, 1));
        assert!(options.contains(&card));
        assert!(!options.contains(&theirs));
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: if pay { vec![card] } else { vec![] },
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(is_tapped(&e, city), !pay);
        assert_eq!(
            e.state().object(card).unwrap().zone,
            if pay { Zone::Exile } else { Zone::Hand }
        );
    }
}
