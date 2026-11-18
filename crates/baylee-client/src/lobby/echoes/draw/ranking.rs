//! Same-cohort comparisons; an unlisted personal best remains private and unranked.
#[allow(clippy::wildcard_imports)]
use super::*;
use api::ranking::{Lane, Standing};

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui) {
    let Some((cohort, lane)) = ui.ranking else {
        return;
    };
    let lang = kit.lang();
    let intro = compact_panel(commands, root, kit);
    widgets::paragraph(
        commands,
        intro,
        kit,
        Phrase::EchoesRankingLead.text(lang),
        tokens::MUTED,
    );
    label(
        commands,
        intro,
        kit,
        &Phrase::EchoesRankingCohort.fill(lang, &[&words::created(cohort, lang)]),
        tokens::GOLD,
    );
    let tabs = row(commands, intro, kit);
    for (selected, title) in [
        (Lane::Echoes, Phrase::EchoesRankingEchoes),
        (Lane::House, Phrase::EchoesRankingHouse),
    ] {
        control(
            commands,
            tabs,
            kit,
            ui,
            title.text(lang),
            Action::Read(Read::Ranking(cohort, selected)),
            true,
        );
    }
    if ui.model.cohorts().len() > 1 {
        let cohorts = row(commands, intro, kit);
        for other in ui.model.cohorts().iter().filter(|c| c.id != cohort) {
            control(
                commands,
                cohorts,
                kit,
                ui,
                &words::created(other.id, lang),
                Action::Read(Read::Ranking(other.id, lane)),
                true,
            );
        }
    }
    let Some(board) = ui
        .model
        .ranking()
        .filter(|b| b.cohort == cohort && b.lane == lane)
    else {
        label(
            commands,
            intro,
            kit,
            Phrase::ChallengeLoading.text(lang),
            tokens::MUTED,
        );
        return;
    };
    personal(commands, root, kit, ui, board);
    let podium = compact_panel(commands, root, kit);
    heading(commands, podium, kit, Phrase::EchoesRankingTop.text(lang));
    if board.leaders.is_empty() {
        widgets::paragraph(
            commands,
            podium,
            kit,
            Phrase::EchoesRankingEmpty.text(lang),
            tokens::MUTED,
        );
    }
    for entry in &board.leaders {
        score(commands, podium, kit, entry);
    }
}

fn personal(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, board: &api::ranking::Board) {
    let lang = kit.lang();
    let personal = compact_panel(commands, root, kit);
    heading(
        commands,
        personal,
        kit,
        Phrase::EchoesRankingBest.text(lang),
    );
    if let Some(best) = &board.best {
        score(commands, personal, kit, best);
    } else {
        widgets::paragraph(
            commands,
            personal,
            kit,
            Phrase::EchoesRankingNoBest.text(lang),
            tokens::MUTED,
        );
    }
    let privacy = row(commands, personal, kit);
    control(
        commands,
        privacy,
        kit,
        ui,
        if board.listing.listed {
            Phrase::EchoesRankingHide
        } else {
            Phrase::EchoesRankingPublish
        }
        .text(lang),
        Action::Listing(!board.listing.listed),
        !ui.model.uncertain()
            && (!ui.model.runs().is_empty()
                || ui.model.run().is_some()
                || board.best.is_some()
                || board.listing.listed),
    );
    label(
        commands,
        privacy,
        kit,
        if board.listing.listed {
            Phrase::EchoesRankingPublic
        } else {
            Phrase::EchoesRankingPrivate
        }
        .text(lang),
        tokens::MUTED,
    );
}

fn score(commands: &mut Commands, root: Entity, kit: Kit, entry: &Standing) {
    let lang = kit.lang();
    let band = row(commands, root, kit);
    let rank_color = match entry.rank {
        Some(1) => tokens::GOLD,
        Some(2) => Color::srgb(0.82, 0.86, 0.92),
        Some(3) => Color::srgb(0.84, 0.58, 0.36),
        _ => tokens::MUTED,
    };
    commands
        .entity(band)
        .insert((
            BackgroundColor(if entry.yours {
                tokens::SELECTED
            } else {
                tokens::CONTROL.with_alpha(0.55)
            }),
            BorderColor::all(if entry.yours {
                tokens::ACCENT.with_alpha(0.55)
            } else if matches!(entry.rank, Some(1..=3)) {
                rank_color.with_alpha(0.45)
            } else {
                tokens::BORDER
            }),
            surfaces::elevation_subcard(),
        ))
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.padding = UiRect::axes(kit.m.px(12.0), kit.m.px(8.0));
            n.min_height = kit.m.px(44.0);
            n.column_gap = kit.m.px(12.0);
            n.border = UiRect::all(px_fixed(1.0));
            n.border_radius = BorderRadius::all(kit.m.px(8.0));
        });
    let rank_label = controls::label(
        commands,
        kit,
        &entry.rank.map_or_else(|| "—".into(), |r| format!("#{r}")),
        kit.m.text,
        rank_color,
    );
    commands
        .entity(rank_label)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.text))
        .entry::<Node>()
        .and_modify(move |mut n| n.min_width = kit.m.px(40.0));
    commands.entity(band).add_child(rank_label);

    let handle = entry
        .name
        .as_ref()
        .map(|n| format!("{}#{:04x}", n.display, n.tag));
    if let Some(ref handle_str) = handle {
        let avatar =
            crate::shellkit::avatar::disc(commands, kit.fonts, handle_str, kit.m.scale * 28.0);
        commands.entity(band).add_child(avatar);
        commands.entity(band).insert((
            crate::shellkit::avatar::PersonLink {
                handle: handle_str.clone(),
                guest: false,
                row: true,
            },
            Pickable::default(),
            PickingInteraction::default(),
        ));
    }
    let name_str = handle.unwrap_or_else(|| Phrase::EchoesRankingYou.text(lang).to_owned());
    let name_label = controls::label(
        commands,
        kit,
        &name_str,
        kit.m.text,
        if entry.yours {
            tokens::ACCENT
        } else {
            tokens::INK
        },
    );
    if entry.yours {
        commands
            .entity(name_label)
            .insert(crate::hud::tf_bold(kit.fonts, kit.m.text));
    }
    commands.entity(band).add_child(name_label);
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(band).add_child(spacer);
    label(
        commands,
        band,
        kit,
        &Phrase::EchoesRankingScore
            .fill(lang, &[&entry.wins.to_string(), &entry.defeats.to_string()]),
        tokens::INK,
    );
}
