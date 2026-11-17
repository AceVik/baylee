//! Read-only upgrade comparison; the selected goods and purchase stay unchanged.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, selected: Definition) {
    let expanded = ui.tiers == Some(selected.family);
    control(
        commands,
        root,
        kit,
        ui,
        if expanded {
            Phrase::EchoesTiersHide
        } else {
            Phrase::EchoesTiersShow
        }
        .text(lang(kit)),
        Action::Tiers(selected.family),
        true,
    );
    if !expanded {
        return;
    }
    widgets::paragraph(
        commands,
        root,
        kit,
        Phrase::EchoesTiersLead.text(lang(kit)),
        tokens::MUTED,
    );
    for rarity in Rarity::ALL {
        let definition = Definition {
            family: selected.family,
            rarity,
        };
        let tier = panel(commands, root, kit);
        commands
            .entity(tier)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.padding = UiRect::all(kit.m.px(8.0));
                node.row_gap = kit.m.px(4.0);
            });
        if rarity == selected.rarity {
            commands
                .entity(tier)
                .insert(BorderColor::all(tokens::ACCENT));
        }
        let title = row(commands, tier, kit);
        commands
            .entity(title)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_wrap = FlexWrap::NoWrap;
                node.align_items = AlignItems::Center;
            });
        merchandise::image(commands, title, kit, Goods::Relic(definition), 40.0);
        label(
            commands,
            title,
            kit,
            words::rarity(rarity, lang(kit)),
            tokens::INK,
        );
        widgets::paragraph(
            commands,
            tier,
            kit,
            &words::tier(definition, lang(kit)),
            tokens::INK,
        );
    }
}
