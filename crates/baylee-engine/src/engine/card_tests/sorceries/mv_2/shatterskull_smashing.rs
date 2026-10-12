//! `cards/sorceries/mv_2/shatterskull_smashing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The land face pays three life to enter untapped and produces red mana.
#[test]
fn shatterskull_the_hammer_pass_pays_three_life_to_enter_untapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(shatterskull_smashing(), 1).expect("plays as Shatterskull, the Hammer Pass");
    assert!(matches!(engine.pending(), Pending::YesNo {
        player, prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount: 3 }, ..
    } if *player == p0));
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(engine.state().object(land).unwrap().face_index, 1);

    assert!(
        !is_tapped(&engine, land),
        "paid 3 life, so it entered untapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        17,
        "life reduced from 20 to 17"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the land taps for mana");

    assert!(is_tapped(&engine, land), "tapped for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one red mana added to pool"
    );
}
