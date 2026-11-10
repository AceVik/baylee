//! `cards/lands/manlands/nantuko_monastery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nantuko Monastery prints `{{T}}: Add {{C}}` and `Threshold — {G}{W}: This
/// land becomes a 4/4 green and white Insect Monk creature with first strike
/// until end of turn. It's still a land. Activate only if there are seven or
/// more cards in your graveyard.`
///
/// Under `Coverage::Partial`, the threshold animate ability is dropped because
/// the DSL cannot express a graveyard count threshold condition for an
/// activation cost. This test seeds seven cards into the graveyard with {G}
/// and {W} floating, verifies that only the mana ability is offered, and taps
/// it for `{C}`.
#[test]
fn nantuko_monastery_offers_only_the_mana_ability_at_threshold() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[nantuko_monastery(), forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 7);
    tap_all_mana_but(&mut engine, p0, Some(nantuko_monastery()));

    let monastery =
        on_battlefield(&engine, p0, nantuko_monastery()).expect("monastery on battlefield");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal
            .abilities
            .iter()
            .filter(|(id, _)| *id == monastery)
            .count(),
        2,
        "the mana ability and the animation — this assertion read 1 for as \
         long as `Condition` had no way to say \"seven or more cards in your \
         graveyard\", and the ability was left off the card"
    );

    activate(&mut engine, p0, nantuko_monastery(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}}"
    );
}

/// Nantuko Monastery: a 4/4 first-striking Insect Monk that is still a land.
#[test]
fn nantuko_monastery_animates_only_at_threshold() {
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(23, forest())
        .battlefield(0, &[nantuko_monastery(), forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, nantuko_monastery()).expect("in play");

    // `{G}{W}` and no tap, so the Monastery itself may be tapped for
    // mana too — the animation costs it nothing.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("still priority")
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "an empty graveyard is not threshold, and the mana is floating"
    );

    seed_graveyard(&mut engine, p0, 7);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let chars = engine
        .state()
        .object(land)
        .expect("the Monastery is still there")
        .characteristics()
        .clone();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.types.contains(TypeSet::LAND), "it's still a land");
    assert_eq!(pt(&engine, land), (4, 4));
    assert!(chars.keywords.contains(KeywordSet::FIRST_STRIKE));
}
