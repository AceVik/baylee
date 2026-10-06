//! `cards/sorceries/mv_3/agadeem_s_awakening.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Agadeem's Awakening` // `Agadeem, the Undercrypt` (`Coverage::Partial`): "Return from
/// your graveyard to the battlefield any number of target creature cards that each have a
/// different mana value X or less. // As this land enters, you may pay 3 life. If you don't,
/// it enters tapped. {T}: Add {B}."
///
/// Under `Coverage::Partial`, the front-face reanimation clause is not expressible in the
/// DSL, but the back face (`Agadeem, the Undercrypt`) is fully modeled. The test plays the
/// land face, pays 3 life on the `EnterModifier::TappedOrPayLife(3)` prompt to have it enter
/// untapped, and immediately activates its mana ability to add `{B}`.
#[test]
fn agadeem_the_undercrypt_pays_three_life_to_enter_untapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(agadeem_s_awakening(), 1).expect("plays as Agadeem, the Undercrypt");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }

    assert!(
        !is_tapped(&engine, land),
        "3 life was paid, so it entered untapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        17,
        "paid 3 life to enter untapped"
    );

    let black_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Black);
    activate(&mut engine, p0, agadeem_s_awakening(), 0);

    assert!(is_tapped(&engine, land), "tapped to activate mana ability");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        black_before + 1,
        "adds one black mana to the pool"
    );
}
