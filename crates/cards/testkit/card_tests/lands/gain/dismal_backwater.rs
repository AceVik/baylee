//! `cards/lands/gain/dismal_backwater.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dismal Backwater prints three sentences and nothing else: it enters
/// tapped, its controller gains 1 life as it arrives, and it taps for {U} or
/// {B}. All three are read off one land over two turns, and the order is the
/// point: a land that entered tapped has no {T} to pay until its controller's
/// untap step (CR 502.3), so the same object that offers nothing to activate
/// on the turn it lands offers a two-colour choice on the next. The life is
/// checked against the seat that played it rather than the table, and the
/// mana against the question itself, because "{U} or {B}" and "any colour"
/// both hand back one mana and only the options tell the two apart.
#[test]
fn dismal_backwater_enters_tapped_gains_a_life_and_taps_for_blue_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[dismal_backwater()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = engine.state().players[0].life;
    let their_life = engine.state().players[1].life;

    let land = play_land(&mut engine, p0, dismal_backwater());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — the permanent is already tapped as it lands"
    );

    pass_until(&mut engine, |e| e.state().players[0].life > life_before);
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"When this land enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        their_life,
        "the life belongs to the land's controller and not to the table"
    );

    // The tapped entry is a rules state and not a status bit: a `{T}` price
    // this board cannot pay is a price the offer withholds.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "the trigger resolved into a priority round: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the active player holds priority again");
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a land that entered tapped has nothing to activate until it untaps: {:?}",
        legal.abilities
    );

    // A whole turn cycle, so that the untap step (CR 502.3) stands it up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step put it back up, which is what makes the tap below \
         the card's own and not the harness'"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing was floated on the way: the pool is empty before the tap"
    );

    // Ability 1 is the printed "{T}: Add {U} or {B}"; ability 0 is the
    // enters-trigger, which is never offered as an activation.
    activate(&mut engine, p0, dismal_backwater(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"{{U}} or {{B}}\" is a choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "the two colours it prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "\"{{U}} or {{B}}\" and not one colour more: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "and no other colour came with it"
    );
    assert_eq!(
        pool.total(),
        1,
        "one tap, one mana — nothing else on this board makes any"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
