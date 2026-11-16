//! Card decisions retain their rules and mana symbols without waiting for images.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::{ids::CardIndex, types::TypeSet};

pub(super) fn hover(
    id: CardIndex,
    catalog: Option<&domain::cards::Catalog>,
    lang: Lang,
) -> crate::lobby::HoverCard {
    let Some(art) = catalog
        .and_then(|catalog| catalog.card(id))
        .and_then(|card| card.run)
        .and_then(|rules| rules.presentation)
    else {
        return crate::lobby::hover_of_index(id.get());
    };
    let print = baylee_core::deckrow::PrintChoice {
        scryfall_id: Some(art.printing.to_string()),
        lang: Some("en".to_owned()),
        finish: Some(art.finish),
        ..default()
    };
    crate::lobby::preview::hover_of_print(id.get(), &crate::towerprints::choice(&print, lang))
}

pub(super) fn rules(commands: &mut Commands, root: Entity, kit: Kit, id: CardIndex) {
    let Some(definition) = baylee_cards::by_index(id) else {
        return;
    };
    label(
        commands,
        root,
        kit,
        Phrase::EchoesCardRulesLanguage.text(lang(kit)),
        tokens::MUTED,
    );
    for (index, face) in definition.faces.iter().enumerate() {
        let Some(text) = crate::cardtext::english(id, index) else {
            continue;
        };
        if index > 0 {
            heading(commands, root, kit, &text.name);
        }
        let band = row(commands, root, kit);
        if !face.types.contains(TypeSet::LAND)
            && let Some(cost) =
                crate::manaui::spawn_cost(commands, kit.fonts, &text.mana_cost, kit.m.text)
        {
            commands.entity(band).add_child(cost);
        }
        label(commands, band, kit, &text.type_line, tokens::GOLD);
        if let (Some(power), Some(toughness)) = (face.power, face.toughness) {
            label(
                commands,
                band,
                kit,
                &format!("{power}/{toughness}"),
                tokens::INK,
            );
        }
        if !text.oracle_text.is_empty() {
            let rules = crate::manaui::spawn_rich(
                commands,
                kit.fonts,
                &text.oracle_text,
                kit.m.text,
                tokens::INK,
            );
            commands.entity(root).add_child(rules);
        }
    }
}

pub(super) fn preview(commands: &mut Commands, root: Entity, kit: Kit, id: CardIndex) {
    let card = panel(commands, root, kit);
    commands
        .entity(card)
        .insert(BorderColor::all(tokens::ACCENT));
    heading(commands, card, kit, &words::card(id, lang(kit)));
    rules(commands, card, kit, id);
}
