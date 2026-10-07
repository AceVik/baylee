//! `cards/lands/tournament_grounds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tournament Grounds prints two mana abilities and no other text: `{T}: Add
/// {C}`, and `{T}: Add {R}, {W}, or {B}. Spend this mana only to cast a Knight
/// or Equipment spell.` Both prices are the land's own tap, so an untapped
/// ground on an empty pool offers both lines at once — the only board on which
/// the two can be told apart at the offer rather than by what they later
/// produce. The first is one fixed colourless with nothing asked; the second is
/// a question exactly three colours wide (CR 105.4: colourless is no colour),
/// and the mana it makes lands in the *restricted* pool, which is precisely
/// what `available` does not read.
#[test]
fn tournament_grounds_offers_colorless_and_restricted_knight_mana_off_the_same_tap_symbol() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[tournament_grounds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land arrives the way a land arrives: played for real, so a card that
    // carried an enters-tapped line could not pass this by accident.
    let grounds = play_land(&mut engine, p0, tournament_grounds());
    assert!(
        !is_tapped(&engine, grounds),
        "a land with no enters-tapped line is standing when it is played"
    );

    // `legal.abilities` is filtered through `can_afford`, and neither printed
    // price asks for mana — both cost their own {T} and nothing else.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(grounds, 0)),
        "{{T}}: Add {{C}} is offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(grounds, 1)),
        "and {{T}}: Add {{R}}, {{W}}, or {{B}} beside it: {:?}",
        legal.abilities
    );

    // Ability 0. A mana ability a card prints has an index to name, so it is an
    // ordinary entry in `abilities` and not the CR 305.6 shortcut, and `{C}` is
    // fixed, so nothing is asked on the way (CR 605.3b).
    activate(&mut engine, p0, tournament_grounds(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{C}}` is not a choice, so nothing is asked, got {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, grounds), "the tap was the whole price");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");

    // Across the opponent's turn and back: both printed lines cost the same
    // {T}, so the second one needs its own untap step.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, grounds),
        "the untap step stood the grounds back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // Ability 1: "{T}: Add {R}, {W}, or {B}."
    activate(&mut engine, p0, tournament_grounds(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`{{R}}, {{W}}, or {{B}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::Red, ManaColor::White, ManaColor::Black],
        "the three colours the card prints, and no fourth"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    assert!(is_tapped(&engine, grounds), "the tap was the whole price");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    let mut restricted_black = 0u32;
    for entry in pool.restricted() {
        if entry.color == ManaColor::Black {
            restricted_black += u32::from(entry.amount);
        }
    }
    assert_eq!(
        restricted_black, 1,
        "\"Spend this mana only to cast a Knight or Equipment spell\": the \
         black landed in the restricted pool, which is the half `available` \
         does not read"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "and the simple pool is empty, so a test that read only `available` \
         would describe a land that produced no mana at all"
    );
}
