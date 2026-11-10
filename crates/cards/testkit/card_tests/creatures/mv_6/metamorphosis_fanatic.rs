//! `cards/creatures/mv_6/metamorphosis_fanatic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Metamorphosis Fanatic — {4}{B}{B}, 4/4 lifelink — "when this creature
/// enters, return up to one target creature card from your graveyard to the
/// battlefield with a lifelink counter on it."
///
/// The counter is the half that is easy to lose, because it is a second
/// effect in the same list and it acts on the card the first one moved: a
/// reader that put it on before the move would have it wiped by
/// `move_object`, and one that read the wrong object would put it on the
/// Fanatic. So the assertion is the counter on the *returned* creature, and
/// the Fanatic's own lifelink is printed rather than countered, which is what
/// tells the two apart.
#[test]
fn metamorphosis_fanatic_returns_a_creature_wearing_a_lifelink_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(43, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                rib_cage_spider(),
            ],
        )
        .hand(0, &[metamorphosis_fanatic()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let spider = on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider is seated");
    bury(&mut engine, &[spider]);

    cast_from_hand(&mut engine, p0, metamorphosis_fanatic());
    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&spider),
        "the one creature card in the graveyard: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![spider],
                players: vec![],
            },
        )
        .expect("the Spider was on the menu");
    pass_until(&mut engine, stack_is_empty);

    let back = on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider came back");
    assert_eq!(
        counters_on(&engine, back, baylee_cards_dsl::CounterKind::Lifelink),
        1,
        "\"with a lifelink counter on it\" — on the creature that returned"
    );
    let fanatic =
        on_battlefield(&engine, p0, metamorphosis_fanatic()).expect("the Fanatic resolved");
    assert_eq!(
        counters_on(&engine, fanatic, baylee_cards_dsl::CounterKind::Lifelink),
        0,
        "and not on the Fanatic, whose lifelink is printed on it"
    );
}

/// Metamorphosis Fanatic's miracle over an untapped Sol Ring and Swamp, with
/// nothing floating: "yes" opens a window to make its {1}{B}.
///
/// By the rules this {1}{B} can be paid, because mana abilities may be
/// activated while a spell's costs are paid (CR 601.2g). Previously this
/// test pinned a cast that quietly failed because only floating mana counted.
#[test]
fn metamorphosis_fanatics_miracle_can_tap_its_sources_to_pay() {
    let mut engine = Duel::new(41, metamorphosis_fanatic())
        .battlefield(0, &[sol_ring(), swamp()])
        .battlefield(1, &[sol_ring(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    for _ in 0..200 {
        match engine.pending().clone() {
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::Miracle { card },
                ..
            } => {
                engine
                    .apply(player, PlayerAction::YesNo(true))
                    .expect("an offered yes is an answer");
                assert_eq!(
                    engine.payment_window(),
                    Some((
                        player,
                        baylee_core::mana::ManaPayment::Fixed("{1}{B}".parse().unwrap())
                    ))
                );
                tap_all_mana(&mut engine, player);
                engine.apply(player, PlayerAction::PassPriority).unwrap();
                assert_eq!(engine.state().object(card).unwrap().zone, Zone::Stack);
                assert_eq!(
                    engine.state().players[player.get() as usize]
                        .mana_pool
                        .total(),
                    1
                );
                return;
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
            other => panic!("unexpected question: {other:?}"),
        }
    }
    panic!("no miracle was offered");
}
