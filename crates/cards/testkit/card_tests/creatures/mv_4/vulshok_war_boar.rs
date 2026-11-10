//! `cards/creatures/mv_4/vulshok_war_boar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vulshok War Boar prints one line — "When this creature enters, sacrifice it
/// unless you sacrifice an artifact" — and that one sentence is two outcomes
/// off one board, so both are played. With a Sol Ring standing under its
/// controller the entry question offers *that* artifact and declines the same
/// card across the table (`Filter::YOUR_ARTIFACT`, CR 701.21a), and paying the
/// price leaves the printed 5/5 on the battlefield while the rock goes to its
/// owner's graveyard. With no artifact anywhere the printed "unless" has no
/// price to name at all, so the Boar sacrifices itself — the half a board that
/// always carried an artifact would read as a card that does nothing.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn vulshok_war_boar_keeps_itself_for_an_artifact_and_sacrifices_itself_without_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // Answers the Boar's own entry question. `fodder` is the artifact the
    // scenario spends, and `None` takes the printed "unless".
    let settle = |engine: &mut Engine<RegistryLookup>,
                  fodder: Option<ObjectId>,
                  theirs: Option<ObjectId>| {
        for _ in 0..12 {
            match engine.pending().clone() {
                Pending::ChooseCards {
                    player,
                    options,
                    prompt,
                    ..
                } if fodder.is_some() => {
                    assert_eq!(
                        prompt,
                        ChoicePrompt::CostSacrifice,
                        "a cost and not a search, which is all a client has to tell apart"
                    );
                    let want = fodder.expect("the guard just said there is one");
                    assert!(
                        options.contains(&want),
                        "the artifact this seat controls is the whole of the price: {options:?}"
                    );
                    if let Some(other) = theirs {
                        assert!(
                            !options.contains(&other),
                            "the same card across the table is not this seat's to spend: {options:?}"
                        );
                    }
                    engine
                        .apply(
                            player,
                            PlayerAction::ChooseObjects {
                                objects: vec![want],
                            },
                        )
                        .expect("the artifact the question offered pays the cost");
                }
                Pending::ChooseCards { .. } => {
                    // No price to name: declining is the whole answer.
                    let Pending::ChooseCards { player, .. } = engine.pending().clone() else {
                        unreachable!("the arm just matched")
                    };
                    engine
                        .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                        .expect("choosing nothing declines the cost");
                }
                Pending::YesNo { player, .. } => {
                    engine
                        .apply(player, PlayerAction::YesNo(fodder.is_some()))
                        .expect("both answers are legal while the question stands");
                }
                Pending::Priority { .. } if stack_is_empty(engine) => return,
                Pending::Priority { player, .. } => {
                    engine
                        .apply(player, PlayerAction::PassPriority)
                        .expect("passing priority is always legal");
                }
                other => panic!("unexpected while the Boar's entry resolves: {other:?}"),
            }
        }
        panic!("the Boar's entry question never settled");
    };

    // -- with an artifact to give up: the printed price is payable and asked --
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[vulshok_war_boar()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    // Four Mountains and only those: the Sol Ring is the price this scenario is
    // about, so it is kept standing rather than tapped for the {2}{R}{R}.
    assert_eq!(
        tap_mana_except(&mut engine, p0, ring),
        4,
        "four Mountains, and the Sol Ring left alone to be sacrificed"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "exactly {{2}}{{R}}{{R}} is floating"
    );
    cast_with_floating(&mut engine, p0, vulshok_war_boar());
    settle(&mut engine, Some(ring), Some(theirs));

    let boar = on_battlefield(&engine, p0, vulshok_war_boar())
        .expect("the Boar paid an artifact and stayed");
    assert_eq!(pt(&engine, boar), (5, 5), "the body the card prints");
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "\"unless you sacrifice an artifact\": the Sol Ring is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the Sol Ring across the table never moved"
    );

    // -- with no artifact at all: there is nothing to pay, so it goes itself --
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[vulshok_war_boar()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, vulshok_war_boar());
    settle(&mut engine, None, None);

    assert!(
        on_battlefield(&engine, p0, vulshok_war_boar()).is_none(),
        "\"sacrifice it unless you sacrifice an artifact\" — with no artifact on \
         the board the Boar sacrifices itself"
    );
    assert!(
        in_graveyard(&engine, p0, vulshok_war_boar()).is_some(),
        "and the card is in its owner's graveyard rather than merely gone"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        4,
        "the Mountains are no artifacts, so none of them was ever a legal price"
    );
}
