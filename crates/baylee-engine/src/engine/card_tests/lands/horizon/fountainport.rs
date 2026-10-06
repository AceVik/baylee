//! `cards/lands/horizon/fountainport.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fountainport: "{2}, {T}, Sacrifice a **token**: Draw a card."
///
/// The thirteenth of the family and the only one whose price is a filter
/// over what a permanent *is* rather than what it is called: `Filter::IsToken`
/// is true of no card in any decklist, so the land has to make its own
/// payment first. Which is also why it takes two turns — one `{T}` per turn,
/// and this card charges one for the Treasure and one for the draw.
#[test]
fn a_land_that_sacrifices_a_token_makes_one_first() {
    let p0 = PlayerId::new(0);
    let port = card_index("94e8b0a9-44a1-4dce-8d44-78681ae638a1");
    let mut engine = Duel::new(931, forest())
        .battlefield(
            0,
            &[
                port,
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Turn one: `{4}, {T}: Create a Treasure token`.
    let treasure_maker = baylee_cards::by_index(port)
        .expect("the card is in the pool")
        .abilities
        .iter()
        .position(|a| {
            format!("{a:?}").contains("CreateToken") && !format!("{a:?}").contains("LoseLife")
        })
        .expect("the Treasure ability");
    tap_all_mana_but(&mut engine, p0, Some(port));
    let source = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == port)
        })
        .expect("Fountainport is on the battlefield");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: u32::try_from(treasure_maker).expect("a small index"),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .any(|id| e.state().object(*id).is_some_and(|o| o.card.is_none()))
    });

    // Turn two: the token is on the board, so the sacrifice has an answer.
    cross_into_the_next_own_main(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(port));
    let (asks, index) = ability_that_asks(&engine, port).expect("the sacrifice is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: asks,
                ability_index: index,
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the cost question, got {:?}", engine.pending())
    };
    assert_eq!(
        options.len(),
        1,
        "the Treasure is a token and nothing else on this board is"
    );
    assert!(
        engine
            .state()
            .object(options[0])
            .is_some_and(|o| o.card.is_none()),
        "what the menu offers is the token"
    );
}

/// Fountainport: "{2}, {T}, Sacrifice a token: Draw a card." Elspeth, Storm
/// Slayer's +1 makes the tokens (two, by her own doubling); one is sacrificed
/// to the land and a card is drawn, the other stays.
#[test]
fn fountainport_sacrifices_a_token_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let elspeth = card_index("f78af825-023a-42e9-8374-5c52303a1417");
    let port = card_index("94e8b0a9-44a1-4dce-8d44-78681ae638a1");
    let mut engine = Duel::new(2307, forest())
        .battlefield(0, &[elspeth, port, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, elspeth, 1);
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 2, "+1 makes a Soldier, doubled");
    let p = on_battlefield(&engine, p0, port).expect("fountainport");
    tap_mana_where(&mut engine, p0, |id| id != p);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, port, 1);
    unf_aim_and_pay(&mut engine, p0, None, Some(tokens[0]));

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1
    );
    assert_eq!(tokens_of(&engine, p0), vec![tokens[1]], "one token is left");
}
