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
        .insert((
            BorderColor::all(tokens::GOLD.with_alpha(0.38)),
            BackgroundColor(tokens::CONTROL.with_alpha(0.86)),
        ))
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.min_width = px(0);
            node.row_gap = kit.m.px(10.0);
            node.padding = UiRect::all(kit.m.px(16.0));
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
        lang,
    );
    let image = crate::lobby::thumbnails::spawn_sized(
        commands,
        &hover,
        kit.m.px(124.0),
        kit.m.px(173.0),
        kit.m.px(9.0),
    );
    commands
        .entity(image)
        .insert((surfaces::elevation_card(), hover, Pickable::default()));
    commands.entity(choice).add_child(image);
    let title = controls::label(commands, kit, &name, kit.m.text, tokens::INK);
    commands
        .entity(title)
        .insert((
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextLayout::justify(Justify::Center),
        ))
        .entry::<Node>()
        .and_modify(move |mut node| node.min_height = kit.m.px(44.0));
    commands.entity(choice).add_child(title);

    let plan =
        strategy(&words::card(commander, Lang::En)).unwrap_or_else(|| fallback_strategy(commander));
    let band = row(commands, choice, kit);
    commands
        .entity(band)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_direction = FlexDirection::Column;
            node.align_items = AlignItems::Center;
            node.min_height = kit.m.px(36.0);
        });
    let plan_label = controls::label(commands, kit, plan.text(lang), kit.m.small, tokens::GOLD);
    commands
        .entity(plan_label)
        .insert(TextLayout::justify(Justify::Center));
    commands.entity(band).add_child(plan_label);

    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(choice).add_child(spacer);
    starter_actions(commands, choice, kit, ui, cohort, number);
}

fn starter_actions(
    commands: &mut Commands,
    choice: Entity,
    kit: Kit,
    ui: &Ui,
    cohort: Uuid,
    number: u8,
) {
    let lang = lang(kit);
    let actions = row(commands, choice, kit);
    commands
        .entity(actions)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = percent(100);
            node.justify_content = JustifyContent::Center;
        });
    let inspect_action = Action::InspectStarter(cohort, number);
    let inspect = controls::button_compact(
        commands,
        kit,
        Some(glyph::LIBRARY),
        Phrase::EchoesDeckCheck.text(lang),
        Weight::Secondary,
        if !ui.model.mutating() && ui.confirmation.is_none() {
            Live::Yes
        } else {
            Live::No(Phrase::EchoesUnavailable.text(lang))
        },
        Press::Echoes(inspect_action),
    );
    commands
        .entity(inspect)
        .insert(super::super::super::keys::stop(inspect_action, ui));
    commands.entity(actions).add_child(inspect);

    let start_action = Action::Starter(cohort, number);
    let can_start =
        !ui.model.busy() && !ui.model.uncertain() && ui.choices.deck_unchanged(ui.model.run());
    let choose = controls::button_compact(
        commands,
        kit,
        Some(glyph::ARROW_RIGHT),
        Phrase::EchoesRunChoose.text(lang),
        Weight::Gold,
        if can_start && ui.confirmation.is_none() {
            Live::Yes
        } else {
            Live::No(Phrase::EchoesUnavailable.text(lang))
        },
        Press::Echoes(start_action),
    );
    commands
        .entity(choose)
        .insert(super::super::super::keys::stop(start_action, ui));
    commands.entity(actions).add_child(choose);
}

pub(super) fn inspect(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    scrolled: &Scrolled,
) {
    let Some((cohort, index)) = ui.starter_preview else {
        return;
    };
    let Some(publication) = ui.model.cohorts().iter().find(|entry| entry.id == cohort) else {
        return;
    };
    let Some(starter) = publication.starters.get(usize::from(index)) else {
        return;
    };
    let deck_row = |id, count| {
        let print = publication
            .catalog
            .card(id)
            .and_then(|card| card.run)
            .and_then(|run| run.presentation)
            .map_or_else(baylee_core::deckrow::PrintChoice::default, |art| {
                baylee_core::deckrow::PrintChoice {
                    scryfall_id: Some(art.printing.to_string()),
                    lang: Some("en".into()),
                    finish: Some(art.finish),
                    ..default()
                }
            });
        baylee_core::deckrow::Row {
            count,
            name: words::card(id, Lang::En),
            print: crate::towerprints::choice(&print, lang(kit)).into_owned(),
            note: None,
        }
        .to_string()
    };
    let mut body = Vec::new();
    let rows = starter
        .formation
        .as_ref()
        .map_or_default(|formation| vec![deck_row(formation.commander, 1)]);
    body.push(crate::lobby::decks::spawn_preview_section_container(
        commands,
        kit,
        &rows,
        crate::lobby::decks::DecksView::Compact,
    ));
    let rows: Vec<_> = starter
        .deck
        .iter()
        .map(|stack| deck_row(stack.card, u32::from(stack.copies)))
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
    let title = starter.formation.as_ref().map_or_else(
        || Phrase::EchoesDeck.text(lang(kit)).to_owned(),
        |formation| words::card(formation.commander, lang(kit)),
    );
    let surface = surfaces::sheet_box_listed(
        commands,
        kit,
        surfaces::SheetWidth::Large,
        &title,
        &body,
        &[close],
        (
            Scrollable(List::EchoesInspector),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::EchoesInspector))),
        ),
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

fn fallback_strategy(commander: baylee_core::ids::CardIndex) -> Phrase {
    use baylee_core::color::Color as ManaColor;
    let Some(def) = baylee_cards::by_index(commander) else {
        return Phrase::EchoesPlanLegends;
    };
    let id = def.color_identity;
    if id.contains(ManaColor::Blue) && id.contains(ManaColor::Red) {
        Phrase::EchoesPlanStorm
    } else if id.contains(ManaColor::Black) && id.contains(ManaColor::Green) {
        Phrase::EchoesPlanGraveyard
    } else if id.contains(ManaColor::White) && id.contains(ManaColor::Black) {
        Phrase::EchoesPlanLife
    } else if id.contains(ManaColor::Green) {
        Phrase::EchoesPlanCombat
    } else if id.contains(ManaColor::Blue) {
        Phrase::EchoesPlanControl
    } else if id.contains(ManaColor::Red) {
        Phrase::EchoesPlanStorm
    } else if id.contains(ManaColor::White) {
        Phrase::EchoesPlanLegends
    } else {
        Phrase::EchoesPlanControl
    }
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
