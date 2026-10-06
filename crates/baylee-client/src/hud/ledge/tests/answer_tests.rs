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
    assert_eq!(text, Phrase::ChooseNewTarget.text(Lang::De));
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
