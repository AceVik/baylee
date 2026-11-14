//! Expedition overview, permanent route and explicit progression controls.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::tower::{Action as TowerAction, api as tower_api};
use tower_api::{Phase, Track};
mod collection;
mod controls;
mod map;
mod ranking;
mod upgrades;

pub(super) fn entry(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, model: &State) {
    let card = panel(commands, body, kit);
    commands.entity(card).insert(BorderColor {
        top: tokens::GOLD,
        ..BorderColor::all(tokens::BORDER)
    });
    let head = row(commands, card, kit);
    heading(commands, head, kit, Phrase::TowerNav.text(lang));
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(head).add_child(spacer);
    control(
        commands,
        head,
        kit,
        lang,
        Phrase::TowerOpen,
        TowerAction::Open,
        model,
        Weight::Gold,
    );
    label(
        commands,
        card,
        kit,
        Phrase::TowerLead.text(lang),
        tokens::MUTED,
    );
    if let Some(run) = &model.tower.run {
        label(
            commands,
            card,
            kit,
            &Phrase::TowerScore.fill(
                lang,
                &[
                    &run.progress.cleared.to_string(),
                    &(3 - run.progress.lives).to_string(),
                    &run.progress.turns.to_string(),
                ],
            ),
            tokens::ACCENT,
        );
    }
}

pub(super) fn draw(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, model: &State) {
    let tower = &model.tower;
    let Some(season) = tower.visible_season() else {
        label(
            commands,
            body,
            kit,
            Phrase::TowerNoSeason.text(lang),
            tokens::MUTED,
        );
        collection::draw(commands, body, kit, lang, model);
        return;
    };
    let hero = panel(commands, body, kit);
    commands
        .entity(hero)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.align_items = AlignItems::Center;
            node.padding = UiRect::axes(px_fixed(kit.m.pad), px_fixed(kit.m.pad * 1.5));
        });
    commands.entity(hero).insert(BorderColor {
        top: tokens::GOLD,
        bottom: tokens::GOLD,
        ..BorderColor::all(tokens::BORDER)
    });
    heading(commands, hero, kit, words(&season.theme.title, lang));
    label(
        commands,
        hero,
        kit,
        Phrase::TowerLead.text(lang),
        tokens::MUTED,
    );
    label(
        commands,
        hero,
        kit,
        &Phrase::ChallengeSchedule.fill(
            lang,
            &[
                &date(season.theme.period.starts_at),
                &date(season.theme.period.ends_at),
            ],
        ),
        tokens::MUTED,
    );
    let halves = row(commands, body, kit);
    commands
        .entity(halves)
        .entry::<Node>()
        .and_modify(|mut n| n.align_items = AlignItems::Stretch);
    let route = column(commands, halves, kit, 48.0);
    let progress = column(commands, halves, kit, 48.0);
    map::draw(commands, route, kit, lang, model, season);
    controls::draw(commands, progress, kit, lang, model, season);
    if tower
        .run
        .as_ref()
        .is_some_and(|run| run.progress.phase == Phase::Choosing)
    {
        upgrades::draw(commands, body, kit, lang, model);
    }
    ranking::draw(commands, body, kit, lang, model);
    collection::draw(commands, body, kit, lang, model);
    details(commands, body, kit, lang, model);
}

fn details(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, model: &State) {
    let rules = panel(commands, body, kit);
    button(
        commands,
        rules,
        kit,
        Phrase::ChallengeRulesToggle.text(lang),
        Action::Rules,
        model,
        Weight::Ghost,
    );
    if model.show_rules {
        label(
            commands,
            rules,
            kit,
            Phrase::TowerRules.text(lang),
            tokens::MUTED,
        );
    }
    if let Some(run) = &model.tower.run {
        let deck = panel(commands, body, kit);
        button(
            commands,
            deck,
            kit,
            Phrase::ChallengeDeck.text(lang),
            Action::Deck,
            model,
            Weight::Ghost,
        );
        if model.show_deck {
            label(
                commands,
                deck,
                kit,
                &run.encounter.description.deck,
                tokens::INK,
            );
        }
    }
}

fn column(commands: &mut Commands, parent: Entity, kit: Kit, share: f32) -> Entity {
    let child = panel(commands, parent, kit);
    commands
        .entity(child)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.width = Val::Auto;
            n.flex_basis = if kit.m.frame == crate::shellkit::Frame::Phone {
                percent(100)
            } else {
                percent(share)
            };
            n.flex_grow = 1.0;
            n.min_width = px_fixed(0.0);
        });
    child
}

fn track_name(track: Track) -> Phrase {
    match track {
        Track::Assault => Phrase::TowerAssault,
        Track::Precision => Phrase::TowerPrecision,
        Track::Resolve => Phrase::TowerResolve,
    }
}

#[allow(clippy::too_many_arguments)]
fn control(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    lang: Lang,
    phrase: Phrase,
    action: TowerAction,
    model: &State,
    weight: Weight,
) {
    button(
        commands,
        root,
        kit,
        phrase.text(lang),
        Action::Tower(action),
        model,
        weight,
    );
}
