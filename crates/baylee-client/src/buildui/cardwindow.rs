//! The card window (windows-b6 §A.3): one sheet over the builder for one
//! card, opened through two doors — the row (`Inspect`, focus on the
//! primary) and the picture (`PickPrint` / `PickRowPrint`, focus on the
//! printing strip). It replaced the card sheet and, in the builder, the
//! printing picker: the picture, the text in the player's language, the
//! printing, the counts and what can be done, on one pane.
//!
//! Nothing is drawn on the print (Scryfall image rules): every label of
//! ours stands beside it, the strip's ring outside its thumbnail.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::shellkit::surfaces::{self, SheetWidth};
use baylee_client_core::deckbuilder::{BuildField, Door, Picker, PoolCard};

/// The card window's own Tab order (modal, KEYBOARD §7.7 "card window").
pub(crate) const CARD: &str = "builder-card";

/// The stops, in Tab order.
pub(crate) const CARD_ORDER: TabOrder = TabOrder {
    name: CARD,
    stops: &[
        "flip",
        "printings",
        "set",
        "langs",
        "finishes",
        "any-finish",
        "main",
        "side",
        "commander",
        "close",
        "add-other",
        "add",
    ],
    modal: true,
};

const fn stop(id: &'static str) -> Stop {
    Stop::new(CARD, id)
}

/// The stop a door opens the window on (§1.7: `AutoFocus` by door).
pub(crate) fn first_stop(door: Door) -> &'static str {
    match door {
        Door::Row => "add",
        Door::Picture => "printings",
    }
}

/// The rules text the window shows, and whether it is the compiled English
/// Oracle standing in for the player's language (§A.3 "the text chain"):
/// the pool's text (the catalog's, or the Scryfall door's once it answered)
/// first, then the compiled Oracle, which is always there, offline too.
/// Never blank: a card with no text at all says so.
pub(crate) fn rules_text(card: &PoolCard, lang: Lang) -> (String, bool) {
    if !card.oracle_text.trim().is_empty() {
        return (card.oracle_text.clone(), false);
    }
    let compiled = baylee_cards::generated_oracle::ORACLE
        .get(card.index as usize)
        .map(|faces| faces.join("\n\n"))
        .unwrap_or_default();
    if compiled.trim().is_empty() {
        return (Phrase::NoRulesText.text(lang).to_string(), false);
    }
    (compiled, lang != Lang::En)
}

/// What the window is drawn from; a change to any of it is a redraw.
#[derive(Clone, PartialEq)]
pub(crate) struct WindowKey {
    picker: Picker,
    held: (u16, u16),
    commander: bool,
    card: Option<(String, String, String)>,
    back: bool,
    lang: Lang,
    set: String,
}

/// The window's key, or `None` while it is shut.
pub(crate) fn key(env: &Env) -> Option<WindowKey> {
    let deck = env.deck();
    let picker = deck.picker()?;
    let slot = picker.slot();
    Some(WindowKey {
        picker: picker.clone(),
        held: (
            deck.count_of(slot, Zone::Main),
            deck.count_of(slot, Zone::Side),
        ),
        commander: deck.is_commander(slot),
        card: deck
            .card(slot)
            .map(|c| (c.name.clone(), c.type_line.clone(), c.oracle_text.clone())),
        back: env.ui().window_back,
        lang: env.lang(),
        set: deck.buffer(BuildField::PickerSet).text().to_string(),
    })
}

/// The window over its scrim.
#[allow(clippy::too_many_lines)] // one window, read top to bottom
pub(crate) fn window(
    commands: &mut Commands,
    env: &Env,
    assets: Option<&AssetServer>,
    cards: Option<&mut UiCards<'_>>,
) -> Option<Entity> {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let picker = deck.picker()?;
    let slot = picker.slot();
    let card = deck.card(slot)?;
    let phone = m.frame == Frame::Phone;
    let print_w = if phone { 186.0 } else { 308.0 };

    // ---- the left column: the print, Flip, the strip, the set field
    let left = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: m.px(8.0),
                width: px_fixed(print_w),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let print = picture(commands, env, picker, card, print_w, assets, cards);
    commands.entity(left).add_child(print);
    let two_faced = picker
        .current()
        .is_some_and(baylee_client_core::deckbuilder::Printing::has_back_image);
    if two_faced {
        let flip = controls::button(
            commands,
            kit,
            Phrase::CardFlip.text(lang),
            Weight::Secondary,
            Live::Yes,
            (!m.touch()).then_some("F"),
            (Press::Build(BuildPress::WindowFlip), stop("flip")),
        );
        commands.entity(left).add_child(flip);
    }
    let strip = strip(commands, env, picker);
    commands.entity(left).add_child(strip);
    if picker.from_catalog() && picker.sets().len() > 1 {
        let sets = super::print_picker::set_search(
            commands,
            kit.fonts,
            env.lobby_metrics(),
            lang,
            deck,
            picker,
        );
        commands.entity(sets).insert(stop("set"));
        commands.entity(left).add_child(sets);
    }

    // ---- the right column: cost, type, text, the sections
    let right = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: m.px(8.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut top = Vec::new();
    if let Some(cost) =
        crate::manaui::spawn_cost_or_text(commands, kit.fonts, &card.mana_cost, m.text * 1.1)
    {
        top.push(cost);
    }
    let type_line = match &card.stats {
        Some(stats) if !stats.is_empty() => format!("{} \u{b7} {stats}", card.type_line),
        _ => card.type_line.clone(),
    };
    top.push(cell(
        commands,
        kit,
        &type_line,
        m.small,
        tokens::MUTED,
        false,
    ));
    let top = line(commands, kit, &top);
    commands.entity(right).add_child(top);
    let (text, english) = rules_text(card, lang);
    let rules = commands
        .spawn((
            Text::new(text),
            crate::hud::tf_serif(kit.fonts, if phone { 13.0 } else { 14.0 }, 400),
            TextColor(tokens::INK),
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(right).add_child(rules);
    if english {
        let tag = cell(
            commands,
            kit,
            Phrase::CardEnglishText.text(lang),
            m.small,
            tokens::MUTED,
            false,
        );
        commands.entity(right).add_child(tag);
    }
    let printing = printing(commands, env, picker);
    commands.entity(right).add_child(printing);
    let counts = counts(commands, env, slot);
    commands.entity(right).add_child(counts);
    if let Some((mark, ink)) = coverage_mark(card.coverage) {
        let said = mark.text(lang);
        let why = match &card.note {
            Some(note) => format!("{said}: {note}"),
            None => Phrase::NotAsPrinted.fill(lang, &[said]),
        };
        let coverage = commands
            .spawn((
                Text::new(why),
                tf(kit.fonts, m.small),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(right).add_child(coverage);
    }

    let pane = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: m.px(24.0),
                align_items: AlignItems::FlexStart,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(pane).add_children(&[left, right]);

    let foot = foot(commands, env, picker, slot, card.commander);
    let title = if card.english_name.is_empty() || card.english_name == card.name {
        card.name.clone()
    } else {
        format!("{} \u{b7} {}", card.english_name, card.name)
    };
    let surface = surfaces::sheet_box(commands, kit, SheetWidth::Large, &title, &[pane], &foot);
    // A press inside the window is not a press on the scrim behind it.
    commands
        .entity(surface)
        .insert(Press::Shared(crate::lobby::SharedPress::PickerNothing));
    let scrim = surfaces::sheet(commands, surface);
    // Click outside closes (§A.3 "Pointer and touch").
    commands
        .entity(scrim)
        .insert(Press::Build(BuildPress::PickerClose));
    Some(scrim)
}

/// The chosen printing, whole; its text face where no picture is known or
/// the player reads text, and under the picture until its texture lands —
/// never an empty box.
#[allow(clippy::too_many_lines)] // one section, top to bottom
fn picture(
    commands: &mut Commands,
    env: &Env,
    picker: &Picker,
    card: &PoolCard,
    width: f32,
    assets: Option<&AssetServer>,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let lang = env.lang();
    let height = width / baylee_client_core::layout::CARD_ASPECT;
    let reads_text = env.ui().reads_text;
    let frame = commands
        .spawn((
            Role::Art,
            crate::flip::Flip {
                turn: if env.ui().window_back { 1.0 } else { 0.0 },
                held: env.ui().window_back,
            },
            Node {
                width: px_fixed(width),
                height: px_fixed(height),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(12.0)),
                ..default()
            },
            crate::hud::soft_shadow(),
            Pickable::IGNORE,
        ))
        .id();
    let Some(cards) = cards else {
        return frame;
    };
    let finish = treatment(picker.finish());
    // The text face first: it is what stands there while a picture loads,
    // and what stands there when there is none.
    let face = crate::face::of_pool(card);
    let laid = crate::face::UiFace::lay(
        &face,
        lang,
        width,
        crate::face::Detail::Full,
        &crate::face::Widths::of(None),
        0,
    );
    let look = crate::cardmat::CardLook::back(finish)
        .faced(crate::face::table_color(face.colors), laid.word);
    let node = commands
        .spawn((
            MaterialNode(cards.get(look, None)),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                border_radius: BorderRadius::all(px_fixed(12.0)),
                ..default()
            },
            crate::flip::Side::Front,
            Visibility::Inherited,
            Pickable::IGNORE,
        ))
        .id();
    crate::face::spawn_ui(commands, node, lang, &face, &laid, env.kit.fonts);
    commands.entity(frame).add_child(node);
    if reads_text {
        return frame;
    }
    let entry = |p: &baylee_client_core::deckbuilder::Printing| baylee_view::PrintEntry {
        scryfall_id: p.scryfall_id.clone(),
        lang: p.lang.clone(),
        finish: baylee_view::Finish::Normal,
    };
    let url = picker.current().and_then(|p| {
        baylee_client_core::images::image_url(
            &entry(p),
            baylee_client_core::images::Face::Front,
            baylee_client_core::images::ArtSize::Normal,
        )
    });
    let (Some(url), Some(assets)) = (url, assets) else {
        return frame;
    };
    let back = picker
        .current()
        .filter(|p| p.has_back_image())
        .and_then(|p| {
            baylee_client_core::images::image_url(
                &entry(p),
                baylee_client_core::images::Face::Back,
                baylee_client_core::images::ArtSize::Normal,
            )
        });
    let mut sides = vec![(url, crate::flip::Side::Front)];
    if let Some(back) = back {
        sides.push((back, crate::flip::Side::Back));
    }
    for (url, side) in sides {
        let material = cards.preview(&url, finish, assets.load(url.clone()));
        commands.entity(frame).with_child((
            MaterialNode(material),
            side,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                border_radius: BorderRadius::all(px_fixed(12.0)),
                ..default()
            },
            if side == crate::flip::Side::Back {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            },
            Pickable::IGNORE,
        ));
    }
    frame
}

/// The printing strip: five thumbnails a page, the chosen one ringed
/// outside its print, `◂ n/m ▸`; one Tab stop (← → inside it step).
#[allow(clippy::too_many_lines)] // one section, top to bottom
fn strip(commands: &mut Commands, env: &Env, picker: &Picker) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: m.px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let thumbs = commands
        .spawn((
            Node {
                column_gap: px_fixed(6.0),
                padding: UiRect::all(px_fixed(4.0)),
                border_radius: BorderRadius::all(px_fixed(6.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            // Enter on the strip is the window's primary (§A.3 keyboard).
            (
                Press::Build(if picker.replacing() {
                    BuildPress::PickerConfirm
                } else {
                    BuildPress::WindowAdd(picker.zone())
                }),
                stop("printings"),
            ),
        ))
        .id();
    let start = (picker.at() / 5) * 5;
    for (at, print) in picker.visible().iter().enumerate().skip(start).take(5) {
        let thumb = crate::lobby::thumbnails::spawn(
            commands,
            &crate::lobby::HoverCard {
                url: baylee_client_core::images::image_url(
                    &baylee_view::PrintEntry {
                        scryfall_id: print.scryfall_id.clone(),
                        lang: print.lang.clone(),
                        finish: baylee_view::Finish::Normal,
                    },
                    baylee_client_core::images::Face::Front,
                    baylee_client_core::images::ArtSize::Small,
                ),
                back_url: None,
                finish: treatment(picker.finish()),
                index: Some(picker_index(env, picker)),
            },
        );
        commands.entity(thumb).insert((
            Press::Build(BuildPress::PickerGo(at)),
            Pickable::default(),
            Node {
                width: px_fixed(48.0),
                height: px_fixed(67.0),
                flex_shrink: 0.0,
                ..default()
            },
            // The ring stands outside the print, never on it.
            Outline::new(
                px_fixed(2.0),
                px_fixed(2.0),
                if at == picker.at() {
                    tokens::ACCENT
                } else {
                    Color::NONE
                },
            ),
        ));
        commands.entity(thumbs).add_child(thumb);
    }
    commands.entity(column).add_child(thumbs);
    let pages = picker.len().div_ceil(5).max(1);
    if pages > 1 || picker.len() > 1 {
        let back = controls::button(
            commands,
            kit,
            "\u{25c2}",
            Weight::Ghost,
            Live::Yes,
            None,
            Press::Build(BuildPress::PickerStep(-1)),
        );
        let said = cell(
            commands,
            kit,
            &format!("{}/{}", picker.at() + 1, picker.len()),
            m.small,
            tokens::MUTED,
            false,
        );
        let forward = controls::button(
            commands,
            kit,
            "\u{25b8}",
            Weight::Ghost,
            Live::Yes,
            None,
            Press::Build(BuildPress::PickerStep(1)),
        );
        let row = line(commands, kit, &[back, said, forward]);
        commands.entity(column).add_child(row);
    }
    if !picker.loading() && !picker.from_catalog() {
        // Offline, or a gateway with no catalog: a sentence, not grey chips.
        let more = surfaces::prose(commands, kit, Phrase::CardMorePrintings.text(lang), true);
        commands.entity(column).add_child(more);
    } else if picker.loading() {
        let spin = crate::card_loading::spinner(commands, 18.0);
        commands.entity(column).add_child(spin);
    }
    column
}

fn picker_index(env: &Env, picker: &Picker) -> u32 {
    env.deck().card(picker.slot()).map_or(0, |c| c.index)
}

/// PRINTING: the chosen one in words, its language and finish.
#[allow(clippy::too_many_lines)] // one section, top to bottom
fn printing(commands: &mut Commands, env: &Env, picker: &Picker) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let section = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: m.px(6.0),
                border: UiRect::top(px_fixed(1.0)),
                padding: UiRect::top(m.px(8.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    let head = cell(
        commands,
        kit,
        Phrase::CardHeadPrinting.text(lang),
        m.small,
        tokens::MUTED,
        true,
    );
    commands.entity(section).add_child(head);
    if let Some(current) = picker.current() {
        let mut words = Vec::new();
        if !current.set.is_empty() {
            let mut set = current.set.to_uppercase();
            if !current.collector_number.is_empty() {
                set = format!("{set} #{}", current.collector_number);
            }
            words.push(set);
        }
        if !current.artist.is_empty() {
            words.push(current.artist.clone());
        }
        if !current.released_at.is_empty() {
            words.push(current.released_at.clone());
        }
        if !words.is_empty() {
            let said = cell(
                commands,
                kit,
                &words.join(" \u{b7} "),
                m.small,
                tokens::INK,
                false,
            );
            commands.entity(section).add_child(said);
        }
    }
    // A language with no printing is not a chip; one language is no choice.
    if picker.from_catalog() && picker.langs().len() > 1 {
        let mut chips = vec![cell(
            commands,
            kit,
            Phrase::CardLanguage.text(lang),
            m.small,
            tokens::MUTED,
            false,
        )];
        let mut order: Vec<(usize, &String)> = picker.langs().iter().enumerate().collect();
        let mine = lang.code();
        order.sort_by_key(|(_, code)| {
            (
                code.as_str() != "en",
                code.as_str() != mine,
                (*code).clone(),
            )
        });
        for (i, code) in order {
            chips.push(controls::chip(
                commands,
                kit,
                &code.to_uppercase(),
                picker.lang() == Some(code.as_str()),
                None,
                false,
                (
                    Press::Build(BuildPress::PickerLang(Some(i))),
                    Stop::item(CARD, "langs", u8::try_from(i).unwrap_or(u8::MAX)),
                ),
            ));
        }
        let row = wrap(commands, kit, &chips);
        commands.entity(section).add_child(row);
    }
    let offered = picker.finishes();
    let mut chips = vec![cell(
        commands,
        kit,
        Phrase::CardFinish.text(lang),
        m.small,
        tokens::MUTED,
        false,
    )];
    for (n, (finish, label)) in [
        (Finish::Normal, Phrase::FinishPlain),
        (Finish::Foil, Phrase::FinishFoil),
        (Finish::Etched, Phrase::FinishEtched),
        (Finish::Holographic, Phrase::FinishHolographic),
        (Finish::Glitter, Phrase::FinishGlitter),
        (Finish::Galaxy, Phrase::FinishGalaxy),
    ]
    .into_iter()
    .enumerate()
    {
        // What the printing sold, or everything under "any finish".
        if !offered.contains(&finish) {
            continue;
        }
        chips.push(controls::chip(
            commands,
            kit,
            label.text(lang),
            picker.finish() == finish,
            None,
            false,
            (
                Press::Build(BuildPress::PickerFinish(finish)),
                Stop::item(CARD, "finishes", u8::try_from(n).unwrap_or(0)),
            ),
        ));
    }
    chips.push(controls::chip(
        commands,
        kit,
        Phrase::CardAnyFinish.text(lang),
        picker.force_finish(),
        None,
        false,
        (
            Press::Build(BuildPress::PickerForceFinish),
            stop("any-finish"),
        ),
    ));
    let row = wrap(commands, kit, &chips);
    commands.entity(section).add_child(row);
    section
}

/// IN THIS DECK: the main and sideboard steppers, written at once.
fn counts(commands: &mut Commands, env: &Env, slot: usize) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let head = cell(
        commands,
        kit,
        Phrase::CardHeadInDeck.text(lang),
        m.small,
        tokens::MUTED,
        true,
    );
    let mut parts = vec![head];
    for (zone, label, id) in [
        (Zone::Main, Phrase::CardMain, "main"),
        (Zone::Side, Phrase::CardSide, "side"),
    ] {
        parts.push(cell(
            commands,
            kit,
            label.text(lang),
            m.small,
            tokens::INK,
            false,
        ));
        let stepper = controls::stepper(
            commands,
            kit,
            &deck.count_of(slot, zone).to_string(),
            (
                Press::Build(BuildPress::WindowStep(zone, false)),
                Stop::item(CARD, id, 0),
            ),
            (
                Press::Build(BuildPress::WindowStep(zone, true)),
                Stop::item(CARD, id, 1),
            ),
        );
        parts.push(stepper);
    }
    let row = wrap(commands, kit, &parts);
    commands.entity(row).insert((
        Node {
            width: Val::Percent(100.0),
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            column_gap: m.px(12.0),
            row_gap: m.px(6.0),
            border: UiRect::top(px_fixed(1.0)),
            padding: UiRect::top(m.px(8.0)),
            ..default()
        },
        BorderColor::all(tokens::BORDER),
    ));
    row
}

/// The footer: Set as commander, Close, Add to the other zone, the primary.
fn foot(
    commands: &mut Commands,
    env: &Env,
    picker: &Picker,
    slot: usize,
    commander: bool,
) -> Vec<Entity> {
    let kit = env.kit;
    let lang = env.lang();
    let deck = env.deck();
    let keys = !kit.m.touch();
    let mut out = Vec::new();
    if commander {
        let leading = deck.is_commander(slot);
        out.push(controls::button(
            commands,
            kit,
            if leading {
                Phrase::CardIsCommander
            } else {
                Phrase::CardSetCommander
            }
            .text(lang),
            Weight::Secondary,
            Live::Yes,
            keys.then_some("C"),
            (
                Press::Build(if leading {
                    BuildPress::ClearCommander
                } else {
                    BuildPress::SetCommander(slot)
                }),
                stop("commander"),
            ),
        ));
        out.push(spring(commands));
    }
    out.push(controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(lang),
        Weight::Secondary,
        Live::Yes,
        keys.then_some("Esc"),
        (Press::Build(BuildPress::PickerClose), stop("close")),
    ));
    let other = match picker.zone() {
        Zone::Main => Zone::Side,
        Zone::Side => Zone::Main,
    };
    out.push(controls::button(
        commands,
        kit,
        match other {
            Zone::Side => Phrase::BuildAddToSide,
            Zone::Main => Phrase::BuildAddToMain,
        }
        .text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        (
            Press::Build(BuildPress::WindowAdd(other)),
            stop("add-other"),
        ),
    ));
    // The primary is the door's: Apply printing through a picture door on a
    // deck row, Add to deck everywhere else (the chosen print rides along).
    let apply = picker.replacing() && picker.door() == Door::Picture;
    out.push(controls::button(
        commands,
        kit,
        if apply {
            Phrase::CardApplyPrinting
        } else {
            Phrase::CardAddToDeck
        }
        .text(lang),
        Weight::Primary,
        Live::Yes,
        keys.then_some("\u{21b5}"),
        (
            Press::Build(if apply {
                BuildPress::PickerConfirm
            } else {
                BuildPress::WindowAdd(picker.zone())
            }),
            stop("add"),
        ),
    ));
    out
}

/// A row that wraps.
fn wrap(commands: &mut Commands, kit: Kit, children: &[Entity]) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(6.0),
                row_gap: kit.m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(children);
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(index: u32, text: &str) -> PoolCard {
        PoolCard {
            index,
            name: "x".into(),
            oracle_text: text.into(),
            ..PoolCard::default()
        }
    }

    /// The text chain (§A.3): the pool's text first; the compiled English
    /// Oracle when it has none, tagged only where the player reads another
    /// language; never blank.
    #[test]
    fn the_text_chain_is_never_blank_and_tags_only_the_fallback() {
        let (text, english) = rules_text(&card(0, "Fliegend."), Lang::De);
        assert_eq!(text, "Fliegend.");
        assert!(!english, "the pool's own text is not a fallback");
        let index = baylee_cards::generated_oracle::ORACLE
            .iter()
            .position(|faces| faces.iter().any(|f| !f.trim().is_empty()))
            .expect("a card with text");
        let index = u32::try_from(index).expect("an index");
        let (text, english) = rules_text(&card(index, ""), Lang::De);
        assert!(!text.trim().is_empty(), "offline, the compiled Oracle");
        assert!(english, "and the German player is told it is English");
        let (_, english) = rules_text(&card(index, ""), Lang::En);
        assert!(!english, "no tag for an English reader");
        let (text, _) = rules_text(&card(u32::MAX, ""), Lang::En);
        assert!(!text.trim().is_empty(), "a card with no text says so");
    }

    /// The doors' first stops are in the window's Tab order.
    #[test]
    fn each_door_opens_on_a_stop_of_the_window() {
        for door in [Door::Row, Door::Picture] {
            assert!(CARD_ORDER.stops.contains(&first_stop(door)));
        }
    }
}
