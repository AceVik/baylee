//! `cards/sorceries/mv_1/lab_rats.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;

const P0: PlayerId = PlayerId::new(0);

fn lab_rats() -> CardIndex {
    card_index("c53d6d3e-64e3-48b9-afab-3bde3164888f")
}

/// Lab Rats (`c53d6d3e-64e3-48b9-afab-3bde3164888f`) — {B} Sorcery (Partial):
/// "Create a 1/1 black Rat creature token."
#[test]
fn lab_rats_creates_one_one_black_rat_token() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[lab_rats()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lab_rats());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, lab_rats()).is_some(),
        "base spell does not get free buyback"
    );

    let tokens = tokens_of(&engine, P0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(power_toughness(&engine, tokens[0]), (1, 1));
    let token = engine.state().object(tokens[0]).unwrap().characteristics();
    assert_eq!(
        token.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Black])
    );
    assert!(
        token
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::RAT)
    );
}
