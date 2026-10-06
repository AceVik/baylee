//! `cards/sorceries/mv_2/shatterskull_smashing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Shatterskull Smashing` // `Shatterskull, the Hammer Pass` (`Coverage::Partial`):
/// "Shatterskull Smashing deals X damage divided as you choose among up to two target creatures
/// and/or planeswalkers. If X is 6 or more, Shatterskull Smashing deals twice X damage divided
/// as you choose among them instead. // As this land enters, you may pay 3 life. If you don't,
/// it enters tapped. `{{T}}`: Add `{{R}}`."
///
/// Under `Coverage::Partial`, the front-face sorcery is not expressible in the DSL, while the
/// back-face land (`Shatterskull, the Hammer Pass`) is implemented in full. The test plays the back
/// face as a land, pays 3 life to arrive untapped, and activates its mana ability to add `{{R}}`.
#[test]
fn shatterskull_the_hammer_pass_pays_three_life_to_enter_untapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(shatterskull_smashing(), 1).expect("plays as Shatterskull, the Hammer Pass");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }

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
