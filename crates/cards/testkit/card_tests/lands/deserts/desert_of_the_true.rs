//! `cards/lands/deserts/desert_of_the_true.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Desert of the True prints three lines — "This land enters tapped",
/// "{T}: Add {W}" and cycling {1}{W} — and one turn cycle plays all three.
/// The entry has to be a real land drop rather than a `starting_battlefield`
/// placement, because that path places a permanent with no event for
/// `EnterModifier::Tapped` to look at; the cycling is the only way to spend
/// the card out of the hand, and its {1}{W} is paid by the two Plains since
/// the land that just entered tapped has no {T} to give; and the mana line is
/// read a turn later, where the same {T} is the third white in the pool.
/// Cycling discards the card it was activated from — the two copies are one
/// printing, so only their object ids say which one went.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn desert_of_the_true_enters_tapped_cycles_for_a_card_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[desert_of_the_true(), desert_of_the_true()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, desert_of_the_true());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — and it is an entry, not the untapped \
         placement `starting_battlefield` makes"
    );

    // Cycling {1}{W}: the two Plains are the whole of the price. The Desert
    // is the card being discarded and it is tapped, so it pays neither half.
    pass_until(&mut engine, |e| at_rest(e, p0));
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        2,
        "two tapped Plains, and nothing off the land that just entered tapped"
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Ability 0 is "{T}: Add {W}", ability 1 the cycling line — and cycling is
    // offered out of the hand, which is the zone it prints.
    activate(&mut engine, p0, desert_of_the_true(), 1);
    pass_until(&mut engine, stack_is_empty);

    let cycled = in_graveyard(&engine, p0, desert_of_the_true())
        .expect("cycling discards the card it was activated from");
    assert_ne!(
        cycled, land,
        "the copy standing on the battlefield is not the one that went"
    );
    assert!(
        on_battlefield(&engine, p0, desert_of_the_true()).is_some(),
        "and the land that was played is still on the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the discarded card left and the drawn one arrived, so the hand is the \
         size it was"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{W}} was really paid out of the pool"
    );

    // The mana line, read a turn later: until the untap step the land has no
    // {T} to spend, so p0's next first main is the first board the third white
    // can exist on. Cycling left this seat a card over the hand limit, so the
    // cleanup discard on the way is answered rather than walked into.
    let turn = engine.state().turn.number;
    let mut back_at_p0 = false;
    for _ in 0..600 {
        if engine.state().turn.number >= turn + 2
            && engine.state().turn.active == p0
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            back_at_p0 = true;
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let hand = engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand.into_iter().take(usize::from(count)).collect(),
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected on the way to the next turn: {other:?}"),
        }
    }
    assert!(back_at_p0, "the game walks a full turn cycle back to p0");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        3,
        "two Plains and the Desert: \"{{T}}: Add {{W}}\" is back after its \
         controller's untap step, which is exactly what the printed entry \
         denied it for a turn"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three sources tapped and three white in the pool, nothing else"
    );
}
