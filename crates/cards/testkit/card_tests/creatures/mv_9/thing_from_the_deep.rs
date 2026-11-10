//! `cards/creatures/mv_9/thing_from_the_deep.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thing from the Deep is a 9/9 for {6}{U}{U}{U} whose whole text is
/// "Whenever this creature attacks, sacrifice it unless you sacrifice an
/// Island." Two boards read the two halves of that "unless": with no Island
/// under the attacking seat there is no price to pay and therefore nothing to
/// ask — the walk below panics on any question rather than tolerating one — so
/// the Leviathan drowns itself before combat damage and the 9 power never
/// lands. With exactly one Island the engine asks, the answer spends the land
/// instead of the creature, and the same attack connects for nine.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn thing_from_the_deep_drowns_itself_without_an_island_and_buys_the_attack_with_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // --- half one: no Island, so nothing to pay and nothing to ask --------
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thing_from_the_deep()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let beast = on_battlefield(&engine, p0, thing_from_the_deep()).expect("the Leviathan is out");
    assert_eq!(pt(&engine, beast), (9, 9), "the body the card prints");
    assert!(
        on_battlefield(&engine, p0, island()).is_none(),
        "and there is no Island anywhere under the seat that is about to attack"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the active seat declares the attackers");
    assert!(
        attackers.contains(&beast),
        "an untapped 9/9 with no attack restriction is on the offer: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beast, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    for _ in 0..40 {
        if in_graveyard(&engine, p0, thing_from_the_deep()).is_some() {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            // "Sacrifice it unless you sacrifice an Island" is no question
            // when there is no Island: a cost with no payable price is never
            // asked, it is simply not paid. Tolerating a question here would
            // prove nothing, so this arm is the assertion.
            Pending::YesNo { player, prompt, .. } => panic!(
                "no Island, so there is no price to pay and nothing to ask: \
                 {prompt:?} to {player:?}"
            ),
            Pending::ChooseCards { prompt, .. } => {
                panic!("no Island, so no sacrifice cost may be asked for: {prompt:?}")
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
            other => panic!("unexpected on the way to the sacrifice: {other:?}"),
        }
    }

    assert!(
        on_battlefield(&engine, p0, thing_from_the_deep()).is_none(),
        "the printed sentence took the creature: attacking with no Island to \
         give up kills the attacker"
    );
    assert!(
        in_graveyard(&engine, p0, thing_from_the_deep()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "it was sacrificed in the declare-attackers step, so its 9 power never \
         dealt any combat damage"
    );

    // --- half two: exactly one Island, and exactly the price --------------
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thing_from_the_deep(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let beast = on_battlefield(&engine, p0, thing_from_the_deep()).expect("the Leviathan is out");
    let land = on_battlefield(&engine, p0, island()).expect("the Island is out");
    assert_eq!(
        lands_of(&engine, p0),
        vec![land],
        "one Island and no other land: the sacrifice has exactly one thing to ask about"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert!(
        attackers.contains(&beast),
        "the same 9/9 that drowned itself above may attack here: {attackers:?}"
    );
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beast, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    // The trigger asks before anything dies, and it asks exactly one question:
    // which Island pays. There is no yes-or-no in front of it — the list of
    // what may pay is the whole question, and naming nothing is how the seat
    // declines — so a `YesNo` here falls through to the panic below.
    // `asked` is what the assertions below turn on, and the state that
    // matters is the one they leave behind.
    let mut asked = false;
    for _ in 0..40 {
        if in_graveyard(&engine, p0, island()).is_some() {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                asked = true;
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell apart"
                );
                assert_eq!(
                    (min, max),
                    (0, 1),
                    "at most one Island, and none at all is the answer that \
                     declines and lets the 9/9 go"
                );
                assert_eq!(
                    options,
                    vec![land],
                    "the one Island this seat controls is the whole of the answer"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![land],
                        },
                    )
                    .expect("the Island the question offered pays the cost");
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
            other => panic!("unexpected while the attack trigger resolves: {other:?}"),
        }
    }

    assert!(
        asked,
        "an Island is a price the seat can really pay, so the engine asks \
         before the 9/9 dies"
    );
    assert!(
        in_graveyard(&engine, p0, island()).is_some(),
        "the answer paid the price: the Island is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, thing_from_the_deep()).is_some(),
        "and the \"unless\" was bought off, so the Leviathan is still attacking"
    );

    pass_until(&mut engine, |e| e.state().players[1].life == 11);
    assert_eq!(
        engine.state().players[1].life,
        11,
        "an unblocked 9/9 connects for nine, which is what the Island bought"
    );
    assert!(
        on_battlefield(&engine, p0, thing_from_the_deep()).is_some(),
        "and it survived its own attack: one trigger, one Island, paid once"
    );
}
