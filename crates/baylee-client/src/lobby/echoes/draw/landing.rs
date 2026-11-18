#[allow(clippy::wildcard_imports)]
use super::*;
mod progress;
mod starter;
pub(super) use progress::{market_progress, progress};

pub(super) fn draw(
    commands: &mut Commands,
    body: Entity,
    modal_root: Entity,
    kit: Kit,
    ui: &Ui,
    scrolled: &Scrolled,
) {
    if let Some(id) = ui.preview {
        card::preview(commands, modal_root, kit, ui, id);
    }
    owned_runs(commands, body, kit, ui);
    enrollment(commands, body, kit, ui);
    enrollment_footer(commands, body, kit, ui);
    starter::inspect(commands, modal_root, kit, ui, scrolled);
}

fn owned_runs(commands: &mut Commands, body: Entity, kit: Kit, ui: &Ui) {
    if ui.model.runs().is_empty() {
        return;
    }
    let lang = kit.lang();
    let section = compact_panel(commands, body, kit);
    heading(commands, section, kit, Phrase::EchoesRuns.text(lang));
    let list = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(section),
        ))
        .id();
    for entry in ui.model.runs().iter().take(6) {
        owned_run_row(commands, list, kit, ui, entry);
    }
}

fn owned_run_row(commands: &mut Commands, list: Entity, kit: Kit, ui: &Ui, entry: &api::Run) {
    let lang = kit.lang();
    let active = matches!(
        entry.phase,
        api::Phase::Shop
            | api::Phase::Reward
            | api::Phase::Reserved
            | api::Phase::Playing
            | api::Phase::Verifying
    );
    let wins = entry
        .history
        .iter()
        .filter(|duel| duel.outcome == domain::v2::run::Outcome::Victory)
        .count();
    let defeats = entry.history.len().saturating_sub(wins);
    let row_card = commands
        .spawn((
            Node {
                width: percent(100),
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(10.0)),
                column_gap: kit.m.px(12.0),
                row_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(10.0)),
                ..default()
            },
            BackgroundColor(if active {
                tokens::SELECTED
            } else {
                tokens::CONTROL.with_alpha(0.65)
            }),
            BorderColor::all(if active {
                tokens::GOLD.with_alpha(0.60)
            } else {
                tokens::BORDER
            }),
            surfaces::elevation_subcard(),
            Pickable::IGNORE,
            ChildOf(list),
        ))
        .id();
    let commander_name = entry
        .cards
        .formation
        .as_ref()
        .map(|f| words::card(f.commander, lang));
    let status = words::phase(entry.phase).text(lang);
    let title_str = commander_name.map_or_else(
        || status.to_owned(),
        |commander| format!("{commander} · {status}"),
    );
    let title_label = controls::label(
        commands,
        kit,
        &title_str,
        kit.m.text,
        if active { tokens::GOLD } else { tokens::INK },
    );
    commands
        .entity(title_label)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.text));
    commands.entity(row_card).add_child(title_label);
    let stats_text = format!(
        "{} · {}",
        Phrase::EchoesStage.fill(lang, &[&entry.shop.stage.to_string()]),
        Phrase::EchoesRankingScore.fill(lang, &[&wins.to_string(), &defeats.to_string()])
    );
    label(commands, row_card, kit, &stats_text, tokens::MUTED);
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(row_card).add_child(spacer);
    owned_run_open_button(commands, row_card, kit, ui, entry, active);
}

fn owned_run_open_button(
    commands: &mut Commands,
    row_card: Entity,
    kit: Kit,
    ui: &Ui,
    entry: &api::Run,
    active: bool,
) {
    let lang = kit.lang();
    let cta = if active {
        Phrase::EchoesReturnShop.text(lang)
    } else {
        Phrase::EchoesInspect.text(lang)
    };
    let open_btn = controls::button_compact(
        commands,
        kit,
        Some(if active {
            glyph::ARROW_RIGHT
        } else {
            glyph::EYE
        }),
        cta,
        if active {
            Weight::Gold
        } else {
            Weight::Secondary
        },
        if !ui.holding() && ui.choices.deck_unchanged(ui.model.run()) {
            Live::Yes
        } else {
            Live::No(Phrase::EchoesUnavailable.text(lang))
        },
        Press::Echoes(Action::Read(Read::Run(entry.id))),
    );
    commands
        .entity(open_btn)
        .insert(super::keys::stop(Action::Read(Read::Run(entry.id)), ui));
    commands.entity(row_card).add_child(open_btn);
}

fn enrollment(commands: &mut Commands, body: Entity, kit: Kit, ui: &Ui) {
    let lang = kit.lang();
    let band = row(commands, body, kit);
    for cohort in ui.model.cohorts().iter().take(1) {
        let run_format = cohort
            .starters
            .iter()
            .all(|starter| starter.formation.is_some());
        let card = tile(commands, band, kit);
        commands
            .entity(card)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.padding = UiRect::all(kit.m.px(16.0));
                node.row_gap = kit.m.px(12.0);
            });
        heading(commands, card, kit, Phrase::EchoesNewRun.text(lang));
        if !run_format {
            label(
                commands,
                card,
                kit,
                &Phrase::EchoesPublished.fill(lang, &[&words::created(cohort.id, lang)]),
                tokens::MUTED,
            );
        }
        label(
            commands,
            card,
            kit,
            Phrase::EchoesStarterLead.text(lang),
            tokens::MUTED,
        );
        let choices = choices(commands, card, kit, run_format);
        for &number in &cohort.offered {
            let Some(starter) = cohort.starters.get(usize::from(number)) else {
                continue;
            };
            if let Some(formation) = &starter.formation {
                starter::draw(
                    commands,
                    choices,
                    kit,
                    ui,
                    cohort.id,
                    number,
                    formation.commander,
                );
                continue;
            }
            let choice = tile(commands, choices, kit);
            commands
                .entity(choice)
                .entry::<Node>()
                .and_modify(move |mut node| {
                    node.flex_basis = if kit.m.frame == crate::shellkit::Frame::Phone {
                        percent(100)
                    } else {
                        percent(30)
                    };
                });
            let label_text = Phrase::EchoesStarter.fill(lang, &[&(number + 1).to_string()]);
            heading(commands, choice, kit, &label_text);
            for stack in &starter.deck {
                label(
                    commands,
                    choice,
                    kit,
                    &format!("{} × {}", stack.copies, words::card(stack.card, lang)),
                    tokens::MUTED,
                );
            }
            let space = crate::lobby::parts::grow(commands);
            commands.entity(choice).add_child(space);
            control(
                commands,
                choice,
                kit,
                ui,
                &label_text,
                Action::Starter(cohort.id, number),
                !ui.model.uncertain() && ui.choices.deck_unchanged(ui.model.run()),
            );
        }
        if run_format {
            label(
                commands,
                card,
                kit,
                Phrase::EchoesRunStarterFormat.text(lang),
                tokens::MUTED,
            );
        }
    }
    no_cohorts(commands, body, kit, ui);
}

fn no_cohorts(commands: &mut Commands, body: Entity, kit: Kit, ui: &Ui) {
    if ui.loaded && ui.model.cohorts().is_empty() {
        let card = compact_panel(commands, body, kit);
        label(
            commands,
            card,
            kit,
            Phrase::EchoesNoEnrollment.text(kit.lang()),
            tokens::MUTED,
        );
    }
}

fn choices(commands: &mut Commands, card: Entity, kit: Kit, run_format: bool) -> Entity {
    let choices = row(commands, card, kit);
    if run_format {
        commands
            .entity(choices)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.display = Display::Grid;
                node.grid_template_columns = vec![RepeatedGridTrack::flex(
                    if kit.m.frame == crate::shellkit::Frame::Phone {
                        1
                    } else {
                        3
                    },
                    1.0,
                )];
                node.align_items = AlignItems::Stretch;
            });
    }
    choices
}

fn enrollment_footer(commands: &mut Commands, body: Entity, kit: Kit, ui: &Ui) {
    let lang = kit.lang();
    let privacy = compact_panel(commands, body, kit);
    widgets::paragraph(
        commands,
        privacy,
        kit,
        Phrase::EchoesStartSharing.text(lang),
        tokens::MUTED,
    );
    let bottom = row(commands, privacy, kit);
    if ui.withdrawn {
        label(
            commands,
            bottom,
            kit,
            Phrase::EchoesWithdrawn.text(lang),
            tokens::ACCENT,
        );
    }
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(bottom).add_child(spacer);
    control(
        commands,
        bottom,
        kit,
        ui,
        Phrase::EchoesWithdraw.text(lang),
        Action::Withdraw,
        !ui.model.uncertain(),
    );
}
