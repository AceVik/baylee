//! `cards/lands/basic/wastes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wastes prints "{T}: Add {C}" and carries **no basic land type**, and that
/// is the whole of what this scenario is about: a Forest's mana is the
/// CR 305.6 shortcut, while the Wastes' is a printed ability and is therefore
/// offered as `(source, index)` in `LegalActions::abilities` and never in
/// `mana_abilities` — the one place a helper that read a single list would
/// float nothing (#159). Over the basics the two readings agree, which is why
/// the field has to be a Wastes to tell them apart. The {C} is then spent
/// paying for a {1} artifact out of the pool, so what the land made is real
/// generic mana and not a label, and the pool reads empty once the price is
/// paid.
#[test]
fn wastes_prints_its_own_colorless_mana_ability_and_pays_for_a_spell_with_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[wastes(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, wastes());
    assert!(
        engine
            .state()
            .object(land)
            .expect("the Wastes is an object")
            .characteristics()
            .supertypes
            .contains(SupertypeSet::BASIC),
        "a basic land — which is what makes the *missing* basic land type the \
         point rather than scenery"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a land drop hands priority back: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "Wastes prints its own mana ability, so it is an ordinary \
         `(source, index)` entry a printed index can name: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&land),
        "and never the CR 305.6 shortcut, which is basic land types and \
         granted abilities alone: {:?}",
        legal.mana_abilities
    );

    activate(&mut engine, p0, wastes(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "`{{T}}: Add {{C}}` — one colourless, in the pool the moment it is \
         activated"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
    assert!(is_tapped(&engine, land), "the Wastes paid its own {{T}}");

    cast_with_floating(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "`{{1}}` paid out of the pool: the {{C}} is generic mana like any other"
    );
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_none(),
        "and the card it paid for left the hand"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the one mana the Wastes made was the whole of the price — no other \
         source is on this board to have paid it"
    );
}
