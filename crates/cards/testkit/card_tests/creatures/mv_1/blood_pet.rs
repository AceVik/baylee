//! `cards/creatures/mv_1/blood_pet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blood Pet prints one line — "Sacrifice this creature: Add {B}" — and that
/// price is the whole card: a mana ability (CR 605.1) whose cost is the
/// permanent itself, so the scenario plays two of them for real and then
/// presses the ability, because no tap reaches it. The {B} in the pool
/// afterwards cannot be the Swamps', since both were spent on the two casts,
/// and the second Pet is the control: `SacrificeSelf` names its own source, so
/// the activation asks no question and empties the table of exactly one
/// permanent, where a generic "sacrifice a creature" cost would have put both
/// on a `CostSacrifice` menu.
#[test]
fn blood_pet_sacrifices_itself_for_one_black_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[blood_pet(), blood_pet()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Both Pets arrive the way the card arrives. `cast_from_hand` taps both
    // Swamps before it casts, and the first Pet is still standing after that
    // tap — which is the other half of the price being the body and not a
    // `{T}`: a `SacrificeSelf` route is not one `tap_all_mana` may take.
    cast_from_hand(&mut engine, p0, blood_pet());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        all_on_battlefield(&engine, p0, blood_pet()).len(),
        1,
        "the Pet on the table is not a mana route the helper may press"
    );
    cast_from_hand(&mut engine, p0, blood_pet());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let pets = all_on_battlefield(&engine, p0, blood_pet());
    assert_eq!(pets.len(), 2, "both Pets resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and both Swamps are spent, so whatever the ability adds is the Pet's"
    );

    let pet = pets[0];
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(pet, 0)),
        "the card's only line is the mana ability, and a mana ability a card \
         prints arrives as an ordinary `(source, index)` entry: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pet,
                ability_index: 0,
            },
        )
        .expect("the ability activates");

    // Nothing stands between the activation and its payment: the cost names
    // its own source, so there is nothing to choose (no `CostSacrifice` menu,
    // which is how a sacrifice-a-creature price would have asked) and nothing
    // on the stack (CR 605.3b).
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "a `SacrificeSelf` price asks nothing, got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "a mana ability never uses the stack"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "{{B}}, and black is the color the card prints"
    );
    assert_eq!(pool.total(), 1, "and one mana is the whole of what it made");
    assert_eq!(
        all_on_battlefield(&engine, p0, blood_pet()).len(),
        1,
        "the Pet paid with itself: the one beside it was never on the menu"
    );
    assert!(
        in_graveyard(&engine, p0, blood_pet()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
}
