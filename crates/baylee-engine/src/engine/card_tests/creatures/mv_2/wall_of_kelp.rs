//! `cards/creatures/mv_2/wall_of_kelp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Kelp prints two lines: "Defender" on a 0/3 Plant Wall and
/// "{U}{U}, {T}: Create a 0/1 blue Plant Wall creature token with defender
/// named Kelp."
///
/// Both halves are played: the creature is cast for {U}{U} and
/// stands there as a 0/3 with projected Defender, and its ability — which in
/// its arrival turn fails at CR 302.6, because its cost is its own {T}
/// — is paid in the next own main phase for {U}{U} from the pool.
/// Tapped Wall, empty pool, and the resolved Kelp token are the cost
/// and the effect; the token is read via the projected characteristics
/// (0/1, creature, Defender) and via the token entry itself (name "Kelp",
/// blue), because neither of the two readings replaces the other.
#[test]
fn wall_of_kelp_taps_and_two_blue_for_a_kelp_token_with_defender() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[wall_of_kelp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, wall_of_kelp());
    pass_until(&mut engine, stack_is_empty);
    let wall = on_battlefield(&engine, p0, wall_of_kelp()).expect("the Wall resolved");
    assert_eq!(pt(&engine, wall), (0, 3), "the printed 0/3 body");
    assert!(
        keywords(&engine, wall).contains(KeywordSet::DEFENDER),
        "Defender is printed on the card and reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "four Islands pay {{U}}{{U}} for the Wall and leave exactly the \
         {{U}}{{U}} the ability charges still floating"
    );

    // A turn later — CR 302.6: a creature that arrived this turn
    // cannot activate an ability with the tap symbol in its cost,
    // and the Islands pay the next cost only again in
    // an own main phase from a fresh pool.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, wall),
        "der Enttappschritt hat sie aufgeweckt"
    );

    // First mana into the pool, then the claim: `tap_all_mana` taps any
    // ability whose entire cost is its own {T}, and the Wall is not one of
    // them — its cost is {U}{U} *and* the tap symbol (#159).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands, and the Wall makes no mana"
    );
    activate(&mut engine, p0, wall_of_kelp(), 0);
    assert!(
        is_tapped(&engine, wall),
        "{{T}} ist die eine Hälfte des Preises (CR 601.2h)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{U}}{{U}} is the other and came from the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "creating a token is not a mana ability, so it goes on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    let kelp = tokens_of(&engine, p0);
    assert_eq!(kelp.len(), 1, "eine Aktivierung, ein Kelp");
    let kelp = kelp[0];
    assert!(
        types(&engine, kelp).contains(TypeSet::CREATURE),
        "the Kelp is a creature"
    );
    assert_eq!(
        pt(&engine, kelp),
        (0, 1),
        "und hat den gedruckten 0/1-Körper"
    );
    assert!(
        keywords(&engine, kelp).contains(KeywordSet::DEFENDER),
        "\"with defender\" — the token cannot attack"
    );
    let printed = engine
        .state()
        .object(kelp)
        .and_then(|o| o.token)
        .expect("der Token weiß, welcher Token er ist");
    assert_eq!(printed.name, "Kelp", "der Token heißt Kelp");
    assert_eq!((printed.power, printed.toughness), (Some(0), Some(1)));
    assert!(
        printed.colors.contains(baylee_core::color::Color::Blue),
        "ein blauer Kelp"
    );
}
