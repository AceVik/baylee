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
    let copy = presentation::feature(
        commands,
        card,
        kit,
        crate::challenge_art::Scene::Tower,
        Phrase::TowerNav.text(lang),
    );
    label(
        commands,
        copy,
        kit,
        Phrase::TowerLead.text(lang),
        tokens::MUTED,
    );
    if let Some(run) = &model.tower.run {
        presentation::tower_score(
            commands,
            copy,
            kit,
            lang,
            (
                run.progress.cleared,
                3 - run.progress.lives,
                run.progress.turns,
            ),
        );
    }
    control(
        commands,
        card,
        kit,
        lang,
        Phrase::TowerOpen,
        TowerAction::Open,
        model,
        Weight::Gold,
    );
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
            node.padding = UiRect::all(px_fixed(kit.m.pad));
        });
    commands.entity(hero).insert(BorderColor {
        top: tokens::GOLD,
        bottom: tokens::GOLD,
        ..BorderColor::all(tokens::BORDER)
    });
    let copy = presentation::feature(
        commands,
        hero,
        kit,
        crate::challenge_art::Scene::Tower,
        words(&season.theme.title, lang),
    );
    label(
        commands,
        copy,
        kit,
        Phrase::TowerLead.text(lang),
        tokens::MUTED,
    );
    presentation::calendar(commands, hero, kit, lang, season.theme.period);
    // A victory's next decision leads the page, before route/history and consent.
    if tower
        .run
        .as_ref()
        .is_some_and(|run| run.progress.phase == Phase::Choosing)
    {
        upgrades::draw(commands, body, kit, lang, model);
    }
    let halves = row(commands, body, kit);
    commands
        .entity(halves)
        .entry::<Node>()
        .and_modify(|mut n| n.align_items = AlignItems::Stretch);
    let route = column(commands, halves, kit, 48.0);
    let progress = column(commands, halves, kit, 48.0);
    map::draw(commands, route, kit, lang, model, season);
    controls::draw(commands, progress, kit, lang, model, season);
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
