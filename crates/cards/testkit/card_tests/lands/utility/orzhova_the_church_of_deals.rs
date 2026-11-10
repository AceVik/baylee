//! `cards/lands/utility/orzhova_the_church_of_deals.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Orzhova, the Church of Deals is a land printing two lines: the plain
/// `{T}: Add {C}`, and a five-mana sink — `{3}{W}{B}, {T}: Target player
/// loses 1 life and you gain 1 life` — whose target is a seat and not a
/// permanent.
///
/// Both are played in one game. The mana ability is read in p0's first main
/// on an empty pool, where the single colourless can only be the land's own
/// and where a nonbasic printing its own `{T}` has to show up as an ordinary
/// indexed ability rather than the CR 305.6 shortcut. A turn later, with the
/// Church standing back up and the pool empty again, the sink is read off
/// exactly the five mana three Plains and two Swamps make: it is absent from
/// the offer until they are tapped, because `can_afford` reads the pool, the
/// target question names either seat, and the life moves on the seat that
/// was named while the activating seat takes its own point back.
#[test]
#[allow(clippy::too_many_lines)]
fn orzhova_taps_for_colorless_and_later_drains_the_player_it_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                orzhova_the_church_of_deals(),
                plains(),
                plains(),
                plains(),
                swamp(),
                swamp(),
            ],
        )
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let church = on_battlefield(&engine, p0, orzhova_the_church_of_deals())
        .expect("the Church is on the battlefield");
    assert!(
        types(&engine, church).contains(TypeSet::LAND),
        "it is the land the card prints"
    );
    assert!(!is_tapped(&engine, church), "and it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats: the board was seated rather than paid for"
    );

    // Ability 0 is the printed "{T}: Add {C}".
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(church, 0)),
        "a nonbasic land's own {{T}} is an ordinary indexed ability and never \
         the CR 305.6 shortcut: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, orzhova_the_church_of_deals(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, church), "the land paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\" — and the Church is the only permanent with a \
         mana ability on this board"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");

    // A turn round the table: the sink below needs the {T} the mana ability
    // just spent, and the untap step is the only thing that gives it back.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Church's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, church),
        "the untap step stood the land back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // `can_afford` reads the pool and not the untapped lands, so with nothing
    // floating the sink is not offered at all — the half a test that only
    // ever taps first would never see.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(church, 1)),
        "{{3}}{{W}}{{B}} is not five untapped lands: on an empty pool the cost \
         is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    // Three Plains and two Swamps are exactly {{3}}{{W}}{{B}}, and the Church
    // is the printing kept back because its own {{T}} is half of the price
    // being claimed.
    tap_all_mana_but(&mut engine, p0, Some(orzhova_the_church_of_deals()));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 3, "three tapped Plains");
    assert_eq!(pool.available(ManaColor::Black), 2, "and two tapped Swamps");
    assert_eq!(pool.total(), 5, "five mana, and nothing else on the board");
    assert!(
        !is_tapped(&engine, church),
        "the Church was the printing kept back from the tapping"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(church, 1)),
        "with the five floating the whole price is payable: {:?}",
        legal.abilities
    );

    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (20, 20),
        "two untouched life totals, so the swing below has a direction"
    );

    activate(&mut engine, p0, orzhova_the_church_of_deals(), 1);

    // "Target player" is a seat rather than a permanent, which the engine may
    // publish either as its own question or as a target choice carrying
    // `player_options` (CR 115.4). Both name the same two seats, so the one
    // that arrives is read and answered rather than the one that was expected.
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the activating seat names the target");
            assert!(
                options.contains(&p0) && options.contains(&p1),
                "\"target player\" is either seat, the activating one included: {options:?}"
            );
            engine
                .apply(p0, PlayerAction::ChoosePlayer(p1))
                .expect("the opponent was one of the seats it enumerated");
        }
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the activating seat names the target");
            assert!(
                player_options.contains(&p0) && player_options.contains(&p1),
                "\"target player\" is either seat, the activating one included: \
                 {player_options:?}"
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("the opponent was one of the seats it enumerated");
        }
        other => panic!("\"target player\" is a target question, got {other:?}"),
    }

    // CR 601.2c before CR 601.2h: the target is answered first, so the {{T}}
    // and the five mana go together with that answer and not before it.
    assert!(
        is_tapped(&engine, church),
        "{{T}} is half the price, paid by the land itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}}{{W}}{{B}} came out of the pool the three Plains and two \
         Swamps filled"
    );
    assert!(
        !stack_is_empty(&engine),
        "draining a player is no mana ability, so the ability is on the stack"
    );
    assert_eq!(
        (
            engine.state().players[0].life,
            engine.state().players[1].life
        ),
        (20, 20),
        "and nothing has moved yet: the life is the resolution, not the cost"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"Target player loses 1 life\" — the seat that was named, and one \
         point of it"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"and you gain 1 life\" — the seat that paid the price, and not the \
         one that paid it"
    );
    assert!(
        on_battlefield(&engine, p0, orzhova_the_church_of_deals()).is_some(),
        "an activated ability costs the land nothing but its tap"
    );
}
