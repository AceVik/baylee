//! `cards/creatures/mv_7/the_terror_of_serpent_s_pass.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn the_terror_of_serpent_s_pass() -> CardIndex {
    card_index("771bf51d-cb32-4240-936c-f9d1e0c90cf0")
}

/// The Terror of Serpent's Pass — a legendary 8/8 with hexproof for
/// {5}{U}{U}.
#[test]
fn the_terror_of_serpent_s_pass_costs_seven_and_is_an_eight_eight() {
    let mut engine = table7(&lots(island(), 7), &[the_terror_of_serpent_s_pass()], &[]);
    let terror = cast_and_resolve(&mut engine, the_terror_of_serpent_s_pass());
    assert_body(&engine, terror, (8, 8), KeywordSet::HEXPROOF);
    assert!(
        engine
            .state()
            .object(terror)
            .expect("out")
            .characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY),
        "legendary"
    );
}

/// Hexproof: their Bolt may aim at the Ogre and not at the Terror.
#[test]
fn the_terror_of_serpent_s_pass_hexproof_keeps_their_bolt_off_it() {
    let (menu, terror, ogre) = bolt_menu(the_terror_of_serpent_s_pass(), 1);
    assert!(menu.contains(&ogre));
    assert!(!menu.contains(&terror));
}
