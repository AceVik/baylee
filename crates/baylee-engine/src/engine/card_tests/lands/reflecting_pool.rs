//! `cards/lands/reflecting_pool.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A Reflecting Pool beside an Uncharted Haven sees the colour the Haven was
/// told to make.
///
/// The Pool reads `produced_colors`, which is a reading of the *card* — and a
/// land whose whole mana is "one mana of the chosen color" has nothing there
/// to read, because the card cannot know. So the Pool was looking at a land
/// that makes nothing. It takes both halves: `produced_chosen` is the card's
/// ("this one reads a chosen colour") and `GameObject::chosen_color` is the
/// object's, because the field is written by an entry and says nothing on its
/// own about what the permanent does with it.
#[test]
fn a_reflecting_pool_sees_the_colour_a_neighbour_was_told_to_make() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(731, forest())
        .battlefield(0, &[reflecting_pool()])
        .hand(0, &[uncharted_haven()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pool = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == reflecting_pool())
        })
        .expect("the Pool was seated");

    play_land(&mut engine, p0, uncharted_haven());
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black is a colour");
    pass_until(&mut engine, stack_is_empty);

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pool,
                ability_index: 0,
            },
        )
        .expect("the Pool taps");
    // One option is not a choice, so nothing is asked and the mana is simply
    // added (`resolve::mana` short-circuits a single colour).
    let mana = &engine.state().players[0].mana_pool;
    assert_eq!(
        mana.available(ManaColor::Black),
        1,
        "the colour its neighbour was told to make"
    );
    assert_eq!(
        mana.total(),
        1,
        "and nothing else: the Haven makes one thing"
    );
}
