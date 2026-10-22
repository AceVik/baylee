//! `cards/creatures/mv_2/wood_sage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn wood_sage() -> CardIndex {
    card_index("6fde7bb3-2539-411f-8733-36ce9595dfd5")
}

/// `Wood Sage` ({G}{U}, 1/1 Creature — Human Druid, `Coverage::Partial`):
/// `{T}: Choose a creature card name. Reveal the top four cards of your library
/// and put all of them with that name into your hand. Put the rest into your
/// graveyard.`
#[test]
fn wood_sage_mills_four_cards_and_puts_a_creature_card_into_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, GRIZZLY_BEARS)
        .battlefield(0, &[wood_sage()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, wood_sage(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        3
    );
}
