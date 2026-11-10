//! `cards/lands/manlands/restless_reef.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Reef prints four sentences: it enters tapped; it taps for {U} or
/// {B}; `{2}{U}{B}` turns it into a 4/4 blue and black Shark with deathtouch
/// for the turn while it stays a land; and whenever it attacks, a targeted
/// player mills four. One game reads all four, and the order is forced by the
/// rules: the land is *played* rather than seeded, so the entry tap is a real
/// replacement and the turn it arrives is spent on the mana line, and the
/// animation waits for the next turn because the permanent has to be standing
/// to attack — and only the untap step can say whether it is still the land
/// that came in.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn restless_reef_enters_tapped_taps_for_blue_or_black_and_attacks_as_a_deathtouch_shark() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(7711, forest())
        .battlefield(0, &[island(), island(), swamp(), swamp()])
        .hand(0, &[restless_reef()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real land drop and not `starting_battlefield`: an entry replacement
    // only runs for a card that enters, and a seeded permanent never looks at
    // `EnterModifier::Tapped`.
    let reef = play_land(&mut engine, p0, restless_reef());
    assert!(entered_tapped(&engine, reef), "\"This land enters tapped\"");

    // Around to p0's next main phase, where the Reef is standing again.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, reef), "the untap step stood it up");

    // The printed tap of a land that is no basic type is an ordinary
    // `(source, index)` entry, and it prints two colours, so it asks.
    activate(&mut engine, p0, restless_reef(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{B}}\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "both halves of the printed line: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "blue or black and nothing else: {options:?}"
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
    assert_eq!(pool.total(), 1, "one land, one mana");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, reef), "the Reef paid its own {{T}}");

    // Around again, and this time the four basics pay for the animation while
    // the Reef is kept back — it has to be able to attack afterwards.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        tap_mana_except(&mut engine, p0, reef),
        4,
        "two Islands and two Swamps; the Reef is the one source this test still needs standing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "exactly the {{2}}{{U}}{{B}} the animation charges"
    );

    // Ability 1 is the animation; ability 0 is the mana line above, 2 the
    // attack trigger, which is never offered as an activation.
    activate(&mut engine, p0, restless_reef(), 1);
    assert!(
        !stack_is_empty(&engine),
        "an animation is no mana ability, so it uses the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    let kinds = types(&engine, reef);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "\"this land becomes a 4/4 blue and black Shark creature\": {kinds:?}"
    );
    assert!(
        kinds.contains(TypeSet::LAND),
        "\"It's still a land\" — a land *and* a creature, not one instead of the other"
    );
    assert_eq!(pt(&engine, reef), (4, 4), "the body the animation prints");
    assert!(
        keywords(&engine, reef).contains(KeywordSet::DEATHTOUCH),
        "with deathtouch"
    );
    assert!(
        !is_tapped(&engine, reef),
        "nothing in the animation taps it, which is what lets it attack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and {{2}}{{U}}{{B}} came out of the pool rather than being a label"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&reef),
        "a 4/4 that has been on the table since turn one may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(reef, Defender::Player(p1))],
            },
        )
        .expect("the offer named it as an attacker");

    // The printed trigger aims at a player, and the two readings below are
    // measured against the board the claim was made on.
    let library_before = library_size(&engine, p1);
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();
    let mut aimed_at_p1 = false;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert!(
                    player_options.contains(&p1),
                    "\"target player\" reaches across the table: {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: Vec::new(),
                            players: vec![p1],
                        },
                    )
                    .unwrap();
                aimed_at_p1 = true;
            }
            Pending::ChoosePlayer { player, options } => {
                assert!(options.contains(&p1), "both seats are legal: {options:?}");
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .unwrap();
                aimed_at_p1 = true;
            }
            Pending::Priority { .. } if stack_is_empty(&engine) => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the attack trigger resolves: {other:?}"),
        }
    }
    assert!(
        aimed_at_p1,
        "\"target player mills four cards\" cannot resolve without asking which player"
    );

    // The end step and not `stack_is_empty`: combat damage is dealt in the
    // damage step (CR 510.2), which an empty stack is already true before.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        16,
        "an unblocked 4/4, and no blockers were declared"
    );
    assert_eq!(
        library_size(&engine, p1),
        library_before - 4,
        "\"Whenever this land attacks, target player mills four cards\""
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        graveyard_before + 4,
        "the four cards are in that player's graveyard and not merely gone from the library"
    );
}
