//! Temporary player actions on the existing ability-sheet surface.
#[allow(clippy::wildcard_imports)] // the shared sheet vocabulary
use super::*;
use baylee_cards_dsl::SpecialActionCost;
use baylee_client_core::granted;
use baylee_engine::choice::{GrantedActionKind, GrantedActionOffer};

/// What the granted-actions sheet was last built from.
pub(crate) struct GrantedRevision {
    menu: granted::Draft,
    offers: Vec<GrantedActionOffer>,
    lang: Lang,
    generation: u64,
    seq: Option<u64>,
}

/// Current offers belong only to this seat's priority, including mana payment.
pub(crate) fn granted_offers(duel: &Duel) -> &[GrantedActionOffer] {
    duel.interaction
        .as_ref()
        .and_then(|i| i.legal_actions())
        .map_or(&[], |legal| legal.granted_actions.as_slice())
}

/// Pointer, touch and keyboard share the same draft and revalidation.
pub(crate) fn granted_click(duel: &mut Duel, action: MenuAction) {
    let offers = granted_offers(duel).to_vec();
    match action {
        MenuAction::ToggleGrantedActions => {
            if duel.granted_menu.open {
                duel.granted_menu = default();
            } else {
                duel.granted_menu.show(&offers);
                duel.ability_menu = None;
                duel.cast_menu = None;
                duel.armed = None;
                duel.hovered = None;
                duel.hovered_at = None;
            }
        }
        MenuAction::PickGranted(id) => duel.granted_menu.select(id, &offers),
        MenuAction::ConfirmGranted => {
            if let Some(action) = duel.granted_menu.confirm(&offers) {
                duel.submit(action);
            }
        }
        _ => {}
    }
}

/// Existing navigation and confirmation keys operate the open sheet.
pub(crate) fn granted_keys(fired: crate::keys::Fired, duel: &mut Duel) -> bool {
    use baylee_client_core::prefs::Action;
    if fired.has(Action::GrantedActions)
        && !duel.browser.is_typing()
        && !granted_offers(duel).is_empty()
    {
        granted_click(duel, MenuAction::ToggleGrantedActions);
        return true;
    }
    if !duel.granted_menu.open {
        return false;
    }
    let offers = granted_offers(duel).to_vec();
    duel.granted_menu.sync(&offers);
    if !duel.granted_menu.open {
        return false;
    }
    if fired.has(Action::Cancel) {
        duel.granted_menu = default();
        return true;
    }
    let step = i32::from(fired.has(Action::CursorDown) || fired.has(Action::CursorRight))
        - i32::from(fired.has(Action::CursorUp) || fired.has(Action::CursorLeft));
    if step != 0 {
        let index = offers
            .iter()
            .position(|o| Some(o.id) == duel.granted_menu.focused)
            .unwrap_or(0);
        let next = abilitysheet::step_down(offers.len(), index, step);
        duel.granted_menu.focused = Some(offers[next].id);
    } else if fired.has(Action::Primary) || fired.has(Action::Confirm) {
        if duel.granted_menu.selected.as_ref().map(|o| o.id) == duel.granted_menu.focused {
            granted_click(duel, MenuAction::ConfirmGranted);
        } else if let Some(id) = duel.granted_menu.focused {
            granted_click(duel, MenuAction::PickGranted(id));
        }
    }
    true
}

fn cost(offer: &GrantedActionOffer, lang: Lang) -> String {
    match offer.cost {
        SpecialActionCost::Life(amount) => {
            Phrase::GrantedLifeCost.fill(lang, &[&amount.to_string()])
        }
        SpecialActionCost::Mana(mana) => mana.to_string(),
    }
}

fn recipient(
    offer: &GrantedActionOffer,
    duel: &Duel,
    lang: Lang,
    faces: &crate::cardtext::CardTexts,
) -> Option<String> {
    let GrantedActionKind::PreventNextDamage { target, .. } = offer.effect else {
        return None;
    };
    let label = crate::choices::target_label(
        lang,
        target,
        crate::choices::FaceNames {
            view: duel.view.as_ref(),
            texts: Some(faces),
        },
        duel.statics.as_ref(),
    );
    Some(Phrase::GrantedRecipient.fill(lang, &[&label]))
}

#[derive(Component)]
pub(crate) struct GrantedSheet;

#[derive(Component)]
pub(crate) struct GrantedScroll;

#[derive(Component)]
pub(crate) struct FocusedOffer;

/// Reuses ability rows, full Oracle blocks, fonts and sheet controls.
pub(crate) fn sync_granted_sheet(
    mut commands: Commands,
    mut duel: ResMut<Duel>,
    fonts: Res<UiFonts>,
    faces: Res<crate::cardtext::CardTexts>,
    settings: Res<crate::settings::ClientSettings>,
    existing: Query<Entity, With<GrantedSheet>>,
    mut revision: Local<Option<GrantedRevision>>,
) {
    // The draft as it should stand, written back only if it moved: this runs
    // every frame, and a write through the duel marks all of it changed.
    let menu = if duel.ability_menu.is_some() || duel.cast_menu.is_some() {
        granted::Draft::default()
    } else {
        let mut menu = duel.granted_menu.clone();
        menu.sync(granted_offers(&duel));
        menu
    };
    if menu != duel.granted_menu {
        duel.granted_menu = menu;
    }
    let lang = Lang::of(&settings.lang);
    let generation = faces.generation();
    let seq = duel.view.as_ref().map(|v| v.seq);
    let offers = granted_offers(&duel);
    // Compared in place, and copied only when it differs: the key used to be
    // a `Debug` string formatted on every frame.
    if revision.as_ref().is_some_and(|r| {
        r.menu == duel.granted_menu
            && r.offers == offers
            && r.lang == lang
            && r.generation == generation
            && r.seq == seq
    }) {
        return;
    }
    *revision = Some(GrantedRevision {
        menu: duel.granted_menu.clone(),
        offers: offers.to_vec(),
        lang,
        generation,
        seq,
    });
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    if !duel.granted_menu.open {
        return;
    }
    let root = commands
        .spawn((
            GrantedSheet,
            crate::table::DuelStage,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            ZIndex(4),
        ))
        .id();
    let sheet = commands
        .spawn((
            GrantedScroll,
            crate::hud::scroll::Scrolls,
            ScrollPosition::default(),
            Node {
                width: px(SHEET_MAX),
                max_width: percent(92),
                max_height: percent(75),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                padding: UiRect::vertical(px(SHEET_PAD_Y)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            crate::hud::sheet_shadow(),
        ))
        .id();
    commands.entity(root).add_child(sheet);
    let heading = commands
        .spawn((
            Text::new(Phrase::GrantedActions.text(lang)),
            tf_bold(&fonts, 16.0),
            TextColor(palette::CANDLE),
            Node {
                margin: UiRect::horizontal(px(SHEET_PAD_X)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(heading);
    spawn_granted_rows(&mut commands, &fonts, &faces, &duel, lang, sheet, offers);
    spawn_granted_footer(
        &mut commands,
        &fonts,
        lang,
        sheet,
        duel.granted_menu.selected.is_some(),
    );
}

fn spawn_granted_rows(
    commands: &mut Commands,
    fonts: &UiFonts,
    faces: &crate::cardtext::CardTexts,
    duel: &Duel,
    lang: Lang,
    sheet: Entity,
    offers: &[GrantedActionOffer],
) {
    for (index, offer) in offers.iter().enumerate() {
        let face = offer.ability.and_then(|a| faces.face(a.card, 0));
        let mut blocks = Vec::new();
        if let Some(face) = face {
            blocks.push(TextBlock::Rules(face.name));
            if let Some(oracle) = offer
                .ability
                .and_then(|a| crate::cardtext::english(a.card, 0))
            {
                blocks.push(TextBlock::Rules(oracle.oracle_text));
            }
        }
        if let Some(recipient) = recipient(offer, duel, lang, faces) {
            blocks.push(TextBlock::Rules(recipient));
        }
        let row = SheetRow {
            answer: index,
            blocks: Some(blocks),
            line: None,
            cost: Some(cost(offer, lang)),
        };
        let selected = duel
            .granted_menu
            .selected
            .as_ref()
            .is_some_and(|o| o.id == offer.id);
        let entity = spawn_row(
            commands,
            fonts,
            Source::Granted(offer.id),
            &row,
            digit_of(index).unwrap_or('·'),
            selected,
            duel.granted_menu.focused == Some(offer.id),
            Some(84.0),
        );
        if duel.granted_menu.focused == Some(offer.id) {
            commands.entity(entity).insert(FocusedOffer);
        }
        commands.entity(sheet).add_child(entity);
    }
}

fn spawn_granted_footer(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    sheet: Entity,
    selected: bool,
) {
    let foot = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                margin: UiRect::all(px(SHEET_PAD_X)),
                column_gap: px(12),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(foot);
    for (action, label) in [
        (
            MenuAction::ToggleGrantedActions,
            Phrase::SheetCloses.text(lang),
        ),
        (
            MenuAction::ConfirmGranted,
            Phrase::GrantedConfirm.text(lang),
        ),
    ] {
        let enabled = action != MenuAction::ConfirmGranted || selected;
        let button = commands
            .spawn((
                Text::new(label),
                tf_bold(fonts, 13.0),
                TextColor(if enabled {
                    palette::DIALOG_INK
                } else {
                    palette::MUTED
                }),
                Node {
                    padding: UiRect::all(px(8)),
                    ..default()
                },
            ))
            .id();
        if enabled {
            commands.entity(button).insert(MenuButton { action });
        }
        commands.entity(foot).add_child(button);
    }
}

/// Keeps keyboard focus readable after the shared rows have acquired layout.
pub(crate) fn focus_granted_sheet(
    mut commands: Commands,
    mut panels: Query<
        (&mut ScrollPosition, &ComputedNode, &UiGlobalTransform),
        With<GrantedScroll>,
    >,
    rows: Query<(Entity, &ComputedNode, &UiGlobalTransform), With<FocusedOffer>>,
) {
    let Ok((mut scroll, panel, panel_at)) = panels.single_mut() else {
        return;
    };
    if panel.size().y <= 0.0 {
        return;
    }
    for (entity, row, at) in &rows {
        if row.size().y <= 0.0 {
            continue;
        }
        let scale = panel.inverse_scale_factor;
        let top = (at.translation.y - row.size().y / 2.0 - panel_at.translation.y
            + panel.size().y / 2.0)
            * scale;
        let bottom = top + row.size().y * scale;
        let height = panel.size().y * scale;
        if top < 0.0 || row.size().y * scale > height {
            scroll.y += top;
        } else if bottom > height {
            scroll.y += bottom - height;
        }
        commands.entity(entity).remove::<FocusedOffer>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::{AbilityRef, DamageSourceRef, GrantedActionId};
    use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
    fn offer(id: u64) -> GrantedActionOffer {
        GrantedActionOffer {
            id: GrantedActionId::new(id),
            source: DamageSourceRef {
                object: ObjectId::new(77, 0),
                version: 2,
            },
            ability: Some(AbilityRef::new(
                baylee_cards::decks::by_name("Channel").unwrap(),
                AbilityRef::SPELL,
            )),
            timing: baylee_cards_dsl::SpecialActionTiming::ManaAbility,
            cost: SpecialActionCost::Life(1),
            effect: GrantedActionKind::AddMana {
                color: baylee_core::mana::ManaColor::Colorless,
                amount: 1,
            },
        }
    }
    fn pending(offers: Vec<GrantedActionOffer>) -> Pending {
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                granted_actions: offers,
                can_pass: true,
                ..LegalActions::default()
            }),
        }
    }
    fn duel() -> Duel {
        let mut duel = Duel::default();
        duel.receive_view(baylee_client_core::test_support::ViewBuilder::new(2).build());
        duel.receive_choice(pending(vec![offer(11)]));
        duel
    }
    fn key(code: KeyCode) -> crate::keys::Fired {
        let mut keys = ButtonInput::default();
        keys.press(code);
        crate::keys::Fired::of(&keys, &baylee_client_core::prefs::Keymap::default())
    }
    #[test]
    fn actual_input_requires_selection_then_confirmation_and_rejects_expired_id() {
        let mut duel = duel();
        assert!(granted_keys(key(KeyCode::KeyJ), &mut duel));
        assert!(duel.granted_menu.open);
        assert!(duel.outbox().is_empty());
        assert!(granted_keys(key(KeyCode::Enter), &mut duel));
        assert!(duel.outbox().is_empty());
        assert!(granted_keys(key(KeyCode::Escape), &mut duel));
        assert!(duel.outbox().is_empty());
        granted_click(&mut duel, MenuAction::ToggleGrantedActions);
        granted_click(&mut duel, MenuAction::PickGranted(GrantedActionId::new(11)));
        duel.receive_choice(pending(vec![offer(12)]));
        granted_click(&mut duel, MenuAction::ConfirmGranted);
        assert!(duel.outbox().is_empty());
        granted_click(&mut duel, MenuAction::PickGranted(GrantedActionId::new(12)));
        granted_click(&mut duel, MenuAction::ConfirmGranted);
        assert_eq!(
            duel.outbox(),
            &[PlayerAction::TakeGrantedAction {
                id: GrantedActionId::new(12)
            }]
        );
    }
    #[test]
    fn presenter_updates_confirmation_on_selection_and_removes_expired_sheet() {
        #[derive(Resource, Default)]
        struct DuelMoved(bool);
        let mut app = App::new();
        let mut duel = duel();
        granted_click(&mut duel, MenuAction::ToggleGrantedActions);
        app.insert_resource(duel)
            .insert_resource(UiFonts {
                text: Handle::default(),
                medium: Handle::default(),
                bold: Handle::default(),
                italic: Handle::default(),
                medium_italic: Handle::default(),
                serif: Handle::default(),
                serif_italic: Handle::default(),
                icons: Handle::default(),
                mana: Handle::default(),
            })
            .init_resource::<crate::cardtext::CardTexts>()
            .init_resource::<crate::settings::ClientSettings>()
            .add_systems(Update, sync_granted_sheet);
        app.update();
        let confirms = |app: &mut App| {
            app.world_mut()
                .query::<&MenuButton>()
                .iter(app.world())
                .filter(|button| button.action == MenuAction::ConfirmGranted)
                .count()
        };
        assert_eq!(confirms(&mut app), 0);
        granted_click(
            &mut app.world_mut().resource_mut::<Duel>(),
            MenuAction::PickGranted(GrantedActionId::new(11)),
        );
        app.update();
        assert_eq!(confirms(&mut app), 1);
        // A frame where nothing moved rebuilds nothing and leaves the duel
        // unchanged: the sheet standing open costs a comparison.
        app.init_resource::<DuelMoved>().add_systems(
            Update,
            (|duel: Res<Duel>, mut moved: ResMut<DuelMoved>| moved.0 = duel.is_changed())
                .after(sync_granted_sheet),
        );
        let sheet = |app: &mut App| {
            app.world_mut()
                .query_filtered::<Entity, With<GrantedSheet>>()
                .single(app.world())
                .expect("the sheet stands")
        };
        let standing = sheet(&mut app);
        app.update();
        app.update();
        assert_eq!(sheet(&mut app), standing, "rebuilt with nothing changed");
        assert!(
            !app.world().resource::<DuelMoved>().0,
            "the sheet marked the duel changed"
        );
        app.world_mut()
            .resource_mut::<Duel>()
            .receive_choice(pending(vec![]));
        app.update();
        assert_eq!(confirms(&mut app), 0);
        assert_eq!(
            app.world_mut()
                .query::<&GrantedSheet>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
