//! `cards/creatures/mv_2/sunstrike_legionnaire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Sunstrike Legionnaire (`e554646f-27c0-49b3-a92c-2191e87d87b7`) — {1}{W} Creature — Human Soldier 1/2:
/// "This creature doesn't untap during your untap step.
/// Whenever another creature enters, untap this creature.
/// {T}: Tap target creature with mana value 3 or less."
#[test]
fn sunstrike_legionnaire_taps_small_creature_skips_untap_and_untaps_on_creature_etb() {
    let legionnaire_card = card_index("e554646f-27c0-49b3-a92c-2191e87d87b7");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[legionnaire_card, forest(), forest()])
        .battlefield(1, &[gray_ogre(), index::HILL_GIANT])
        .hand(0, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let legionnaire = on_battlefield(&engine, p0, legionnaire_card).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();

    activate(&mut engine, p0, legionnaire_card, 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Sunstrike Legionnaire");
    };
    assert!(options.contains(&ogre));
    assert!(!options.contains(&giant));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(ogre)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(
        engine
            .state()
            .object(legionnaire)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // Stays tapped across p0's next untap step.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(legionnaire)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // Untaps when another creature enters the battlefield.
    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !engine
            .state()
            .object(legionnaire)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
