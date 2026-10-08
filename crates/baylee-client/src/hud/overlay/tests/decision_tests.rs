//! The decision sheet (the owner, 08.10.2026, item 7): its head names the
//! question and shows the question's source, it folds to a pill that leaves
//! the table to pick on, and it opens again — and the question stands
//! throughout.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;
use baylee_engine::choice::{Pending, TargetPrompt};

/// A target question of this seat's own: one creature or the other seat,
/// asked by a spell (object 90) the view names as its source.
fn targeting() -> (Duel, ObjectId) {
    let creature = baylee_client_core::test_support::token(30, 1, "Bear", 2, 2);
    let target = creature.id;
    let spell = baylee_client_core::test_support::printed(90, 0, "Giant Growth", 1);
    let source = spell.id;
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2)
        .with_battlefield(1, [creature])
        .build();
    view.stack.push(spell.clone());
    view.targeting = Some(baylee_view::TargetingContext {
        source: spell,
        text: None,
        whole_spell: true,
        second: false,
        batch_count: 1,
    });
    let mut duel = Duel {
        interaction: Some(baylee_client_core::Interaction::new(
            Pending::ChooseTargets {
                player: PlayerId::new(0),
                options: vec![target],
                player_options: vec![PlayerId::new(1)],
                min: 1,
                max: 1,
                reason: TargetPrompt::Targets,
            },
            PlayerId::new(0),
        )),
        statics: Some(baylee_client_core::test_support::statics(4)),
        ..Duel::default()
    };
    duel.view = Some(view);
    crate::rebuild_board(&mut duel);
    (duel, source)
}

fn count<C: Component>(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<(), With<C>>()
        .iter(app.world())
        .count()
}

/// The head shows the question's own source — the spell the view names —
/// as a picture that previews it.
#[test]
fn the_heads_picture_is_the_questions_source() {
    let (duel, source) = targeting();
    let mut app = bar_of(duel);
    app.update();
    let shown: Vec<ObjectId> = app
        .world_mut()
        .query::<&ledge::drawer::SheetSource>()
        .iter(app.world())
        .map(|s| s.object)
        .collect();
    assert_eq!(shown, [source], "the head shows the source, and only it");
    assert_eq!(count::<ledge::drawer::SheetTitle>(&mut app), 1);
    let previews: Vec<ObjectId> = app
        .world_mut()
        .query_filtered::<&ChoicePreview, With<ledge::drawer::SheetSource>>()
        .iter(app.world())
        .map(|p| p.object)
        .collect();
    assert_eq!(previews, [source], "the picture opens the source's preview");
}

/// Folded, the sheet is one pill at the window's edge and nothing else of
/// it stands over the table — red if the panel, its rows or its head stayed
/// up to take the press meant for a target. The question is untouched, and
/// a second fold opens it again with everything back.
#[test]
fn a_fold_leaves_the_table_to_pick_on_and_the_question_standing() {
    let (duel, _) = targeting();
    let mut app = bar_of(duel);
    app.update();
    assert!(
        count::<ChoiceButton>(&mut app) > 0,
        "the open sheet lists targets"
    );
    assert_eq!(count::<ledge::drawer::SheetPill>(&mut app), 0);

    app.world_mut().resource_mut::<Duel>().fold_decision();
    app.update();
    app.update();
    assert_eq!(
        count::<ledge::drawer::SheetPill>(&mut app),
        1,
        "folded to its pill"
    );
    assert_eq!(
        count::<ChoiceButton>(&mut app),
        0,
        "no row stands over the table"
    );
    assert_eq!(
        count::<ledge::drawer::SheetTitle>(&mut app),
        0,
        "nor the head"
    );
    // Everything the drawer's root holds is the pill: nothing else of it can
    // take a press.
    let world = app.world_mut();
    let root = world
        .query_filtered::<Entity, With<ledge::drawer::DrawerRoot>>()
        .single(world)
        .expect("the drawer's root");
    let pill = world
        .query_filtered::<Entity, With<ledge::drawer::SheetPill>>()
        .single(world)
        .expect("the pill");
    let kids: Vec<Entity> = world
        .get::<Children>(root)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    assert_eq!(kids, [pill], "the root holds the pill and nothing else");
    let duel = app.world().resource::<Duel>();
    assert!(
        matches!(
            duel.interaction
                .as_ref()
                .map(baylee_client_core::Interaction::pending),
            Some(Pending::ChooseTargets { .. })
        ),
        "folding answered nothing: the question stands"
    );
    assert!(duel.outbox().is_empty(), "and nothing was sent");

    app.world_mut().resource_mut::<Duel>().fold_decision();
    app.update();
    app.update();
    assert_eq!(
        count::<ledge::drawer::SheetPill>(&mut app),
        0,
        "opened again"
    );
    assert!(count::<ChoiceButton>(&mut app) > 0, "with its rows");
    assert_eq!(
        count::<ledge::drawer::SheetTitle>(&mut app),
        1,
        "and its head"
    );
}

/// A cast sent before its mana (CR 601.2g) asks its own target question
/// first: the sheet says the cast, the question and that the payment
/// follows in one title, and the shelf beside it says nothing of its own.
#[test]
fn a_cast_first_target_question_is_one_sentence() {
    let (mut duel, source) = targeting();
    duel.mana_run = Some(crate::ManaRun::cast_first(source).sent());
    let mut app = bar_of(duel);
    app.update();
    let titles: Vec<String> = app
        .world_mut()
        .query_filtered::<&Text, With<ledge::drawer::SheetTitle>>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert_eq!(titles.len(), 1);
    assert!(
        titles[0].contains("Giant Growth") && titles[0].contains("paid after"),
        "{titles:?}"
    );
    assert!(
        app.world()
            .resource::<ledge::LedgeRevision>()
            .prompt_for_tests()
            .is_none(),
        "the shelf says no second sentence under the sheet"
    );
}

/// A discard from hand and the mulligan's cards to the bottom are the same
/// sheet (the owner, 08.10.2026): a head with the fold, which folds to the
/// pill and leaves the question — and the hand to pick from — standing.
#[test]
fn a_discard_and_the_cards_to_the_bottom_fold_like_a_target_question() {
    for pending in [
        Pending::DiscardChoice {
            player: PlayerId::new(0),
            count: 1,
        },
        Pending::MulliganBottom {
            player: PlayerId::new(0),
            count: 1,
        },
    ] {
        let mut duel = Duel {
            interaction: Some(baylee_client_core::Interaction::new(
                pending.clone(),
                PlayerId::new(0),
            )),
            ..Duel::default()
        };
        duel.view = Some(baylee_client_core::test_support::ViewBuilder::new(2).build());
        crate::rebuild_board(&mut duel);
        let mut app = bar_of(duel);
        app.update();
        assert_eq!(
            count::<ledge::drawer::SheetTitle>(&mut app),
            1,
            "{pending:?}: a head"
        );
        let folds = app
            .world_mut()
            .query::<&MenuButton>()
            .iter(app.world())
            .filter(|b| b.action == MenuAction::FoldDecision)
            .count();
        assert_eq!(folds, 1, "{pending:?}: with its fold");
        app.world_mut().resource_mut::<Duel>().fold_decision();
        app.update();
        app.update();
        assert_eq!(
            count::<ledge::drawer::SheetPill>(&mut app),
            1,
            "{pending:?}: folded to its pill"
        );
        let duel = app.world().resource::<Duel>();
        assert!(
            duel.interaction
                .as_ref()
                .is_some_and(|i| format!("{:?}", i.pending()) == format!("{pending:?}")),
            "{pending:?}: the question stands"
        );
        assert!(duel.outbox().is_empty());
    }
}

/// The fold is the question's: a new snapshot (the game moved on) opens the
/// next sheet unfolded by itself.
#[test]
fn the_next_question_opens_unfolded() {
    let (mut duel, _) = targeting();
    duel.fold_decision();
    assert!(!ledge::drawer::sheet_up(&duel), "folded");
    assert!(crate::hud::sheet_asked(&duel), "but still asked");
    duel.view.as_mut().expect("a view").seq += 1;
    assert!(
        ledge::drawer::sheet_up(&duel),
        "a new question is not hidden"
    );
}

/// A press on the pill opens the sheet again, through the pointer's one
/// road; the shelf's sentence goes while the sheet stands and comes back
/// while it is folded.
#[test]
fn the_pill_restores_the_sheet_on_a_press() {
    let (mut duel, _) = targeting();
    duel.fold_decision();
    let mut app = crate::input::tests::pointer_app(duel);
    let pill = app
        .world_mut()
        .spawn(MenuButton {
            action: MenuAction::FoldDecision,
        })
        .id();
    let label = app.world_mut().spawn(ChildOf(pill)).id();
    crate::input::tests::click(&mut app, label);
    let duel = app.world().resource::<Duel>();
    assert!(ledge::drawer::sheet_up(duel), "the press opened the sheet");
}

/// A Cavern of Souls–style question ("as this enters, choose a creature
/// type"), offered Ally, Elf, Goblin and Zombie by the engine, from a seat
/// whose deck is mostly elves: the sheet's quick list leads with Elf and its
/// count, and a press on that chip answers the engine with Elf — through the
/// offer's own index, not the row's place.
#[test]
fn a_creature_type_is_answered_from_the_decks_quick_list() {
    use baylee_core::generated::subtypes;
    let offered: Vec<_> = ["Ally", "Elf", "Goblin", "Zombie"]
        .into_iter()
        .map(|name| subtypes::by_name(name).expect("a type"))
        .collect();
    let elf = offered[1];
    let card = |name| baylee_cards::decks::by_name(name).expect("in the pool");
    let question = || {
        let mut duel = Duel {
            interaction: Some(baylee_client_core::Interaction::new(
                Pending::ChooseSubtype {
                    player: PlayerId::new(0),
                    options: offered.clone(),
                },
                PlayerId::new(0),
            )),
            own_deck: [
                vec![card("Llanowar Elves"); 4],
                vec![card("Elvish Mystic"); 4],
                vec![card("Forest"); 10],
            ]
            .concat(),
            ..Duel::default()
        };
        duel.view = Some(baylee_client_core::test_support::ViewBuilder::new(2).build());
        crate::rebuild_board(&mut duel);
        duel
    };

    let mut app = bar_of(question());
    app.update();
    // Each chip's words, read from everything under it: a rich label is a
    // text with spans.
    let rows: Vec<(String, usize)> = {
        let world = app.world_mut();
        let chips: Vec<(Entity, usize)> = world
            .query::<(Entity, &ChoiceButton)>()
            .iter(world)
            .map(|(e, b)| (e, b.index))
            .collect();
        chips
            .into_iter()
            .map(|(chip, index)| {
                let mut said = String::new();
                let mut stack = vec![chip];
                while let Some(e) = stack.pop() {
                    let node = world.entity(e);
                    if let Some(text) = node.get::<Text>() {
                        said.push_str(&text.0);
                    }
                    if let Some(span) = node.get::<TextSpan>() {
                        said.push_str(&span.0);
                    }
                    if let Some(kids) = node.get::<Children>() {
                        stack.extend(kids.iter().rev());
                    }
                }
                (said, index)
            })
            .collect()
    };
    let quick = rows
        .iter()
        .find(|(said, _)| said.contains('·'))
        .expect("a quick chip");
    assert_eq!(
        quick.0, "Elf · 8",
        "the deck's eight elves, first: {rows:?}"
    );
    assert_eq!(quick.1, 1, "the chip answers with the offer's own index");

    let mut press = crate::input::tests::pointer_app(question());
    let chip = press
        .world_mut()
        .spawn(ChoiceButton {
            decision_id: None,
            index: quick.1,
        })
        .id();
    crate::input::tests::click(&mut press, chip);
    let sent = press.world().resource::<Duel>().outbox().to_vec();
    assert!(
        sent.iter().any(|action| matches!(
            action,
            baylee_engine::choice::PlayerAction::ChooseSubtype(chosen) if *chosen == elf
        )),
        "the press answered Elf: {sent:?}"
    );
}
