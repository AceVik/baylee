//! The hero: "Your next game", with its deck, its difficulty and its start.

#[allow(clippy::wildcard_imports)] // the Play screen's own vocabulary
use super::*;

/// The hero: "Your next game".
#[allow(clippy::too_many_lines)] // one panel, read top to bottom
pub(super) fn hero(
    commands: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
) -> HeroAnchors {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let phone = kit.m.frame == ShellFrame::Phone;
    let strip_hero = !phone && !matches!(kit.m.frame, ShellFrame::Wide | ShellFrame::Vast);
    let stack_deck = !phone
        && !strip_hero
        && crate::shellkit::Platform::current() == crate::shellkit::Platform::Desktop;
    let mut anchors = HeroAnchors::default();
    let panel = commands
        .spawn((
            TourAnchor(Anchor::PlayHero),
            Role::Panel,
            Node {
                width: percent(100),
                flex_direction: if strip_hero {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                flex_wrap: if strip_hero {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                align_items: if strip_hero {
                    AlignItems::Center
                } else {
                    AlignItems::Stretch
                },
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: px_fixed(kit.m.gap),
                column_gap: px_fixed(kit.m.gap),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
            super::super::dock::Dock(3),
        ))
        .id();
    commands.entity(parent).add_child(panel);
    let off = starts_off(state, lang);
    let deck = lobby.next_deck();
    if deck.is_none() && !lobby.offline() {
        // First run (S-3): three house decks, one press to copy and use.
        let caption = parts::caption(commands, kit, Phrase::PlayPickToStart.text(lang));
        let minis = mini_tiles(commands, state, kit, &orders::PLAY, "minis");
        commands.entity(panel).add_children(&[caption, minis]);
    } else if !strip_hero && !phone {
        let caption = parts::caption(commands, kit, Phrase::PlayYourNextGame.text(lang));
        commands.entity(panel).add_child(caption);
    }
    if let Some(deck) = deck {
        let line = commands
            .spawn((
                Node {
                    // A desktop sidebar can lose a third of its space to
                    // the friends drawer. Stack artwork and words there;
                    // their widths must never compete inside a narrow lane.
                    flex_direction: if stack_deck {
                        FlexDirection::Column
                    } else {
                        FlexDirection::Row
                    },
                    row_gap: kit.m.px(8.0),
                    column_gap: kit.m.px(12.0),
                    align_items: AlignItems::Start,
                    flex_shrink: 1.0,
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let art_width = if phone || strip_hero { 78.0 } else { 164.0 };
        if let Some(art) = deck.art() {
            let picture = parts::art_with_credit(commands, kit, lang, &art, art_width);
            commands.entity(line).add_child(picture);
        } else {
            // No artist known: the identity gradient alone (principle 4).
            let band = commands
                .spawn((
                    Node {
                        width: kit.m.px(art_width),
                        height: kit.m.px(art_width * 457.0 / 626.0),
                        flex_shrink: 0.0,
                        border_radius: BorderRadius::all(px_fixed(6.0)),
                        ..default()
                    },
                    parts::identity_gradient(&deck.identity),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(line).add_child(band);
        }
        let words = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(4.0),
                    flex_grow: 1.0,
                    width: if stack_deck { percent(100) } else { Val::Auto },
                    flex_basis: if stack_deck { Val::Auto } else { px(0) },
                    flex_shrink: 1.0,
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let name = commands
            .spawn((
                Text::new(deck.name.clone()),
                crate::hud::tf_bold(kit.fonts, if phone { kit.m.text } else { kit.m.head }),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            ))
            .id();
        // On a phone and a narrow strip, `⋯` (Change deck · Edit) stands
        // beside the name (M4-6).
        let name_row = parts::row(commands, kit, false);
        // The name takes the row beside `⋯` and wraps there; it never
        // shrinks out of sight.
        commands.entity(name).insert(Node {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_width: kit.m.px(48.0),
            ..default()
        });
        commands.entity(name_row).add_child(name);
        if phone {
            // A phone's rail is 220-280 wide: the name takes its own line
            // over the picture, with `⋯` at its end, so a long one is not
            // squeezed beside the art.
            commands
                .entity(name_row)
                .entry::<Node>()
                .and_modify(|mut n| n.width = percent(100));
            commands.entity(panel).add_child(name_row);
        } else {
            commands.entity(words).add_child(name_row);
        }
        let pips = parts::row(commands, kit, true);
        let format = parts::badge(
            commands,
            kit,
            &shelf::format_label(lang, &deck.format),
            tokens::ACCENT,
        );
        commands.entity(pips).add_child(format);
        if phone {
            if !deck.identity.is_empty() {
                let letters = parts::badge(commands, kit, &deck.identity, tokens::INK);
                commands.entity(pips).add_child(letters);
            }
        } else if !deck.identity.is_empty() {
            let discs = parts::identity_discs(commands, kit, &deck.identity);
            commands.entity(pips).add_child(discs);
        }
        commands.entity(words).add_child(pips);
        // The gateway will refuse this deck while it holds a blocked card:
        // say which and why before the press, not only after it.
        if let Some(block) = deck.blocked.first() {
            let said = parts::line(
                commands,
                kit,
                &block.sentence(lang),
                kit.m.small,
                tokens::DANGER,
            );
            commands.entity(words).add_child(said);
        }
        if !phone {
            let meta = parts::line(
                commands,
                kit,
                &shelf::meta_line(lang, deck, parts::now_secs()),
                kit.m.small,
                tokens::MUTED,
            );
            commands.entity(words).add_child(meta);
        }
        if phone || strip_hero {
            let more = parts::icon_button(
                commands,
                kit,
                parts::ELLIPSIS,
                Press::Shared(SharedPress::OpenMenu(ShellMenu::Hero)),
            );
            orders::stop(commands, more, &orders::PLAY, "more");
            anchors.more = Some(more);
            commands.entity(name_row).add_child(more);
        } else {
            let links = parts::row(commands, kit, true);
            let change = controls::button(
                commands,
                kit,
                Phrase::PlayChangeDeck.text(lang),
                Weight::Ghost,
                Live::Yes,
                None,
                Press::Play(PlayPress::ChangeDeck),
            );
            let edit = controls::button(
                commands,
                kit,
                Phrase::ShellEdit.text(lang),
                Weight::Ghost,
                Live::Yes,
                None,
                Press::Play(PlayPress::EditNext),
            );
            orders::stop(commands, change, &orders::PLAY, "change-deck");
            orders::stop(commands, edit, &orders::PLAY, "edit");
            commands.entity(links).add_children(&[change, edit]);
            commands.entity(words).add_child(links);
        }
        commands.entity(line).add_child(words);
        commands.entity(panel).add_child(line);
    }

    // Return to your game: gold, the hero's primary while the player holds
    // a chair — not on a phone, where the header's pill is the way (S4-3).
    let playing = matches!(strips::strip(lobby), Some(Strip::Playing { .. }));
    if !phone && playing {
        let back = controls::button(
            commands,
            kit,
            Phrase::ShellReturnToGame.text(lang),
            Weight::Gold,
            Live::Yes,
            Some("r"),
            Press::Play(PlayPress::Return),
        );
        orders::stop(commands, back, &orders::PLAY, "return");
        commands.entity(panel).add_child(back);
    }
    let buttons = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap),
                flex_grow: if strip_hero { 1.0 } else { 0.0 },
                min_width: if strip_hero { kit.m.px(260.0) } else { px(0) },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(buttons);
    let house_live = match (&off, deck) {
        (Some(_), _) => Live::No(""),
        (None, None) => Live::No(Phrase::PlayPickToStart.text(lang)),
        (None, Some(_)) if lobby.busy() => Live::No(Phrase::VeilTalking.text(lang)),
        (None, Some(_)) => Live::Yes,
    };
    let house = controls::button(
        commands,
        kit,
        Phrase::PlayTheHouse.text(lang),
        Weight::Primary,
        house_live,
        None,
        Press::Play(PlayPress::PlayHouse),
    );
    let caret = parts::menu_button(
        commands,
        kit,
        super::super::ai_name(lang, difficulty(state.play.difficulty)),
        Weight::Secondary,
        Press::Shared(SharedPress::OpenMenu(ShellMenu::Difficulty)),
    );
    anchors.caret = Some(caret);
    orders::stop(commands, house, &orders::PLAY, "play-house");
    orders::stop(commands, caret, &orders::PLAY, "difficulty");
    let pair = commands
        .spawn((
            Node {
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(8.0),
                align_items: AlignItems::Start,
                // A 220 rail (640 x 360) puts the difficulty under it.
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(house)
        .entry::<Node>()
        .and_modify(|mut n| n.flex_grow = 1.0);
    commands.entity(pair).insert(TourAnchor(Anchor::PlayHouse));
    commands.entity(pair).add_children(&[house, caret]);
    let create_live = if off.is_some() {
        Live::No("")
    } else if lobby.busy() {
        Live::No(Phrase::VeilTalking.text(lang))
    } else {
        Live::Yes
    };
    let create = controls::button(
        commands,
        kit,
        Phrase::CreateTable.text(lang),
        Weight::Secondary,
        create_live,
        Some("c"),
        Press::Play(PlayPress::CreateTable),
    );
    orders::stop(commands, create, &orders::PLAY, "create");
    commands
        .entity(create)
        .insert(TourAnchor(Anchor::PlayCreate));
    commands.entity(buttons).add_children(&[pair, create]);
    // The reason stands once, under the pair (S4-3).
    if let Some(reason) = &off {
        let why = parts::line(commands, kit, reason, kit.m.small, tokens::GOLD);
        commands.entity(buttons).add_child(why);
    }
    let _ = metrics;
    anchors
}
