#[allow(clippy::wildcard_imports)]
use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn control(
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
        if words::selected(action, ui) {
            glyph::CHECK
        } else {
            words::icon(action)
        },
        words,
        if matches!(
            action,
            Action::Accept | Action::Buy | Action::Play(_) | Action::Resume | Action::SaveDeck
        ) {
            Weight::Gold
        } else if words::selected(action, ui) {
            Weight::Primary
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

pub(super) fn can(ui: &Ui, command: &Command) -> bool {
    !ui.model.uncertain()
        && (matches!(command, Command::Edit { .. }) || ui.choices.deck_unchanged(ui.model.run()))
        && ui
            .model
            .run()
            .zip(ui.model.publication())
            .is_some_and(|(run, cohort)| allowed(run, cohort, command))
}

pub(super) fn portrait(commands: &mut Commands, root: Entity, kit: Kit, goods: Goods) {
    let band = row(commands, root, kit);
    commands
        .entity(band)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_wrap = FlexWrap::NoWrap;
            node.align_items = AlignItems::Center;
        });
    let image = commands
        .spawn((
            super::super::art::Portrait(goods),
            Node {
                width: kit.m.px(84.0),
                height: kit.m.px(84.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(band).add_child(image);
    heading(commands, band, kit, &words::goods(goods, lang(kit)));
}

pub(super) fn tile(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
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

pub(super) fn confirmation(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui) {
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
