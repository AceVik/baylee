//! The pool side (§7): the search with its `?` syntax help and the filter
//! builder's gear, **one** chip row with the Filters disclosure, the count
//! line, and the list of compact rows; on a phone the search, *Filters · n*
//! and the count line.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::{FieldLook, FieldTail, List};
use baylee_client_core::deckbuilder::BuildField;

/// What the syntax popover offers (`docs/client.md` §"What the search box
/// understands"): an example a press inserts, and what it means.
pub(crate) const SYNTAX: [(&str, Phrase); 12] = [
    ("t:creature", Phrase::BuildSyntaxType),
    ("o:draw", Phrase::BuildSyntaxText),
    ("c:rg", Phrase::BuildSyntaxColour),
    ("id<=wu", Phrase::BuildSyntaxIdentity),
    ("mv<=3", Phrase::BuildSyntaxMana),
    ("pow>=4", Phrase::BuildSyntaxPower),
    ("m:{G}{G}", Phrase::BuildSyntaxCost),
    ("is:commander", Phrase::BuildSyntaxCommander),
    ("is:partial", Phrase::BuildSyntaxPartial),
    ("-t:land", Phrase::BuildSyntaxNot),
    ("!\"Llanowar Elves\"", Phrase::BuildSyntaxExact),
    ("t:elf or t:goblin", Phrase::BuildSyntaxOr),
];

/// Whether the search box has the caret (and so the keyboard's letters).
pub(crate) fn search_has_caret(env: &Env) -> bool {
    let deck = env.deck();
    env.ui().nav == Nav::Field
        && deck.focus() == BuildField::Search
        && deck.panel().is_none()
        && deck.picker().is_none()
}

/// Whether the pool's filters are read on a phone (search, *Filters · n*,
/// the count line), not on the chip row.
fn compact(env: &Env) -> bool {
    matches!(env.layout, Layout::Rail | Layout::PhoneSingle)
}

/// The pool panel and its holders.
pub(super) fn panel(commands: &mut Commands, env: &Env, holders: &mut retained::Holders) -> Entity {
    let kit = env.kit;
    let grow = if env.layout == Layout::Columns {
        3.0
    } else {
        1.0
    };
    let panel = super::panel(commands, kit, grow, None);
    commands.entity(panel).insert(crate::lobby::dock::Dock(6));
    holders.toolbar = Some(holder(
        commands,
        panel,
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: kit.m.px(8.0),
            flex_shrink: 0.0,
            ..default()
        },
    ));
    holders.pool_list = Some(holder(
        commands,
        panel,
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_basis: px_fixed(0.0),
            min_height: px_fixed(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
    // The key hints, where a pointer is the input and there is room (§2.7).
    if env.layout == Layout::Columns && !kit.m.touch() {
        let hints = hints(commands, env);
        commands.entity(panel).add_child(hints);
    }
    holders.pool_over = Some(holder(
        commands,
        panel,
        Node {
            position_type: PositionType::Absolute,
            left: px_fixed(0.0),
            top: px_fixed(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    ));
    panel
}

/// The footer of key hints: `↑↓ move · ↵ add · ⇧↵ other list · / search ·
/// Space card`.
fn hints(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let lang = env.lang();
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: kit.m.px(6.0),
                flex_shrink: 0.0,
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (keys, what) in [
        ("\u{2191}\u{2193}", Phrase::BuildHintMove),
        ("Enter", Phrase::BuildHintAdd),
        ("Shift+Enter", Phrase::BuildHintOther),
        ("/", Phrase::BuildHintSearch),
        ("Space", Phrase::BuildHintPreview),
    ] {
        if let Some(cap) = controls::key_cap(commands, kit, keys) {
            commands.entity(cap).insert(Node {
                padding: UiRect::axes(kit.m.px(5.0), px_fixed(1.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(4.0)),
                ..default()
            });
            commands.entity(row).add_child(cap);
        }
        let said = words(commands, kit, what.text(lang), kit.m.small, tokens::MUTED);
        commands.entity(row).add_child(said);
    }
    row
}

/// The toolbar: the search, the chip row, the count line.
#[allow(clippy::too_many_lines)] // the pool's controls, read left to right, top to bottom
pub(super) fn toolbar(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let ui = env.ui();

    // ---- the search box, its syntax help, the filter builder's gear
    let search = crate::lobby::text_field(
        commands,
        kit.fonts,
        env.lobby_metrics(),
        "",
        &FieldLook {
            buffer: deck.buffer(BuildField::Search),
            focused: search_has_caret(env),
            mask: None,
            press: Press::Build(BuildPress::FocusBuild(BuildField::Search)),
            lead: Some(crate::hud::glyph::MAGNIFIER),
            hint: Some(Phrase::SearchCards.text(lang)),
            tail: Some(FieldTail {
                glyph: mark::HELP,
                press: Press::Build(BuildPress::ToggleSyntax),
                lit: ui.syntax,
            }),
        },
    );
    commands.entity(search).insert((
        Node {
            flex_grow: 1.0,
            flex_basis: px_fixed(0.0),
            min_width: px_fixed(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        stop("search"),
    ));
    let mut first = vec![search];
    if compact(env) {
        first.push(filters_button(commands, env));
    }
    first.push(icon_button(
        commands,
        kit,
        crate::hud::glyph::GEAR,
        deck.panel().is_some(),
        (Press::Build(BuildPress::ToggleFilterPanel), stop("gear")),
    ));
    let top = line(commands, kit, &first);
    commands.entity(holder).add_child(top);

    // The filter builder the gear opens: what the box holds, taken apart.
    if let Some(built) = deck.panel() {
        let also = chips_in_words(deck, lang);
        let rows = crate::filterui::build(
            commands,
            kit.fonts,
            built,
            baylee_client_core::cardquery::Surface::POOL,
            also.as_deref(),
            lang,
            crate::filterui::Register::LOBBY,
        );
        commands.entity(holder).add_child(rows);
    }

    // ---- the one chip row (not on a phone, where Filters · n holds it)
    if !compact(env) {
        let row = chip_row(commands, env);
        commands.entity(holder).add_child(row);
    }

    // ---- choosing a commander narrows the list to who may lead
    if let Some(partner) = env.state.commander_pick {
        let hint = words(
            commands,
            kit,
            if partner {
                Phrase::PartnerHint
            } else {
                Phrase::CommanderHint
            }
            .text(lang),
            m.small,
            tokens::MUTED,
        );
        commands.entity(hint).insert(Node {
            flex_shrink: 1.0,
            min_width: px_fixed(0.0),
            ..default()
        });
        commands.entity(hint).remove::<TextLayout>();
        let done = link(
            commands,
            kit,
            Phrase::DoneChoosing.text(lang),
            Press::Build(BuildPress::CancelCommanderPick),
        );
        let row = line(commands, kit, &[hint, done]);
        commands.entity(holder).add_child(row);
    }

    // ---- the count line
    let said = count_line(env);
    let count = cell(commands, kit, &said, m.small, tokens::MUTED, false);
    commands.entity(holder).add_child(count);
}

/// "412 of 3322 cards · also narrowing: colour identity ≤ WUG" — on a phone
/// with every chip's words, since there is no chip row to read them on.
fn count_line(env: &Env) -> String {
    let deck = env.deck();
    let lang = env.lang();
    if !deck.loaded() {
        return Phrase::LoadingPool.text(lang).to_string();
    }
    let shown = shown(env).len();
    let mut said = Phrase::PoolTally.fill(
        lang,
        &[
            &deck.results().len().to_string(),
            &deck.pool().len().to_string(),
            &if shown < deck.results().len() {
                Phrase::PoolNarrow.fill(lang, &[&shown.to_string()])
            } else {
                String::new()
            },
        ],
    );
    if let Some(also) = chips_in_words(deck, lang) {
        said.push_str(" \u{b7} ");
        if compact(env) {
            said.push_str(&also);
        } else {
            said.push_str(&Phrase::BuildAlsoNarrowing.fill(lang, &[&also]));
        }
    }
    said
}

/// The results the list shows: all of them, or while a commander is being
/// chosen the ones who may lead.
pub(crate) fn shown(env: &Env) -> Vec<usize> {
    let deck = env.deck();
    deck.results()
        .iter()
        .copied()
        .filter(|slot| match env.state.commander_pick {
            Some(true) => deck.can_partner(*slot),
            Some(false) => deck.card(*slot).is_some_and(|card| card.commander),
            None => true,
        })
        .collect()
}

/// The chip row: colours · the active chips · Filters · Playable only ·
/// Sort · Clear.
fn chip_row(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: m.px(6.0),
                row_gap: m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut kids = colour_toggles(commands, env);
    if let Some(kind) = deck.kind() {
        let name = KINDS
            .iter()
            .find(|(k, _)| *k == kind)
            .map_or(kind, |(_, p)| p.text(lang));
        kids.push(controls::chip(
            commands,
            kit,
            name,
            true,
            None,
            true,
            (
                Press::Build(BuildPress::SetKind(None)),
                Stop::item(BUILDER, "chips", 0),
            ),
        ));
    }
    if let Some(cmc) = deck.cmc() {
        kids.push(controls::chip(
            commands,
            kit,
            &mana_value_words(cmc, lang),
            true,
            None,
            true,
            (
                Press::Build(BuildPress::ClearCmc),
                Stop::item(BUILDER, "chips", 1),
            ),
        ));
    }
    kids.push(filters_button(commands, env));
    kids.push(spring(commands));
    kids.push(controls::chip(
        commands,
        kit,
        Phrase::PlayableOnly.text(lang),
        deck.playable_only(),
        None,
        false,
        (Press::Build(BuildPress::TogglePlayable), stop("playable")),
    ));
    kids.push(sort_chip(commands, env));
    if deck.filtered() {
        kids.push(link(
            commands,
            kit,
            Phrase::ClearFilters.text(lang),
            (Press::Build(BuildPress::ClearFilters), stop("clear")),
        ));
    }
    commands.entity(row).add_children(&kids);
    row
}

/// "Mana value 3", "Mana value 7+".
fn mana_value_words(cmc: u32, lang: Lang) -> String {
    if cmc as usize + 1 >= CURVE_BUCKETS {
        Phrase::BuildManaValueUp.fill(lang, &[&cmc.to_string()])
    } else {
        Phrase::BuildManaValue.fill(lang, &[&cmc.to_string()])
    }
}

/// The six colour discs of the identity filter (W U B R G C), letters in
/// the discs so colour is never the only carrier (§2.4).
fn colour_toggles(commands: &mut Commands, env: &Env) -> Vec<Entity> {
    let kit = env.kit;
    let m = kit.m;
    let deck = env.deck();
    let side = m.scaled(26.0).max(20.0);
    COLORS
        .iter()
        .enumerate()
        .map(|(i, (letter, _))| {
            let on = deck.colors().contains(letter);
            let disc = commands
                .spawn((
                    Role::Toggle,
                    Node {
                        width: px_fixed(side),
                        height: px_fixed(side),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(px_fixed(if on { 2.0 } else { 1.0 })),
                        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                        ..default()
                    },
                    BackgroundColor(if on {
                        mana_tone(*letter)
                    } else {
                        tokens::CONTROL
                    }),
                    BorderColor::all(if on { tokens::ACCENT } else { tokens::BORDER }),
                ))
                .id();
            let letter_ink = if on {
                crate::shellkit::tokens::INK_ON_LIGHT
            } else {
                tokens::MUTED
            };
            let said = crate::manaui::spawn_mark_on(commands, kit.fonts, *letter, side, letter_ink);
            commands.entity(disc).add_child(said);
            controls::hit(
                commands,
                kit,
                disc,
                (
                    Press::Build(BuildPress::ToggleColor(*letter)),
                    Stop::item(BUILDER, "colours", u8::try_from(i).unwrap_or(0)),
                ),
            )
        })
        .collect()
}

/// *Filters* with a caret and the count of rail filters in force; on a
/// phone it opens the filters as a sheet.
fn filters_button(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let deck = env.deck();
    let active = u32::from(deck.kind().is_some()) + u32::from(deck.cmc().is_some());
    controls::chip(
        commands,
        kit,
        Phrase::BuildFilters.text(env.lang()),
        env.ui().rail,
        (active > 0).then_some(active),
        false,
        (Press::Build(BuildPress::ToggleRail), stop("filters")),
    )
}

/// The sort chip: the order's glyph and its name; a press takes the next.
fn sort_chip(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let height = if m.touch() { 40.0 } else { m.scaled(32.0) };
    let face = commands
        .spawn((
            Role::Chip,
            Node {
                min_height: px_fixed(height),
                padding: UiRect::axes(m.px(12.0), px_fixed(0.0)),
                column_gap: m.px(6.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    let sign = glyph(commands, kit, mark::SORT, m.small * 0.9, tokens::MUTED);
    let said = words(
        commands,
        kit,
        env.deck().sort().label().text(env.lang()),
        m.small,
        tokens::INK,
    );
    commands.entity(face).add_children(&[sign, said]);
    controls::hit(
        commands,
        kit,
        face,
        (Press::Build(BuildPress::CycleSort), stop("sort")),
    )
}

/// The list: skeleton rows while the pool is on its way, then the rows.
pub(super) fn list(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let deck = env.deck();
    if !deck.loaded() {
        let rows = crate::shellkit::states::skeleton(commands, kit, 6);
        commands.entity(holder).add_child(rows);
        return;
    }
    let results = shown(env);
    if results.is_empty() {
        let said = crate::shellkit::states::empty(
            commands,
            kit,
            crate::hud::glyph::MAGNIFIER,
            Phrase::NothingMatches.text(env.lang()),
            None,
        );
        commands.entity(holder).add_child(said);
        return;
    }
    let metrics = env.lobby_metrics();
    let list = crate::lobby::scroller(commands, metrics, List::Pool, env.scrolled.get(List::Pool));
    commands.entity(list).insert(stop("pool"));
    crate::lobby::scrollbars::attach(commands, holder, list, metrics);
    let content = virtual_rows::pool(commands, env, results);
    commands.entity(list).add_child(content);
}

/// The pool's overlays: the Filters rail over its left edge, the syntax
/// popover under the search.
pub(super) fn over(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let ui = env.ui();
    if ui.rail && !compact(env) {
        let rail = commands
            .spawn((
                Role::Opaque,
                Node {
                    position_type: PositionType::Absolute,
                    left: px_fixed(0.0),
                    top: px_fixed(0.0),
                    bottom: px_fixed(0.0),
                    width: m.px(320.0),
                    max_width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px_fixed(m.gap),
                    padding: UiRect::all(px_fixed(m.pad)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(tokens::OPAQUE),
                BorderColor::all(tokens::BORDER),
                GlobalZIndex(tokens::z::POPOVER),
            ))
            .id();
        let head = rail_head(commands, env);
        commands.entity(rail).add_child(head);
        let body = filters_body(commands, env);
        commands.entity(rail).add_children(&body);
        commands.entity(holder).add_child(rail);
    }
    if ui.syntax {
        let pop = syntax(commands, env);
        commands.entity(pop).insert(Node {
            position_type: PositionType::Absolute,
            top: px_fixed(m.pad + m.control + m.scaled(6.0)),
            right: px_fixed(m.pad),
            max_width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: m.px(2.0),
            padding: UiRect::all(px_fixed(m.pad)),
            border: UiRect::all(px_fixed(1.0)),
            border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
            ..default()
        });
        commands.entity(holder).add_child(pop);
    }
}

/// The rail's head: its name and its close.
pub(crate) fn rail_head(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let title = words(
        commands,
        kit,
        Phrase::BuildFilters.text(env.lang()),
        kit.m.head,
        tokens::INK,
    );
    let gap = spring(commands);
    let close = icon_button(
        commands,
        kit,
        mark::CLOSE,
        false,
        (Press::Build(BuildPress::ToggleRail), stop("rail")),
    );
    line(commands, kit, &[title, gap, close])
}

/// The rail's sections: colour, card type, mana value, what to show, and
/// Clear. The pool carries no rarity, set or keyword facts, so the rail
/// offers none (principle 5; they are round 2, §15).
pub(crate) fn filters_body(commands: &mut Commands, env: &Env) -> Vec<Entity> {
    let kit = env.kit;
    let lang = env.lang();
    let deck = env.deck();
    let mut out = Vec::new();
    let wrap = |commands: &mut Commands, kids: &[Entity]| {
        let row = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: kit.m.px(6.0),
                    row_gap: kit.m.px(6.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_children(kids);
        row
    };
    if compact(env) {
        out.push(caption(commands, kit, Phrase::FilterKeyIdentity.text(lang)));
        let discs = colour_toggles(commands, env);
        out.push(wrap(commands, &discs));
    }
    out.push(caption(commands, kit, Phrase::BuildRailType.text(lang)));
    let kinds: Vec<Entity> = KINDS
        .iter()
        .map(|(kind, name)| {
            let on = deck.kind() == Some(*kind);
            controls::chip(
                commands,
                kit,
                name.text(lang),
                on,
                None,
                false,
                Press::Build(BuildPress::SetKind(Some(*kind))),
            )
        })
        .collect();
    out.push(wrap(commands, &kinds));
    out.push(caption(commands, kit, Phrase::BuildRailMana.text(lang)));
    let values: Vec<Entity> = (0..u32::try_from(CURVE_BUCKETS).unwrap_or(8))
        .map(|cmc| {
            let last = cmc as usize + 1 == CURVE_BUCKETS;
            let label = if last {
                format!("{cmc}+")
            } else {
                cmc.to_string()
            };
            controls::chip(
                commands,
                kit,
                &label,
                deck.cmc() == Some(cmc),
                None,
                false,
                Press::Build(BuildPress::SetCmc(cmc)),
            )
        })
        .collect();
    out.push(wrap(commands, &values));
    out.push(caption(commands, kit, Phrase::BuildRailShow.text(lang)));
    let playable = controls::chip(
        commands,
        kit,
        Phrase::PlayableOnly.text(lang),
        deck.playable_only(),
        None,
        false,
        Press::Build(BuildPress::TogglePlayable),
    );
    out.push(wrap(commands, &[playable]));
    if deck.filtered() {
        let clear = link(
            commands,
            kit,
            Phrase::ClearFilters.text(lang),
            Press::Build(BuildPress::ClearFilters),
        );
        out.push(clear);
    }
    out
}

/// The syntax popover: each example, what it means; a press inserts it.
fn syntax(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let pop = commands
        .spawn((
            Role::Opaque,
            Node::default(),
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            GlobalZIndex(tokens::z::POPOVER),
        ))
        .id();
    let title = words(
        commands,
        kit,
        Phrase::BuildSyntaxTitle.text(lang),
        m.text,
        tokens::INK,
    );
    commands.entity(pop).add_child(title);
    for (i, (example, meaning)) in SYNTAX.iter().enumerate() {
        let row = commands
            .spawn((
                Node {
                    min_height: px_fixed(if m.touch() { m.hit } else { m.scaled(28.0) }),
                    align_items: AlignItems::Center,
                    column_gap: m.px(12.0),
                    padding: UiRect::axes(m.px(6.0), px_fixed(0.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                crate::ambience::Feel::tinting_to(Color::NONE, tokens::SELECTED),
                Press::Build(BuildPress::InsertSyntax(i)),
            ))
            .id();
        let code = commands
            .spawn((
                Text::new(*example),
                tf_bold(kit.fonts, m.small),
                TextColor(tokens::ACCENT),
                TextLayout::no_wrap(),
                Node {
                    width: m.px(150.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let what = words(commands, kit, meaning.text(lang), m.small, tokens::MUTED);
        commands.entity(row).add_children(&[code, what]);
        commands.entity(pop).add_child(row);
    }
    pop
}
