//! Retained, responsive Tower panels using the game's existing typography and art.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::lobby::challenges::draw::{attach_body, heading, label, panel, presentation, row};
use crate::{
    hud::glyph,
    shellkit::{
        controls::{self, Kit, Live, Weight},
        px_fixed, tokens,
    },
};
use baylee_client_core::echoes::draft::allowed;
use domain::v2::{
    catalog::{Definition, Family, Potion, Rarity},
    shop::Goods,
};

mod deck;
mod equipment;
mod landing;
mod shop;
mod words;

pub(in crate::lobby) fn entry(commands: &mut Commands, root: Entity, kit: Kit, lang: Lang) {
    let card = panel(commands, root, kit);
    let copy = presentation::feature(
        commands,
        card,
        kit,
        crate::challenge_art::Scene::Tower,
        Phrase::EchoesTitle.text(lang),
    );
    label(
        commands,
        copy,
        kit,
        Phrase::EchoesLead.text(lang),
        tokens::MUTED,
    );
    let button = controls::button_with_icon(
        commands,
        kit,
        glyph::COMPASS,
        Phrase::TowerOpen.text(lang),
        Weight::Gold,
        Live::Yes,
        None,
        Press::Echoes(Action::Open),
    );
    commands
        .entity(button)
        .insert(crate::shellkit::focus::Stop::new(
            crate::lobby::orders::CHALLENGES.name,
            "echoes",
        ));
    commands.entity(card).add_child(button);
}

pub(in crate::lobby) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
    scrolled: &Scrolled,
) {
    let ui = &state.echoes;
    let lang = state.lobby.lang();
    let body = commands
        .spawn((
            Node {
                width: percent(100),
                max_width: px_fixed(kit.m.body_max),
                align_self: AlignSelf::Center,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_height: px(0),
                row_gap: px_fixed(kit.m.gap),
                padding: UiRect::all(px_fixed(kit.m.body)),
                overflow: if metrics.frame == Frame::Compact {
                    Overflow::DEFAULT
                } else {
                    Overflow::scroll_y()
                },
                ..default()
            },
            Pickable::default(),
        ))
        .id();
    if metrics.frame != Frame::Compact {
        commands.entity(body).insert((
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Table))),
        ));
    }
    attach_body(commands, root, body, metrics.frame);
    let hero = panel(commands, body, kit);
    let top = row(commands, hero, kit);
    presentation::title(
        commands,
        top,
        kit,
        glyph::CROWN,
        Phrase::EchoesTitle.text(lang),
    );
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(top).add_child(spacer);
    control(
        commands,
        top,
        kit,
        ui,
        Phrase::Back.text(lang),
        Action::Back,
        true,
    );
    control(
        commands,
        top,
        kit,
        ui,
        Phrase::Refresh.text(lang),
        Action::Refresh,
        true,
    );
    label(
        commands,
        hero,
        kit,
        Phrase::EchoesLead.text(lang),
        tokens::MUTED,
    );
    if !state.central_ready() {
        label(
            commands,
            hero,
            kit,
            Phrase::ChallengeSignIn.text(lang),
            tokens::MUTED,
        );
        return;
    }
    notices(commands, hero, kit, ui);
    if ui.confirmation.is_some() {
        confirmation(commands, body, kit, ui);
    }
    selected(commands, body, hero, kit, ui);
    landing::draw(commands, body, kit, ui);
}

fn selected(commands: &mut Commands, body: Entity, hero: Entity, kit: Kit, ui: &Ui) {
    let lang = lang(kit);
    if let Some(run) = ui.model.run() {
        landing::progress(commands, hero, kit, ui, run);
        let tabs = row(commands, body, kit);
        for (page, title) in [
            (Page::Shop, Phrase::EchoesShop),
            (Page::Equipment, Phrase::EchoesEquipment),
            (Page::Deck, Phrase::ChallengeDeck),
        ] {
            control(
                commands,
                tabs,
                kit,
                ui,
                title.text(lang),
                Action::Page(page),
                true,
            );
        }
        if let Some(cohort) = ui.model.publication() {
            match ui.page {
                Page::Shop => shop::draw(commands, body, kit, ui, run, cohort),
                Page::Equipment => equipment::draw(commands, body, kit, ui, run, cohort),
                Page::Deck => deck::draw(commands, body, kit, ui, run, cohort),
            }
        }
    }
}

fn lang(kit: Kit) -> Lang {
    if kit.german { Lang::De } else { Lang::En }
}

fn notices(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui) {
    let lang = lang(kit);
    if ui.model.busy() {
        label(
            commands,
            root,
            kit,
            Phrase::ChallengeLoading.text(lang),
            tokens::ACCENT,
        );
    }
    if let Some(failure) = ui.model.failure() {
        label(
            commands,
            root,
            kit,
            words::failure(failure).text(lang),
            tokens::DANGER,
        );
    }
    if ui.model.uncertain() {
        label(
            commands,
            root,
            kit,
            Phrase::EchoesUncertain.text(lang),
            tokens::MUTED,
        );
        control(
            commands,
            root,
            kit,
            ui,
            Phrase::EchoesRetry.text(lang),
            Action::Retry,
            true,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn control(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    words: &str,
    action: Action,
    available: bool,
) {
    let lang = lang(kit);
    let allowed = available
        && !ui.model.busy()
        && (ui.confirmation.is_none() || matches!(action, Action::Accept | Action::Cancel));
    let button = controls::button_with_icon(
        commands,
        kit,
        words::icon(action),
        words,
        if matches!(
            action,
            Action::Accept | Action::Buy | Action::Play(_) | Action::Resume | Action::SaveDeck
        ) {
            Weight::Gold
        } else {
            Weight::Secondary
        },
        if allowed {
            Live::Yes
        } else {
            Live::No(Phrase::EchoesUnavailable.text(lang))
        },
        None,
        Press::Echoes(action),
    );
    commands.entity(button).insert((
        super::keys::stop(action, ui),
        crate::shellkit::focus::Current(words::selected(action, ui)),
    ));
    commands.entity(root).add_child(button);
}

fn can(ui: &Ui, command: &Command) -> bool {
    !ui.model.uncertain()
        && (matches!(command, Command::Edit { .. }) || ui.choices.deck_unchanged(ui.model.run()))
        && ui
            .model
            .run()
            .zip(ui.model.publication())
            .is_some_and(|(run, cohort)| allowed(run, cohort, command))
}

fn tile(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
    let tile = panel(commands, root, kit);
    commands
        .entity(tile)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.width = Val::Auto;
            n.flex_basis = if kit.m.frame == crate::shellkit::Frame::Phone {
                percent(100)
            } else {
                percent(47)
            };
            n.flex_grow = 1.0;
            n.min_width = px(0);
            n.align_self = AlignSelf::Stretch;
        });
    tile
}

fn confirmation(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui) {
    let lang = lang(kit);
    let card = panel(commands, root, kit);
    commands.entity(card).insert(BorderColor::all(tokens::GOLD));
    heading(commands, card, kit, Phrase::ChallengeConfirm.text(lang));
    let (text, valid) = match ui.confirmation.as_ref().expect("shown confirmation") {
        Confirmation::Withdrawal => (
            Phrase::EchoesWithdrawConfirm.text(lang).to_string(),
            !ui.model.uncertain(),
        ),
        Confirmation::Enrollment {
            cohort,
            starter,
            consent,
        } => (
            Phrase::EchoesConsent.fill(lang, &[&(*starter + 1).to_string(), &consent.to_string()]),
            *consent == ui.model.consent()
                && ui.model.cohorts().iter().any(|entry| entry.id == *cohort),
        ),
        Confirmation::Choice {
            run,
            revision,
            command,
        } => (
            words::command(command, ui, lang),
            ui.model
                .run()
                .is_some_and(|current| current.id == *run && current.revision == *revision)
                && can(ui, command),
        ),
    };
    label(commands, card, kit, &text, tokens::INK);
    let band = row(commands, card, kit);
    control(
        commands,
        band,
        kit,
        ui,
        Phrase::ChallengeCancel.text(lang),
        Action::Cancel,
        true,
    );
    control(
        commands,
        band,
        kit,
        ui,
        Phrase::ChallengeConfirm.text(lang),
        Action::Accept,
        valid,
    );
}
