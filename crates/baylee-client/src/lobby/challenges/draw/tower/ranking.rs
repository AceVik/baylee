//! Independent, paginated rankings with explicit comparison rules.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, model: &State) {
    let section = panel(commands, body, kit);
    if let Some(season) = &model.tower.season {
        presentation::title(
            commands,
            section,
            kit,
            crate::hud::glyph::CROWN,
            words(&season.theme.title, lang),
        );
        presentation::calendar(commands, section, kit, lang, season.theme.period);
    }
    let tabs = row(commands, section, kit);
    for (board, phrase) in [
        (tower_api::Board::Ascent, Phrase::TowerBoardAscent),
        (tower_api::Board::Sprint, Phrase::TowerBoardSprint),
    ] {
        control(
            commands,
            tabs,
            kit,
            lang,
            phrase,
            TowerAction::Board(board),
            model,
            if model.tower.board == board {
                Weight::Gold
            } else {
                Weight::Secondary
            },
        );
    }
    label(
        commands,
        section,
        kit,
        Phrase::TowerRankingRules.text(lang),
        tokens::MUTED,
    );
    if model.tower.standings.is_empty() {
        label(
            commands,
            section,
            kit,
            Phrase::ChallengeNoScores.text(lang),
            tokens::MUTED,
        );
    }
    standings(commands, section, kit, lang, model);
    let pager = row(commands, section, kit);
    if model.tower.offset > 0 {
        control(
            commands,
            pager,
            kit,
            lang,
            Phrase::ChallengePrevious,
            TowerAction::Page(false),
            model,
            Weight::Ghost,
        );
    }
    if model.tower.standings.len() == 30 && model.tower.offset < 3000 {
        control(
            commands,
            pager,
            kit,
            lang,
            Phrase::ChallengeNext,
            TowerAction::Page(true),
            model,
            Weight::Ghost,
        );
    }
}

fn standings(commands: &mut Commands, section: Entity, kit: Kit, lang: Lang, model: &State) {
    for standing in &model.tower.standings {
        let line = row(commands, section, kit);
        commands
            .entity(line)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.padding = UiRect::all(kit.m.px(8.0));
                node.min_height = kit.m.px(48.0);
                node.border = UiRect::bottom(px(1));
            });
        commands
            .entity(line)
            .insert(BorderColor::all(tokens::BORDER));
        if standing.own {
            commands.entity(line).insert(BackgroundColor(tokens::MIST));
        }
        let name = standing.tag.map_or_else(
            || standing.name.clone(),
            |tag| format!("{}#{tag:04x}", standing.name),
        );
        label(
            commands,
            line,
            kit,
            &format!("#{}  {name}", standing.rank),
            if standing.own {
                tokens::GOLD
            } else {
                tokens::INK
            },
        );
        let spacer = crate::lobby::parts::grow(commands);
        commands.entity(line).add_child(spacer);
        if model.tower.board == tower_api::Board::Sprint {
            presentation::metric(
                commands,
                line,
                kit,
                crate::hud::glyph::CLOCK,
                Phrase::ChallengeStatTime.text(lang),
                &baylee_client_core::challenges::presentation::elapsed(standing.milliseconds),
            );
        } else {
            presentation::tower_score(
                commands,
                line,
                kit,
                lang,
                (standing.floors, standing.losses, standing.turns),
            );
        }
    }
}
