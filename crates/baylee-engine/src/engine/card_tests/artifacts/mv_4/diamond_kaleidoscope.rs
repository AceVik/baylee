//! `cards/artifacts/mv_4/diamond_kaleidoscope.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Diamond Kaleidoscope — {4} artifact: "{3}, {T}: Create a 0/1 colorless
/// Prism artifact creature token", and "Sacrifice a Prism token: Add one mana
/// of any color."
///
/// Neither half is worth anything alone, so both are played in one main
/// phase: seven Forests pay the {4} and leave exactly the {3}, the {3} and
/// the tap build the Prism, and that Prism is then the *whole* price of a
/// mana ability. The Sol Ring standing beside it is the control that the
/// sacrifice filter is really read — it is an artifact this seat controls and
/// no Prism token, so it is on no menu and never moves.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn diamond_kaleidoscope_builds_a_prism_and_trades_it_for_one_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[diamond_kaleidoscope()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Seven Forests pay the {4} and leave exactly the {3} the token ability
    // charges. The Sol Ring is named as the printing kept back, so "seven" is
    // the Forests and the artifact is still standing as the control below.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Forests and nothing else: the Sol Ring was kept back"
    );
    cast_with_floating(&mut engine, p0, diamond_kaleidoscope());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let kaleidoscope =
        on_battlefield(&engine, p0, diamond_kaleidoscope()).expect("the Kaleidoscope resolved");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{4}} is spent and the {{3}} it charges is still floating"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(kaleidoscope, 0)),
        "with {{3}} in the pool the token line is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(kaleidoscope, 1)),
        "and the mana ability is not: it sacrifices a *Prism token* and no \
         Prism exists yet — the Sol Ring beside it is an artifact this seat \
         controls and still not one: {:?}",
        legal.abilities
    );

    // Ability 0: "{3}, {T}: Create a 0/1 colorless Prism artifact creature
    // token." Both halves of the price are read here, before it resolves.
    activate(&mut engine, p0, diamond_kaleidoscope(), 0);
    assert!(
        is_tapped(&engine, kaleidoscope),
        "{{T}} is half the price and is paid as the ability is announced"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} came out of that pool"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    let prism = tokens_of(&engine, p0);
    assert_eq!(prism.len(), 1, "one activation, one Prism");
    let prism = prism[0];
    assert_eq!(pt(&engine, prism), (0, 1), "the body the card prints");
    let kinds = types(&engine, prism);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "an artifact creature token: {kinds:?}"
    );
    assert_eq!(
        engine
            .state()
            .object(prism)
            .expect("the Prism is an object")
            .token
            .expect("it knows which token it is")
            .name,
        "Prism",
        "and it is a Prism, which is the word the sacrifice below turns on"
    );

    // Ability 1: "Sacrifice a Prism token: Add one mana of any color." The
    // cost is a sacrifice, so the engine asks which one (CR 601.2h); the
    // color is the effect's own question and arrives with the token already
    // gone.
    activate(&mut engine, p0, diamond_kaleidoscope(), 1);
    let mut menu: Option<Vec<ObjectId>> = None;
    let mut colors: Option<Vec<ManaColor>> = None;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the seat paying the cost answers it");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a cost and not a search, which is all a client has to tell apart"
                );
                assert_eq!((min, max), (1, 1), "one Prism token, no more and no fewer");
                menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![prism],
                        },
                    )
                    .expect("the Prism the question offered pays the cost");
            }
            Pending::ChooseColor { player, options } => {
                assert_eq!(
                    player, p0,
                    "and the seat that names the color is the one that paid"
                );
                colors = Some(options);
                engine
                    .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
                    .expect("black was one of the colors it offered");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected while the mana ability is paid: {other:?}"),
        }
    }

    assert_eq!(
        menu.expect("the sacrifice cost asks which permanent is being given up"),
        vec![prism],
        "the Prism token you control is the whole menu — the Sol Ring beside \
         it is an artifact and no Prism token"
    );
    let colors = colors.expect("\"add one mana of any color\" is a question");
    assert_eq!(
        colors.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {colors:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(colors.contains(&color), "\"any color\" includes {color:?}");
    }

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one Prism");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Forests' green went into the {{4}} and the {{3}}, so the black \
         has no source still standing on this board"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the token was the price: a sacrificed token ceases to exist (CR 111.7)"
    );
    assert!(
        on_battlefield(&engine, p0, diamond_kaleidoscope()).is_some(),
        "and the artifact that made it outlives it — the price was the Prism \
         and not its maker"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the filter declined never moved"
    );
    assert!(
        !is_tapped(&engine, ring),
        "and was never tapped for anything"
    );
}
