//! `cards/lands/utility/tolaria.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tolaria prints two abilities and only the first is written: "{T}: Add
/// {U}", where the upkeep-speed "target creature loses banding and all
/// 'bands with other' abilities" line is the `Coverage::Partial` gap. The land
/// is *played* rather than seeded, because `starting_battlefield` places a
/// permanent through `Cause::Setup` and never asks a land's own entry rules
/// anything — so the untapped arrival is earned here and not assumed. One
/// printed ability in the offer, one blue mana in a pool nothing else on the
/// board could have filled, and no stack under it (CR 605.3b) is the whole of
/// what the card implements.
#[test]
fn tolaria_plays_untapped_and_its_one_ability_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest()).hand(0, &[tolaria()]).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, tolaria());
    assert_eq!(
        on_battlefield(&engine, p0, tolaria()),
        Some(land),
        "the land drop put Tolaria onto the battlefield"
    );
    assert!(
        !entered_tapped(&engine, land),
        "it prints no enters-tapped clause, so it arrives ready to tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the tap"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop leaves the seat holding priority: {:?}",
            engine.pending()
        )
    };
    // Index 0 is the printed mana ability; the second printed line is written
    // nowhere on the card def, so an offer carrying anything else would name
    // an ability this test never read.
    assert_eq!(
        legal
            .abilities
            .iter()
            .filter(|(source, _)| *source == land)
            .map(|(_, index)| *index)
            .collect::<Vec<u32>>(),
        vec![0],
        "one printed ability and no other: the banding line is the Partial gap"
    );

    activate(&mut engine, p0, tolaria(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "{{U}}, and not {{C}}");
    assert_eq!(
        pool.available(ManaColor::Colorless),
        0,
        "one blue and nothing beside it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
