//! Three equal choices; inspection is reversible, confirmation commits the route.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, model: &State) {
    let Some(run) = &model.tower.run else {
        return;
    };
    let section = panel(commands, body, kit);
    heading(commands, section, kit, Phrase::TowerChooseLead.text(lang));
    label(
        commands,
        section,
        kit,
        Phrase::TowerChoiceNotice.text(lang),
        tokens::MUTED,
    );
    let choices = row(commands, section, kit);
    commands
        .entity(choices)
        .entry::<Node>()
        .and_modify(|mut n| n.align_items = AlignItems::Stretch);
    let counts = run.progress.counts();
    for track in Track::ALL {
        let Some(offer) =
            run.season.theme.upgrades[track.index()].get(usize::from(counts[track.index()]))
        else {
            continue;
        };
        let cell = column(commands, choices, kit, 30.0);
        let selected = model.tower.upgrade == Some(track);
        if selected {
            commands.entity(cell).insert(BorderColor::all(tokens::GOLD));
        }
        heading(commands, cell, kit, track_name(track).text(lang));
        label(commands, cell, kit, words(&offer.title, lang), tokens::INK);
        label(
            commands,
            cell,
            kit,
            words(&offer.summary, lang),
            tokens::MUTED,
        );
        replacement(commands, cell, kit, lang, offer);
        let spacer = crate::lobby::parts::grow(commands);
        commands.entity(cell).add_child(spacer);
        control(
            commands,
            cell,
            kit,
            lang,
            track_name(track),
            TowerAction::Upgrade(track),
            model,
            if selected {
                Weight::Gold
            } else {
                Weight::Secondary
            },
        );
    }
    if model.tower.upgrade.is_some() {
        let controls = row(commands, section, kit);
        control(
            commands,
            controls,
            kit,
            lang,
            Phrase::ChallengeCancel,
            TowerAction::Cancel,
            model,
            Weight::Ghost,
        );
        control(
            commands,
            controls,
            kit,
            lang,
            Phrase::TowerChoose,
            TowerAction::Choose,
            model,
            Weight::Gold,
        );
    }
}

fn replacement(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    lang: Lang,
    offer: &tower_api::Upgrade,
) {
    for (title, name, ink) in [
        (Phrase::TowerRemove, &offer.remove, tokens::MUTED),
        (Phrase::TowerAdd, &offer.add, tokens::ACCENT),
    ] {
        let card = panel(commands, parent, kit);
        commands.entity(card).insert(BackgroundColor(tokens::MIST));
        commands
            .entity(card)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.padding = UiRect::all(kit.m.px(10.0));
                node.row_gap = kit.m.px(4.0);
                node.min_height = kit.m.px(60.0);
            });
        label(commands, card, kit, title.text(lang), ink);
        label(
            commands,
            card,
            kit,
            &format!("{} × {name}", offer.copies),
            tokens::INK,
        );
    }
}
