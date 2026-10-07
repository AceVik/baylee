//! `cards/lands/towns/adventurer_s_inn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Adventurer's Inn prints two lines and one real play reads both: "When
/// this land enters, you gain 2 life" and "{T}: Add {C}". The life is
/// measured after a genuine `PlayLand`, because an entry is the only thing a
/// land's own trigger can look at — a `starting_battlefield` placement moves
/// the card with `Cause::Setup` and would leave the total at twenty whatever
/// the card says. The mana half is then read off the pool with an empty
/// stack, since a mana ability never uses one (CR 605.3b), and `{C}` is the
/// colour that tells this land from the Forest it is being played beside.
#[test]
fn adventurers_inn_gains_two_life_on_entry_and_taps_for_a_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[adventurers_inn()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = in_hand(&engine, p0, adventurers_inn()).expect("the Inn is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .expect("a land drop on an empty board is legal");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let inn = on_battlefield(&engine, p0, adventurers_inn()).expect("the Inn entered");
    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"When this land enters, you gain 2 life\" — read off the entry, not \
         off a placement no trigger watches"
    );
    assert!(
        !is_tapped(&engine, inn),
        "it enters untapped, so its {{T}} is already standing to be paid"
    );

    // The one activated line the card prints is a mana ability a card prints,
    // so it is an ordinary `(source, index)` entry in `abilities` with an
    // index to name rather than the CR 305.6 shortcut a basic land uses.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == inn)
        .expect("{{T}}: Add {{C}} is offered on a board that holds no mana at all");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ability came out of the list that offered it");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\" — one colourless, off one tap"
    );
    assert_eq!(
        pool.total(),
        1,
        "and nothing else came with it: the Forest beside it was never tapped"
    );
    assert!(is_tapped(&engine, inn), "the Inn paid its own {{T}}");
}
