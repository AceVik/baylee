//! The rows of answers: payment, damage, sources, retargeting.

#[allow(clippy::wildcard_imports)] // the tests' shared fixtures
use super::*;

#[test]
fn temporary_payment_presenter_fits_at_960_and_1280_with_every_control() {
    use baylee_core::ids::{DamageSourceRef, GrantedActionId};
    use baylee_engine::choice::{GrantedActionKind, GrantedActionOffer, LegalActions, Pending};
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.owed = Some(baylee_core::mana::ManaPayment::Fixed(
        baylee_core::mana::ManaCost::parse("{1}{R}"),
    ));
    let mut duel = Duel::default();
    duel.receive_view(view);
    duel.receive_choice(Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(LegalActions {
            can_pass: true,
            granted_actions: vec![GrantedActionOffer {
                id: GrantedActionId::new(4),
                source: DamageSourceRef {
                    object: ObjectId::new(3, 0),
                    version: 2,
                },
                ability: None,
                timing: baylee_cards_dsl::SpecialActionTiming::ManaAbility,
                cost: baylee_cards_dsl::SpecialActionCost::Life(1),
                effect: GrantedActionKind::AddMana {
                    color: baylee_core::mana::ManaColor::Colorless,
                    amount: 1,
                },
            }],
            ..LegalActions::default()
        }),
    });
    let texts = crate::cardtext::CardTexts::default();
    let prefs = crate::prefs::Prefs::default();
    for lang in [Lang::De, Lang::En] {
        let sentence = shelf_headline(&duel, lang, &texts).unwrap();
        assert_eq!(sentence, Phrase::GrantedPaymentHint.text(lang));
        let answers = answers_for(&duel, lang, false, false, false);
        assert!(
            answers
                .iter()
                .any(|(says, _)| *says == Says::Answer(PromptAction::Confirm))
        );
        assert!(answers.iter().any(
            |(says, _)| *says == Says::Command(super::super::MenuAction::ToggleGrantedActions)
        ));
        // A payment window is paid or declined; skipping the turn is no
        // answer to it (no standing order passes one).
        assert!(
            answers
                .iter()
                .any(|(says, _)| *says == Says::Command(super::super::MenuAction::DeclinePayment))
        );
        let caps = keys_for(&prefs, &answers, false, false);
        let caps_w = caps.iter().flatten().map(|c| cap_width(c) + CAP_GAP).sum();
        let mid = mid_width(Some(&sentence), Clock::None, &answers, &caps);
        for width in [960, 1280] {
            let layout = baylee_client_core::ledge::arrange(
                width as f32,
                baylee_client_core::ledge::Columns {
                    left: tools_reserved(width),
                    mid,
                    right: RIGHT_RESERVED,
                },
                caps_w,
            );
            assert_ne!(
                layout.density,
                baylee_client_core::ledge::Density::Split,
                "{lang:?} at {width}"
            );
        }
    }
}

#[test]
fn damage_row_selection_rebuilds_confirm_without_a_new_board_snapshot() {
    use baylee_engine::choice::{DamageChoiceId, DamageEffectKind, DamageEffectOption, Pending};
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    let me = view.seat;
    let offer = |step| Pending::ChooseDamageEffect {
        player: me,
        choice: DamageChoiceId { batch: 7, step },
        damage: vec![],
        options: vec![DamageEffectOption {
            id: 91,
            source: None,
            ability: None,
            controller: me,
            kind: DamageEffectKind::Protection,
            parts: vec![],
        }],
    };
    let mut app = App::new();
    app.insert_resource(Duel {
        view: Some(view),
        interaction: Some(baylee_client_core::Interaction::new(offer(1), me)),
        ..Duel::default()
    });
    app.insert_resource(UiFonts {
        text: Handle::default(),
        medium: Handle::default(),
        bold: Handle::default(),
        italic: Handle::default(),
        medium_italic: Handle::default(),
        serif: Handle::default(),
        serif_italic: Handle::default(),
        icons: Handle::default(),
        mana: Handle::default(),
    });
    app.init_resource::<LedgeRevision>()
        .init_resource::<LedgeLayout>()
        .init_resource::<crate::settings::ClientSettings>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::cardtext::CardTexts>()
        .add_systems(Update, sync_ledge);
    app.world_mut().spawn((LedgeShelf, Node::default()));
    app.update();
    assert!(damage_confirm_ids(&mut app).is_empty());
    app.world_mut()
        .resource_mut::<Duel>()
        .interaction
        .as_mut()
        .unwrap()
        .choose_index(0);
    app.update();
    assert_eq!(
        damage_confirm_ids(&mut app),
        vec![Some(baylee_client_core::interaction::DecisionId::Damage(
            DamageChoiceId { batch: 7, step: 1 }
        ))]
    );
    app.world_mut().resource_mut::<Duel>().interaction =
        Some(baylee_client_core::Interaction::new(offer(2), me));
    app.update();
    assert!(damage_confirm_ids(&mut app).is_empty());
    app.world_mut()
        .resource_mut::<Duel>()
        .interaction
        .as_mut()
        .unwrap()
        .choose_index(0);
    app.update();
    assert_eq!(
        damage_confirm_ids(&mut app),
        vec![Some(baylee_client_core::interaction::DecisionId::Damage(
            DamageChoiceId { batch: 7, step: 2 }
        ))]
    );
}

#[test]
fn source_row_selection_rebuilds_confirm_without_a_new_board_snapshot() {
    use baylee_core::ids::{DamageSourceRef, ObjectId, SourceChoiceId};
    use baylee_engine::choice::Pending;
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    let me = view.seat;
    let offer = |step| Pending::ChooseDamageSource {
        player: me,
        choice: SourceChoiceId::new(step),
        options: vec![DamageSourceRef {
            object: ObjectId::new(9, 0),
            version: 1,
        }],
    };
    let mut app = App::new();
    app.insert_resource(Duel {
        view: Some(view),
        interaction: Some(baylee_client_core::Interaction::new(offer(1), me)),
        ..Duel::default()
    });
    app.insert_resource(UiFonts {
        text: Handle::default(),
        medium: Handle::default(),
        bold: Handle::default(),
        italic: Handle::default(),
        medium_italic: Handle::default(),
        serif: Handle::default(),
        serif_italic: Handle::default(),
        icons: Handle::default(),
        mana: Handle::default(),
    });
    app.init_resource::<LedgeRevision>()
        .init_resource::<LedgeLayout>()
        .init_resource::<crate::settings::ClientSettings>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::cardtext::CardTexts>()
        .add_systems(Update, sync_ledge);
    app.world_mut().spawn((LedgeShelf, Node::default()));
    app.update();
    assert!(damage_confirm_ids(&mut app).is_empty());
    app.world_mut()
        .resource_mut::<Duel>()
        .interaction
        .as_mut()
        .unwrap()
        .choose_index(0);
    app.update();
    assert_eq!(
        damage_confirm_ids(&mut app),
        vec![Some(baylee_client_core::interaction::DecisionId::Source(
            SourceChoiceId::new(1)
        ))]
    );
    app.world_mut().resource_mut::<Duel>().interaction =
        Some(baylee_client_core::Interaction::new(offer(2), me));
    app.update();
    assert!(damage_confirm_ids(&mut app).is_empty());
    app.world_mut()
        .resource_mut::<Duel>()
        .interaction
        .as_mut()
        .unwrap()
        .choose_index(0);
    app.update();
    assert_eq!(
        damage_confirm_ids(&mut app),
        vec![Some(baylee_client_core::interaction::DecisionId::Source(
            SourceChoiceId::new(2)
        ))]
    );
}

#[test]
fn retarget_confirmation_rebuilds_when_only_the_original_version_changes() {
    use baylee_core::ids::{DamageSourceRef, ObjectId, TargetRef};
    use baylee_engine::choice::{Pending, TargetPrompt};
    let view = baylee_client_core::test_support::ViewBuilder::new(2)
        .with_battlefield(
            0,
            [baylee_client_core::test_support::token(9, 0, "Bear", 2, 2)],
        )
        .build();
    let me = view.seat;
    let offer = |version| Pending::ChooseTargets {
        player: me,
        options: vec![ObjectId::new(9, 0)],
        player_options: vec![],
        min: 0,
        max: 1,
        reason: TargetPrompt::Retarget {
            current: TargetRef::Object(DamageSourceRef {
                object: ObjectId::new(9, 0),
                version,
            }),
            index: 0,
            of: 1,
        },
    };
    let mut app = App::new();
    app.insert_resource(Duel {
        view: Some(view),
        interaction: Some(baylee_client_core::Interaction::new(offer(1), me)),
        ..Duel::default()
    });
    app.insert_resource(UiFonts {
        text: Handle::default(),
        medium: Handle::default(),
        bold: Handle::default(),
        italic: Handle::default(),
        medium_italic: Handle::default(),
        serif: Handle::default(),
        serif_italic: Handle::default(),
        icons: Handle::default(),
        mana: Handle::default(),
    });
    app.init_resource::<LedgeRevision>()
        .init_resource::<LedgeLayout>()
        .init_resource::<crate::settings::ClientSettings>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::cardtext::CardTexts>()
        .add_systems(Update, sync_ledge);
    app.world_mut().spawn((LedgeShelf, Node::default()));
    app.update();
    let duel = app.world().resource::<Duel>();
    let text = shelf_headline(duel, Lang::De, &crate::cardtext::CardTexts::default()).unwrap();
    assert_eq!(
        text,
        Phrase::RetargetKeepOrChange.fill(Lang::De, &["1", "1"])
    );
    let answers = answers_for(duel, Lang::De, false, false, false);
    let caps = keys_for(
        app.world().resource::<crate::prefs::Prefs>(),
        &answers,
        false,
        false,
    );
    let width = mid_width(Some(&text), Clock::None, &answers, &caps);
    assert!(
        width + tools_reserved(960) + RIGHT_RESERVED < 960.0,
        "Retarget question and confirmation must fit beside both toolbars: {width}"
    );
    let old = damage_confirm_ids(&mut app);
    assert_eq!(old.len(), 1);
    app.world_mut()
        .resource_mut::<Duel>()
        .receive_choice(offer(3));
    app.update();
    let new = damage_confirm_ids(&mut app);
    assert_eq!(new.len(), 1);
    assert_ne!(old, new);
    assert_eq!(
        new[0],
        app.world()
            .resource::<Duel>()
            .interaction
            .as_ref()
            .unwrap()
            .decision_id()
    );
}

fn damage_confirm_ids(app: &mut App) -> Vec<Option<baylee_client_core::interaction::DecisionId>> {
    let mut query = app.world_mut().query::<&PromptButton>();
    query
        .iter(app.world())
        .filter(|button| button.action == PromptAction::Confirm)
        .map(|button| button.decision_id)
        .collect()
}

#[test]
fn retarget_confirmation_names_keeping_or_changing_the_target() {
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_engine::choice::{Pending, TargetPrompt};
    let me = PlayerId::new(0);
    let target = ObjectId::new(1, 0);
    let mut duel = Duel {
        interaction: Some(baylee_client_core::Interaction::new(
            Pending::ChooseTargets {
                player: me,
                options: vec![target],
                player_options: vec![PlayerId::new(1)],
                min: 0,
                max: 1,
                reason: TargetPrompt::Retarget {
                    current: baylee_engine::choice::TargetRef::Player(me),
                    index: 0,
                    of: 1,
                },
            },
            me,
        )),
        ..Duel::default()
    };
    for lang in [Lang::En, Lang::De] {
        assert_eq!(
            confirmation_row(&duel, lang)[0].1,
            Phrase::KeepCurrentTarget.text(lang)
        );
    }
    duel.interaction.as_mut().unwrap().toggle(target);
    assert_eq!(confirmation_row(&duel, Lang::En)[0].1, "Change target");
    duel.interaction.as_mut().unwrap().toggle(target);
    duel.interaction
        .as_mut()
        .unwrap()
        .toggle_player(PlayerId::new(1));
    assert_eq!(confirmation_row(&duel, Lang::De)[0].1, "Ziel ändern");
}

#[test]
fn payment_questions_draw_symbols_in_the_mana_font_and_keep_asides_quiet() {
    let mut app = App::new();
    let mut assets = Assets::<Font>::default();
    let mana = assets.add(Font::from_bytes(
        include_bytes!("../../../../assets/fonts/mana.ttf").to_vec(),
    ));
    let fonts = UiFonts {
        text: Handle::default(),
        medium: Handle::default(),
        bold: Handle::default(),
        italic: Handle::default(),
        medium_italic: Handle::default(),
        serif: Handle::default(),
        serif_italic: Handle::default(),
        icons: Handle::default(),
        mana: mana.clone(),
    };
    sentence(
        &mut app.world_mut().commands(),
        &fonts,
        "Pay {X}{B} ({T}: add {B})?",
        SENTENCE_PT,
        palette::DIALOG_INK,
    );
    app.world_mut().flush();
    let mut text = app.world_mut().query::<(&Text, &TextFont, &TextColor)>();
    let mut marks = Vec::new();
    let mut aside = false;
    for (words, font, color) in text.iter(app.world()) {
        assert!(
            !words.0.contains('{'),
            "printed symbols cannot remain prose"
        );
        if font.font == bevy::text::FontSource::Handle(mana.clone()) {
            marks.push(words.0.clone());
        }
        if words.0.contains("add") {
            aside = true;
            assert_eq!(color.0, palette::LEDGE_SOFT);
        }
    }
    assert_eq!(
        marks.len(),
        4,
        "X, black, tap and black are all font glyphs"
    );
    assert!(aside, "the parenthesized payment hint remains visible");
}

/// A copy's new-target question, `index` of `of`, asked of seat 0 on a view.
fn retarget_duel(min: u32, index: u32, of: u32) -> Duel {
    use baylee_core::ids::{DamageSourceRef, TargetRef};
    use baylee_engine::choice::{Pending, TargetPrompt};
    let view = baylee_client_core::test_support::ViewBuilder::new(2)
        .with_battlefield(
            0,
            [baylee_client_core::test_support::token(9, 0, "Bear", 2, 2)],
        )
        .build();
    let me = view.seat;
    let pending = Pending::ChooseTargets {
        player: me,
        options: vec![ObjectId::new(9, 0)],
        player_options: vec![],
        min,
        max: 1,
        reason: TargetPrompt::Retarget {
            current: TargetRef::Object(DamageSourceRef {
                object: ObjectId::new(9, 0),
                version: 1,
            }),
            index,
            of,
        },
    };
    Duel {
        view: Some(view),
        interaction: Some(baylee_client_core::Interaction::new(pending, me)),
        ..Duel::default()
    }
}

/// The shelf names which of how many targets is asked, and says whether it
/// may be kept: "keep it, or choose a new one" where the engine's `min` is 0,
/// "choose a new one" where it is above 0 (report 10). Red on the old fixed
/// "Choose a new target" for both.
#[test]
fn a_copys_target_question_says_which_of_how_many_and_whether_it_may_be_kept() {
    let texts = crate::cardtext::CardTexts::default();
    for lang in [Lang::En, Lang::De] {
        let may_keep = shelf_headline(&retarget_duel(0, 2, 10), lang, &texts).unwrap();
        assert_eq!(
            may_keep,
            Phrase::RetargetKeepOrChange.fill(lang, &["3", "10"]),
            "{lang:?}: a keepable target says 3 of 10 and offers keeping"
        );
        let must = shelf_headline(&retarget_duel(1, 2, 10), lang, &texts).unwrap();
        assert_eq!(
            must,
            Phrase::RetargetMustChange.fill(lang, &["3", "10"]),
            "{lang:?}: a target that may not be kept says only to choose a new one"
        );
        assert_ne!(
            may_keep, must,
            "{lang:?}: the two questions read differently"
        );
    }
    assert_eq!(
        shelf_headline(&retarget_duel(0, 2, 10), Lang::En, &texts).unwrap(),
        "Target 3 of 10: keep it, or choose a new one"
    );
}

/// "Keep all remaining (n)" stands beside the confirmation exactly where
/// `KeepTargets::offered` says: a keepable question with more after it. It
/// counts this one and the rest. Red on the old row, which never had it.
#[test]
fn keep_all_remaining_stands_beside_the_confirmation_only_where_more_follow() {
    let keep = Says::Answer(PromptAction::KeepTargets);
    let has = |duel: &Duel| {
        confirmation_row(duel, Lang::En)
            .into_iter()
            .find(|(says, _)| *says == keep)
    };
    let duel = retarget_duel(0, 0, 3);
    assert_eq!(
        has(&duel),
        Some((keep, "Keep all remaining (3)".to_string())),
        "the first of three offers keeping all three"
    );
    assert_eq!(
        has(&retarget_duel(0, 1, 3)).map(|(_, text)| text),
        Some("Keep all remaining (2)".to_string()),
        "the middle one counts itself and the last"
    );
    assert!(
        has(&retarget_duel(0, 2, 3)).is_none(),
        "the last target has nothing after it to keep"
    );
    assert!(
        has(&retarget_duel(1, 0, 3)).is_none(),
        "a question that must be changed offers no keeping"
    );
    let all = answers_for(&duel, Lang::En, false, false, false);
    assert!(
        all.iter()
            .any(|(says, _)| *says == Says::Answer(PromptAction::Confirm)),
        "the confirmation stays beside it"
    );
    assert!(
        all.iter().any(|(says, _)| *says == keep),
        "answers_for carries the button the confirmation row builds"
    );
}

#[test]
fn auto_damage_button_is_only_offered_for_combat_shares() {
    use baylee_engine::choice::{NumberPrompt, Pending};
    let me = baylee_core::ids::PlayerId::new(0);
    for (reason, offered) in [
        (NumberPrompt::X, false),
        (
            NumberPrompt::CombatDamage {
                source: baylee_core::ids::ObjectId::new(1, 0),
                recipient: baylee_core::ids::ObjectId::new(2, 0),
                index: 1,
                of: 4,
                left: 5,
            },
            true,
        ),
    ] {
        let duel = Duel {
            interaction: Some(baylee_client_core::Interaction::new(
                Pending::ChooseNumber {
                    player: me,
                    min: 0,
                    max: 5,
                    reason,
                },
                me,
            )),
            ..Duel::default()
        };
        let answers = answers_for(&duel, Lang::De, false, false, false);
        assert_eq!(
            answers
                .iter()
                .any(|(a, _)| *a == Says::Answer(PromptAction::AutoDamage)),
            offered
        );
        assert!(
            answers
                .iter()
                .any(|(a, _)| *a == Says::Answer(PromptAction::Confirm))
        );
    }
}

/// A cast the view names (CR 732) adds "Cancel cast" as the last answer,
/// capped with `Esc`, to its target question and its payment window alike;
/// the same questions with no cast named offer no such answer.
#[test]
fn a_named_cast_offers_its_cancel_and_nothing_else_does() {
    use baylee_engine::choice::{LegalActions, Pending, TargetPrompt};
    let questions = [
        Pending::ChooseTargets {
            player: PlayerId::new(0),
            options: vec![ObjectId::new(30, 0)],
            player_options: Vec::new(),
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                ..LegalActions::default()
            }),
        },
    ];
    let prefs = crate::prefs::Prefs::default();
    for pending in questions {
        for named in [true, false] {
            let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
            view.owed = Some(baylee_core::mana::ManaPayment::Fixed(
                baylee_core::mana::ManaCost::parse("{1}{R}"),
            ));
            view.casting = named.then_some(ObjectId::new(90, 0));
            let mut duel = Duel::default();
            duel.receive_view(view);
            duel.receive_choice(pending.clone());
            let answers = answers_for(&duel, Lang::En, false, false, false);
            let at = answers
                .iter()
                .position(|(says, _)| *says == Says::Command(super::super::MenuAction::CancelCast));
            if named {
                assert_eq!(at, Some(answers.len() - 1), "last: {answers:?}");
                assert_eq!(answers[answers.len() - 1].1, "Cancel cast");
                let caps = keys_for(&prefs, &answers, false, false);
                assert!(caps[answers.len() - 1].is_some(), "with Esc's cap");
            } else {
                assert_eq!(at, None, "{answers:?}");
            }
        }
    }
}

/// The payment shelf in a window owing `{1}`: with nothing staged the
/// confirm is "Pay now (tap 1)" and there is nothing to take back; with a
/// Forest staged (covering the cost) it is "Pay" and the shelf offers to
/// take the staged taps back; taking them back restores the first.
#[test]
fn the_payment_shelf_follows_what_is_staged() {
    use baylee_client_core::manaplan::{Step, Tap};
    let confirm = |answers: &[(Says, String)]| {
        answers
            .iter()
            .find(|(says, _)| *says == Says::Answer(PromptAction::Confirm))
            .map(|(_, label)| label.clone())
    };
    let has_unstage = |answers: &[(Says, String)]| {
        answers
            .iter()
            .any(|(says, _)| *says == Says::Command(super::super::MenuAction::UnstagePayment))
    };
    let mut duel =
        crate::owed_tests::seat_with_two_forests(Some(baylee_core::mana::ManaCost::parse("{1}")));
    for lang in [Lang::En, Lang::De] {
        let answers = answers_for(&duel, lang, false, false, false);
        assert_eq!(
            confirm(&answers),
            Some(Phrase::PayStage.fill(lang, &["1"])),
            "{lang:?}: nothing staged, a plan stands"
        );
        assert!(!has_unstage(&answers), "{lang:?}: nothing to take back");
    }

    let forest = duel.view.as_ref().expect("the view").battlefield[0].id;
    assert!(duel.stage(Step {
        source: forest,
        tap: Tap::Intrinsic,
        color: None,
    }));
    for lang in [Lang::En, Lang::De] {
        let answers = answers_for(&duel, lang, false, false, false);
        assert_eq!(
            confirm(&answers),
            Some(Phrase::PayGo.text(lang).to_string()),
            "{lang:?}: the staged Forest covers the cost"
        );
        assert!(
            has_unstage(&answers),
            "{lang:?}: the staged tap can be undone"
        );
        assert!(
            answers
                .iter()
                .any(|(says, _)| *says == Says::Command(super::super::MenuAction::DeclinePayment)),
            "{lang:?}: and declining stays"
        );
    }

    assert!(duel.unstage_all());
    let answers = answers_for(&duel, Lang::En, false, false, false);
    assert_eq!(
        confirm(&answers),
        Some(Phrase::PayStage.fill(Lang::En, &["1"]))
    );
    assert!(!has_unstage(&answers));
}

/// With one tap staged and `{2}` owed, the shelf still says "Pay now" and
/// counts only the taps still to stage: the Forest already staged is not
/// asked for again.
#[test]
fn the_pay_now_label_counts_only_the_taps_still_to_stage() {
    use baylee_client_core::manaplan::{Step, Tap};
    let mut duel =
        crate::owed_tests::seat_with_two_forests(Some(baylee_core::mana::ManaCost::parse("{2}")));
    let answers = answers_for(&duel, Lang::En, false, false, false);
    assert!(
        answers
            .iter()
            .any(|(_, label)| *label == Phrase::PayStage.fill(Lang::En, &["2"])),
        "two taps owed: {answers:?}"
    );
    let forest = duel.view.as_ref().expect("the view").battlefield[0].id;
    assert!(duel.stage(Step {
        source: forest,
        tap: Tap::Intrinsic,
        color: None,
    }));
    let answers = answers_for(&duel, Lang::En, false, false, false);
    assert!(
        answers
            .iter()
            .any(|(says, label)| *says == Says::Answer(PromptAction::Confirm)
                && *label == Phrase::PayStage.fill(Lang::En, &["1"])),
        "one tap still to stage: {answers:?}"
    );
}
