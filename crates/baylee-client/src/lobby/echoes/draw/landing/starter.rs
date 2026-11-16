//! A compact identity for each published Commander, never inferred from its position.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    cohort: Uuid,
    number: u8,
    commander: baylee_core::ids::CardIndex,
) {
    let lang = lang(kit);
    let choice = tile(commands, root, kit);
    commands
        .entity(choice)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.min_width = px(0);
            node.row_gap = kit.m.px(8.0);
            node.padding = UiRect::all(kit.m.px(12.0));
            node.align_items = AlignItems::Center;
        });
    let name = words::card(commander, lang);
    let hover = card::hover(
        commander,
        ui.model
            .cohorts()
            .iter()
            .find(|entry| entry.id == cohort)
            .map(|entry| &entry.catalog),
    );
    let image = crate::lobby::thumbnails::spawn_sized(
        commands,
        &hover,
        kit.m.px(112.0),
        kit.m.px(156.0),
        kit.m.px(8.0),
    );
    commands.entity(choice).add_child(image);
    let action = Action::Starter(cohort, number);
    commands.entity(choice).insert((
        crate::shellkit::Role::Hit,
        PickingInteraction::default(),
        Pickable::default(),
        Press::Echoes(action),
        super::super::super::keys::stop(action, ui),
        hover,
        crate::ambience::Feel::tinting_to(tokens::CONTROL, tokens::HOVER),
        crate::ambience::FeelBorder {
            rest: tokens::GOLD.with_alpha(0.4),
            hot: tokens::GOLD,
            press: tokens::INK,
        },
        crate::ambience::FeelShadow,
        crate::shellkit::sheen::Sheen,
    ));
    if ui.model.busy() || ui.model.uncertain() || !ui.choices.deck_unchanged(ui.model.run()) {
        commands.entity(choice).insert(controls::Disabled);
    }
    let title = controls::label(commands, kit, &name, kit.m.text, tokens::INK);
    commands
        .entity(title)
        .insert((
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextLayout::justify(Justify::Center),
        ))
        .entry::<Node>()
        .and_modify(move |mut node| node.min_height = kit.m.px(48.0));
    commands.entity(choice).add_child(title);
    if let Some(strategy) = strategy(&words::card(commander, Lang::En)) {
        let band = row(commands, choice, kit);
        commands
            .entity(band)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_direction = FlexDirection::Column;
                node.align_items = AlignItems::Center;
            });
        label(commands, band, kit, strategy.text(lang), tokens::GOLD);
    }
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(choice).add_child(spacer);
    let actions = row(commands, choice, kit);
    let action = Action::InspectStarter(cohort, number);
    let inspect =
        crate::lobby::parts::icon_button(commands, kit, glyph::LIBRARY, Press::Echoes(action));
    commands
        .entity(inspect)
        .insert(super::super::super::keys::stop(action, ui));
    controls::attach_tooltip(commands, inspect, kit, Phrase::EchoesDeck.text(lang));
    commands.entity(actions).add_child(inspect);
}

pub(super) fn inspect(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui) {
    let Some((cohort, index)) = ui.starter_preview else {
        return;
    };
    let Some(starter) = ui
        .model
        .cohorts()
        .iter()
        .find(|entry| entry.id == cohort)
        .and_then(|entry| entry.starters.get(usize::from(index)))
    else {
        return;
    };
    let mut body = Vec::new();
    let rows = starter.formation.as_ref().map_or_default(|formation| {
        vec![format!("1 {}", words::card(formation.commander, Lang::En))]
    });
    body.push(crate::lobby::decks::spawn_preview_section_container(
        commands,
        kit,
        &rows,
        crate::lobby::decks::DecksView::Compact,
    ));
    let rows: Vec<_> = starter
        .deck
        .iter()
        .map(|stack| format!("{} {}", stack.copies, words::card(stack.card, Lang::En)))
        .collect();
    body.push(crate::lobby::decks::spawn_preview_section_container(
        commands,
        kit,
        &rows,
        crate::lobby::decks::DecksView::Compact,
    ));
    let close = controls::button_compact(
        commands,
        kit,
        Some(glyph::CLOSE),
        Phrase::ShellClose.text(lang(kit)),
        Weight::Secondary,
        Live::Yes,
        Press::Echoes(Action::ClosePreview),
    );
    commands
        .entity(close)
        .insert(super::super::super::keys::stop(Action::ClosePreview, ui));
    let surface = surfaces::sheet_box_listed(
        commands,
        kit,
        surfaces::SheetWidth::Large,
        Phrase::EchoesDeck.text(lang(kit)),
        &body,
        &[close],
        (Scrollable(List::EchoesInspector), ScrollPosition::default()),
    );
    commands
        .entity(surface)
        .insert(Press::Shared(crate::lobby::SharedPress::PickerNothing));
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Echoes(Action::ClosePreview));
    commands.entity(root).add_child(scrim);
}

fn strategy(name: &str) -> Option<Phrase> {
    Some(match name {
        "Emmara, Soul of the Accord"
        | "Rin and Seri, Inseparable"
        | "Maja, Bretagard Protector"
        | "Arasta of the Endless Web" => Phrase::EchoesRunEmmara,
        "Elas il-Kor, Sadistic Pilgrim" | "Ayara, First of Locthwain" => Phrase::EchoesRunElas,
        "Talrand, Sky Summoner"
        | "Kykar, Wind's Fury"
        | "Tura Kennerüd, Skyknight"
        | "Alela, Artful Provocateur" => Phrase::EchoesRunTalrand,
        "Slimefoot, the Stowaway"
        | "Judith, the Scourge Diva"
        | "Yahenni, Undying Partisan"
        | "Mahadi, Emporium Master" => Phrase::EchoesRunSlimefoot,
        "Pia and Kiran Nalaar"
        | "Galazeth Prismari"
        | "Padeem, Consul of Innovation"
        | "Glissa, the Traitor"
        | "Emry, Lurker of the Loch" => Phrase::EchoesRunPia,
        "Radha, Heir to Keld"
        | "Rishkar, Peema Renegade"
        | "Ashaya, Soul of the Wild"
        | "Titania, Protector of Argoth"
        | "Tatyova, Benthic Druid" => Phrase::EchoesPlanLands,
        "Sram, Senior Edificer" => Phrase::EchoesPlanEquipment,
        "Hapatra, Vizier of Poisons" | "Grist, the Hunger Tide" => Phrase::EchoesPlanCounters,
        "Giada, Font of Hope" | "Lyra Dawnbringer" => Phrase::EchoesPlanAngels,
        "Dwynen, Gilt-Leaf Daen" => Phrase::EchoesPlanElves,
        "Pashalik Mons" => Phrase::EchoesPlanGoblins,
        "Sythis, Harvest's Hand" | "Ghen, Arcanum Weaver" | "Eutropia the Twice-Favored" => {
            Phrase::EchoesPlanEnchantments
        }
        "Kess, Dissident Mage" | "Muldrotha, the Gravetide" => Phrase::EchoesPlanGraveyard,
        "Emiel the Blessed" | "Derevi, Empyrial Tactician" | "Garna, the Bloodflame" => {
            Phrase::EchoesPlanReturns
        }
        "Thrun, the Last Troll" | "Samut, Voice of Dissent" | "Saryth, the Viper's Fang" => {
            Phrase::EchoesPlanCombat
        }
        "Vona, Butcher of Magan"
        | "Chevill, Bane of Monsters"
        | "Ertai Resurrected"
        | "Kaervek, the Spiteful" => Phrase::EchoesPlanControl,
        "Kambal, Consul of Allocation" | "Linden, the Steadfast Queen" => Phrase::EchoesPlanLife,
        "Birgi, God of Storytelling" => Phrase::EchoesPlanStorm,
        "Arvad the Cursed" => Phrase::EchoesPlanLegends,
        _ => return None,
    })
}
