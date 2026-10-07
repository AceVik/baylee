//! `cards/lands/utility/serpent_s_pass.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "f715f701-a735-42ab-b31a-8e1bd04ac5ff"

/// Serpent's Pass prints three lines: it enters tapped, it taps for {U} or
/// {B}, and it sells itself — "{4}, {T}, Sacrifice this land: Draw a card."
/// All three are read off one game, because each is the other's control. The
/// entry is only visible through a real `PlayLand` (a `starting_battlefield`
/// placement is moved with `Cause::Setup`, which no replacement effect looks
/// at), and it is what keeps the printed `{T}` out of the offer for a whole
/// turn — so the one mana the ability then makes is exact, off an empty pool
/// and a board of tapped Islands. The last line is the only one that costs the
/// land itself, and the `{4}` is read where `can_afford` reads it: off the
/// pool, never off the untapped lands.
#[test]
#[allow(clippy::too_many_lines)]
fn serpents_pass_enters_tapped_makes_blue_or_black_and_sells_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[serpent_s_pass()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The printed entry, and it can only be read off a real `PlayLand`.
    let pass = play_land(&mut engine, p0, serpent_s_pass());
    assert!(is_tapped(&engine, pass), "\"This land enters tapped\"");

    // A tapped land has no {T} left to pay with, so the mana line is not in the
    // offer at all — the half a test that only ever taps first would never see.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(pass, 0)),
        "{{T}} is already spent: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back: the untap step is the only thing
    // that makes the printed {{T}} payable, and it leaves the pool empty.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, pass), "the untap step stood it up");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(pass, 0)),
        "an untapped land is a paid {{T}}, so the mana line is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(pass, 1)),
        "and an empty pool pays no {{4}}, because `can_afford` reads the pool \
         and not the untapped lands: {:?}",
        legal.abilities
    );

    // Ability 0: "{T}: Add {U} or {B}". Nothing is tapped beforehand, so the
    // single mana in the pool afterwards came off the land itself.
    activate(&mut engine, p0, serpent_s_pass(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{B}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "both halves of the printed choice are offered: {options:?}"
    );
    assert_eq!(options.len(), 2, "and no third colour: {options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    assert!(
        is_tapped(&engine, pass),
        "the tap symbol was the whole price"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "one blue, off one tap");
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "`or` is one colour, and not both halves at once"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");

    // The third printed line wants that {{T}} back, which is another untap step.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, pass),
        "the second untap step stood it up again"
    );

    // Six Islands into the pool before anything is claimed, with the land named
    // as the printing kept back: it is the permanent whose {T} the last line
    // still has to pay, and `tap_all_mana_but` would otherwise spend it.
    tap_all_mana_but(&mut engine, p0, Some(serpent_s_pass()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Islands tapped, six blue, and nothing off the land"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(pass, 1)),
        "with {{4}} floating the last printed line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, serpent_s_pass(), 1);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{4}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, serpent_s_pass()).is_none(),
        "\"Sacrifice this land\" is part of the price, so the permanent left \
         the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, serpent_s_pass()).is_some(),
        "and the card is in its owner's graveyard rather than merely gone"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so an emptied library would not satisfy the count above"
    );
}
