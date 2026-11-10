//! `cards/enchantments/mv_3/sacred_mesa.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sacred Mesa — {2}{W} enchantment: "{1}{W}: Create a 1/1 white Pegasus
/// creature token with flying", and "At the beginning of your upkeep, sacrifice
/// this enchantment unless you sacrifice a Pegasus."
///
/// Two copies stand on the table and exactly one Pegasus is made, so the first
/// upkeep after that reads the whole card in both directions at once:
/// `PlayerMayPayCostOr` is no yes/no — it asks through a `ChooseCards` whose menu
/// is one Pegasus and whose `min` is 0 — and the copy that pays keeps standing
/// while the copy with nothing left to give up is sacrificed. The Llanowar Elves
/// is the cost's filter: a creature you control that is no Pegasus never reaches
/// that menu, and is still standing once both triggers have resolved.
#[test]
#[allow(clippy::too_many_lines)] // two casts, one token, and the upkeep that reads the whole card
fn sacred_mesa_trades_one_pegasus_for_one_of_its_two_copies_at_the_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[sacred_mesa(), sacred_mesa()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Two casts at {2}{W} and one activation at {1}{W} are eight mana, and the
    // Elf is kept standing because it is the creature the upkeep cost's filter
    // is read against. `legal.castable` sits behind `can_afford`, which reads the
    // pool and not the untapped lands, so the empty pool is asserted first.
    let card = in_hand(&engine, p0, sacred_mesa()).expect("a Mesa is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{2}}{{W}}, so no Mesa is offered: {:?}",
        legal.castable
    );

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight tapped Plains, and the untapped Elf gave nothing"
    );
    cast_with_floating(&mut engine, p0, sacred_mesa());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, sacred_mesa());
    pass_until(&mut engine, stack_is_empty);

    let mesas = all_on_battlefield(&engine, p0, sacred_mesa());
    assert_eq!(mesas.len(), 2, "both copies resolved onto the battlefield");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two {{2}}{{W}} casts out of eight white leave exactly the {{1}}{{W}}"
    );

    // The activated half, pressed by the index the offer names rather than a
    // guessed one: a Sacred Mesa prints a triggered ability and an activated one,
    // and only the activated one is ever an `abilities` entry.
    let maker = mesas[0];
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == maker)
        .expect("{{1}}{{W}} buys a Pegasus, and its price is already in the pool");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the price the offer named is payable");
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Pegasus");
    let pegasus = tokens[0];
    let printed = engine
        .state()
        .object(pegasus)
        .expect("the Pegasus is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Pegasus");
    assert_eq!(
        (printed.power, printed.toughness),
        (Some(1), Some(1)),
        "the 1/1 body the card prints"
    );
    assert!(
        printed.colors.contains(baylee_core::color::Color::White),
        "\"a 1/1 white Pegasus\": the token is white and not merely named so"
    );
    assert!(
        printed.keywords.contains(KeywordSet::FLYING),
        "\"with flying\" reaches the token"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{W}} came out of the pool"
    );
    assert!(
        !is_tapped(&engine, maker),
        "the price is mana: nothing on the card taps the enchantment for it"
    );

    // Across the opponent's turn and into p0's next upkeep, where both copies ask
    // their question. The Pegasus pays for one; the other has nothing left to
    // give up, and that is the "unless" half of the printed sentence doing the
    // sacrificing. The loop answers whatever arrives and stops at p0's own main
    // phase, so a trigger that never fired is a board assertion and not a walk
    // that quietly gave up.
    let cast_turn = engine.state().turn.number;
    let mut asked = 0u32;
    for _ in 0..400 {
        if engine.state().turn.number > cast_turn
            && engine.state().turn.active == p0
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the cost is paid by the Mesa's controller");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell apart"
                );
                asked += 1;
                let offering = if options.contains(&pegasus) {
                    assert_eq!(
                        (min, max),
                        (0, 1),
                        "\"unless you sacrifice a Pegasus\": declining is legal"
                    );
                    assert_eq!(
                        options,
                        vec![pegasus],
                        "a creature you control is no Pegasus, and the Pegasus \
                         across the table would not be yours to give up: {options:?}"
                    );
                    vec![pegasus]
                } else {
                    Vec::new()
                };
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: offering })
                    .expect("the question offered what was answered");
            }
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
            other => panic!("unexpected on the way to the next upkeep: {other:?}"),
        }
    }

    assert!(
        asked >= 1,
        "Sacred Mesa's upkeep trigger asked the copy that had a price to pay"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, sacred_mesa()).len(),
        1,
        "the copy that paid a Pegasus is still on the battlefield"
    );
    assert_eq!(
        mine(&engine, p0, sacred_mesa(), Zone::Graveyard).len(),
        1,
        "and the copy with nothing to give up was sacrificed to its owner's graveyard"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the Pegasus was the price: a sacrificed token ceases to exist (CR 111.7)"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf was never a legal price and never moved"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and nothing on the other side of the table paid for any of it"
    );
}
