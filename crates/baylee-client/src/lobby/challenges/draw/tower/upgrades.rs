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
    confirmation(commands, section, kit, lang, model);
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
        let selected = model.tower.upgrade_preview() == Some(track);
        if selected {
            commands.entity(cell).insert(BorderColor::all(tokens::GOLD));
        }
        presentation::title(
            commands,
            cell,
            kit,
            presentation::action_icon(Action::Tower(TowerAction::Upgrade(track))),
            track_name(track).text(lang),
        );
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
}

fn confirmation(commands: &mut Commands, section: Entity, kit: Kit, lang: Lang, model: &State) {
    if model.tower.upgrade_preview().is_some() {
        let controls = crate::shellkit::surfaces::dialog_button_row(commands, kit, &[]);
        commands.entity(section).add_child(controls);
        let cancel = control(
            commands,
            controls,
            kit,
            lang,
            Phrase::ChallengeCancel,
            TowerAction::Cancel,
            model,
            Weight::Ghost,
        );
        let choose = control(
            commands,
            controls,
            kit,
            lang,
            Phrase::TowerChoose,
            TowerAction::Choose,
            model,
            Weight::Gold,
        );
        crate::shellkit::surfaces::stretch_footer_items(commands, kit, &[cancel, choose]);
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
        let icon = if title == Phrase::TowerRemove {
            crate::hud::glyph::MINUS
        } else {
            crate::hud::glyph::PLUS
        };
        let caption = crate::lobby::parts::glyph_line(commands, kit, icon, title.text(lang), ink);
        commands.entity(card).add_child(caption);
        label(
            commands,
            card,
            kit,
            &format!("{} × {name}", offer.copies),
            tokens::INK,
        );
    }
}
