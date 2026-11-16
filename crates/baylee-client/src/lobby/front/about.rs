//! The symmetric, scrollable "About Baylee" (`Über Baylee`) modal dialog:
//! standard dialog header with glowing icon badge and top-right `×` close
//! button, structured sections with clear headings and styled hyperlinks, an
//! expanded third-party acknowledgements inventory, and the full build
//! identification centered along the bottom edge.

#[allow(clippy::wildcard_imports)]
use super::super::*;
use super::keys::ABOUT;
use super::links::{self, EXTERNAL_ICON};
use super::{FAN_CONTENT_NOTICE, FrontPress, source_address};
use crate::lobby::source::ExternalLink;
use crate::shellkit::controls::Kit;
use crate::shellkit::focus::Stop;
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::shellkit::{surfaces, tokens};

/// Formats the full build identification shown centered at the bottom of the
/// About dialog (`Baylee <short> · <target> · <profile>[ · <built_at>]`).
#[must_use]
pub(crate) fn full_build_line() -> String {
    let mut out = format!(
        "Baylee {} \u{b7} {} \u{b7} {}",
        baylee_build::short(),
        baylee_build::TARGET,
        baylee_build::PROFILE,
    );
    if !baylee_build::BUILT_AT.is_empty() && baylee_build::BUILT_AT != "unknown" {
        out.push_str(" \u{b7} ");
        out.push_str(baylee_build::BUILT_AT);
    }
    out
}

/// Spawns the `Über Baylee` / `About Baylee` modal dialog when `state.about_open`
/// is true.
#[allow(clippy::too_many_lines)]
pub(crate) fn about(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    scrolled: &Scrolled,
) {
    if !state.about_open {
        return;
    }
    let lang = state.lobby.lang();
    let at = |id: &'static str| Stop::new(ABOUT.name, id);
    let phone = kit.m.frame == Frame::Phone;

    let surface = surfaces::dialog(
        commands,
        kit,
        kit.m.px(760.0),
        tokens::PRIMARY_EDGE.with_alpha(0.45),
        Press::Shared(SharedPress::PickerNothing),
    );
    commands
        .entity(surface)
        .entry::<Node>()
        .and_modify(move |mut node| {
            if phone {
                node.width = Val::Percent(100.0);
                node.max_width = Val::Percent(100.0);
                node.max_height = Val::Percent(100.0);
            } else {
                node.max_height = Val::Vh(86.0);
            }
        });

    let mut close_a11y = accesskit::Node::new(accesskit::Role::Button);
    close_a11y.set_label(Phrase::SheetClose.text(lang));
    let head = surfaces::dialog_header(
        commands,
        kit,
        surfaces::SheetHeader::new(Phrase::AboutTitle.text(lang))
            .icon('\u{f05a}')
            .subtitle(Phrase::AboutSubtitle.text(lang))
            .close((
                Press::Front(FrontPress::About(false)),
                at("close"),
                bevy::a11y::AccessibilityNode(close_a11y),
            )),
    );

    let scroller = commands
        .spawn((
            surfaces::SheetScroller,
            Role::Scroll,
            Scrollable(List::About),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::About))),
            at("text"),
            Pickable::default(),
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 1.0,
                flex_grow: 1.0,
                min_height: px_fixed(0.0),
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: kit.m.px(18.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();

    let legal_sec = legal_section(commands, kit, lang);
    let source_sec = source_section(commands, state, kit, lang);
    let third_party_sec = third_party_section(commands, kit, lang);
    commands
        .entity(scroller)
        .add_children(&[legal_sec, source_sec, third_party_sec]);

    let text_band = crate::lobby::scrollbars::band_when_needed(commands, scroller, phone);
    let version_foot = build_version_footer(commands, kit);

    commands
        .entity(surface)
        .add_children(&[head, text_band, version_foot]);

    let scrim = surfaces::scrim(
        commands,
        kit,
        tokens::z::SHEET,
        Press::Front(FrontPress::About(false)),
        surface,
    );
    commands.entity(root).add_child(scrim);
}

fn section_card(commands: &mut Commands, kit: Kit, glyph: char, title: &str) -> (Entity, Entity) {
    let card = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(kit.m.px(16.0)),
                row_gap: kit.m.px(12.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::srgba(0.028, 0.052, 0.088, 0.56)),
            BorderColor::all(tokens::BORDER.with_alpha(0.78)),
            Pickable::IGNORE,
        ))
        .id();

    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: kit.m.px(9.0),
                padding: UiRect::bottom(kit.m.px(8.0)),
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER.with_alpha(0.55)),
            Pickable::IGNORE,
        ))
        .id();
    let icon = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.96),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let heading = commands
        .spawn((
            Text::new(title),
            crate::hud::tf_bold(kit.fonts, surfaces::whole_pixels(kit.m.text * 1.04)),
            TextColor(tokens::INK),
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head).add_children(&[icon, heading]);

    let content = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(10.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_children(&[head, content]);
    (card, content)
}

fn para(commands: &mut Commands, kit: Kit, words: &str, muted: bool) -> Entity {
    let text = surfaces::prose(commands, kit, words, muted);
    commands.entity(text).insert(Node {
        width: Val::Percent(100.0),
        ..default()
    });
    text
}

fn legal_section(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let (card, body) = section_card(
        commands,
        kit,
        '\u{f24e}',
        Phrase::AboutSectionLegal.text(lang),
    );
    let fan_notice = links::prose_with_inline_links(
        commands,
        kit,
        FAN_CONTENT_NOTICE,
        &[(
            "Fan Content Policy",
            FrontPress::OpenExternal(ExternalLink::FanContentPolicy),
        )],
        Justify::Left,
        false,
    );
    let scryfall = links::prose_with_inline_links(
        commands,
        kit,
        Phrase::ScryfallCredit.text(lang),
        &[("Scryfall", FrontPress::OpenExternal(ExternalLink::Scryfall))],
        Justify::Left,
        false,
    );
    commands.entity(body).add_children(&[fan_notice, scryfall]);
    card
}

fn source_section(commands: &mut Commands, state: &LobbyState, kit: Kit, lang: Lang) -> Entity {
    let at = |id: &'static str| Stop::new(ABOUT.name, id);
    let (card, body) = section_card(
        commands,
        kit,
        '\u{f121}',
        Phrase::AboutSectionSource.text(lang),
    );
    let licence_para = para(commands, kit, Phrase::AboutLicence.text(lang), false);
    commands.entity(body).add_child(licence_para);

    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(16.0),
                row_gap: kit.m.px(10.0),
                flex_wrap: FlexWrap::Wrap,
                padding: UiRect::top(kit.m.px(2.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let links_col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: kit.m.px(6.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let address = Phrase::SourceCode.fill(lang, &[source_address(state)]);
    let source_item = if state.source_code.is_some() {
        links::hyperlink(
            commands,
            kit,
            '\u{f126}',
            &address,
            true,
            (Press::Front(FrontPress::OpenSource), at("source")),
        )
    } else {
        para(commands, kit, &address, false)
    };
    let agpl_link = links::hyperlink(
        commands,
        kit,
        EXTERNAL_ICON,
        Phrase::AboutLicenceLink.text(lang),
        true,
        Press::Front(FrontPress::OpenExternal(ExternalLink::AgplLicence)),
    );
    commands
        .entity(links_col)
        .add_children(&[source_item, agpl_link]);

    if let Some((_, host)) = super::impressum_link(state) {
        let imp = links::hyperlink(
            commands,
            kit,
            EXTERNAL_ICON,
            &Phrase::ImpressumLink.fill(lang, &[&host]),
            true,
            (Press::Front(FrontPress::OpenImpressum), at("impressum")),
        );
        commands.entity(links_col).add_child(imp);
    }

    commands.entity(row).add_child(links_col);
    if let Some(code) = state.source_code.as_ref() {
        let qr = super::door::qr(commands, code);
        commands.entity(row).add_child(qr);
    }
    commands.entity(body).add_child(row);
    card
}

struct ThirdPartyEntry<'a> {
    icon: char,
    heading: &'a str,
    links: &'a [(&'a str, ExternalLink)],
    paragraphs: &'a [&'a str],
}

#[allow(clippy::too_many_lines)]
fn third_party_section(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let (card, body) = section_card(
        commands,
        kit,
        '\u{f1b3}',
        Phrase::AboutThirdParty.text(lang),
    );
    let entries: [ThirdPartyEntry<'_>; 8] = [
        ThirdPartyEntry {
            icon: '\u{f031}',
            heading: Phrase::AboutCatTypography.text(lang),
            links: &[
                ("Alegreya Sans", ExternalLink::AlegreyaSans),
                ("Faustina", ExternalLink::Faustina),
            ],
            paragraphs: &[Phrase::AboutFonts.text(lang)],
        },
        ThirdPartyEntry {
            icon: '\u{f53f}',
            heading: Phrase::AboutCatIcons.text(lang),
            links: &[
                ("Font Awesome Free", ExternalLink::FontAwesome),
                ("Mana", ExternalLink::ManaFont),
            ],
            paragraphs: &[Phrase::AboutIcons.text(lang), Phrase::AboutMana.text(lang)],
        },
        ThirdPartyEntry {
            icon: '\u{f5fd}',
            heading: Phrase::AboutCatDecks.text(lang),
            links: &[("MTGJSON", ExternalLink::MtgJson)],
            paragraphs: &[Phrase::AboutPrecons.text(lang)],
        },
        ThirdPartyEntry {
            icon: '\u{f1b2}',
            heading: Phrase::AboutCatEngine.text(lang),
            links: &[
                ("Bevy Engine", ExternalLink::Bevy),
                ("wgpu", ExternalLink::Wgpu),
                ("rav1d (dav1d)", ExternalLink::Rav1d),
            ],
            paragraphs: &[Phrase::AboutEngine.text(lang)],
        },
        ThirdPartyEntry {
            icon: '\u{f0ec}',
            heading: Phrase::AboutCatNetwork.text(lang),
            links: &[
                ("Tokio & Axum", ExternalLink::Tokio),
                ("Rustls", ExternalLink::Rustls),
                ("SeaORM", ExternalLink::SeaOrm),
            ],
            paragraphs: &[Phrase::AboutNetwork.text(lang)],
        },
        ThirdPartyEntry {
            icon: '\u{f3ed}',
            heading: Phrase::AboutCatCrypto.text(lang),
            links: &[
                ("OPAQUE (opaque-ke)", ExternalLink::OpaqueKe),
                ("RustCrypto", ExternalLink::RustCrypto),
                ("GNOME libsecret", ExternalLink::Libsecret),
            ],
            paragraphs: &[Phrase::AboutCrypto.text(lang)],
        },
        ThirdPartyEntry {
            icon: '\u{f001}',
            heading: Phrase::AboutCatAudio.text(lang),
            links: &[
                ("VSCO 2 CE & VCSL", ExternalLink::Vsco2),
                ("FreePats", ExternalLink::FreePats),
                ("Mutopia Project", ExternalLink::Mutopia),
            ],
            paragraphs: &[Phrase::AboutAudio.text(lang)],
        },
        ThirdPartyEntry {
            icon: '\u{f126}',
            heading: Phrase::AboutCatEcosystem.text(lang),
            links: &[
                ("Project Fluent", ExternalLink::Fluent),
                ("qrcodegen", ExternalLink::Qrcodegen),
            ],
            paragraphs: &[Phrase::AboutCrates.text(lang)],
        },
    ];

    for (idx, entry) in entries.iter().enumerate() {
        let item = spawn_third_party_entry(commands, kit, entry, idx > 0);
        commands.entity(body).add_child(item);
    }
    card
}

fn spawn_third_party_entry(
    commands: &mut Commands,
    kit: Kit,
    entry: &ThirdPartyEntry<'_>,
    top_divider: bool,
) -> Entity {
    let block = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(5.0),
                padding: UiRect::top(if top_divider {
                    kit.m.px(10.0)
                } else {
                    px_fixed(0.0)
                }),
                border: UiRect::top(px_fixed(if top_divider { 1.0 } else { 0.0 })),
                ..default()
            },
            BorderColor::all(tokens::BORDER.with_alpha(0.38)),
            Pickable::IGNORE,
        ))
        .id();

    let title_row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(10.0),
                row_gap: kit.m.px(4.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let left = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(7.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let cat_icon = commands
        .spawn((
            Text::new(entry.icon.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.86),
            TextColor(tokens::ACCENT.with_alpha(0.90)),
            Pickable::IGNORE,
        ))
        .id();
    let cat_title = commands
        .spawn((
            Text::new(entry.heading),
            crate::hud::tf_bold(kit.fonts, surfaces::whole_pixels(kit.m.small * 1.04)),
            TextColor(tokens::INK),
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(left).add_children(&[cat_icon, cat_title]);

    let link_strip = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(4.0),
                row_gap: kit.m.px(2.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for &(label, target) in entry.links {
        let chip = links::hyperlink(
            commands,
            kit,
            EXTERNAL_ICON,
            label,
            true,
            Press::Front(FrontPress::OpenExternal(target)),
        );
        commands.entity(link_strip).add_child(chip);
    }

    commands.entity(title_row).add_children(&[left, link_strip]);
    commands.entity(block).add_child(title_row);

    for words in entry.paragraphs {
        let p = para(commands, kit, words, true);
        commands.entity(block).add_child(p);
    }
    block
}

fn build_version_footer(commands: &mut Commands, kit: Kit) -> Entity {
    let band = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(px_fixed(kit.m.pad), kit.m.px(10.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: kit.m.px(7.0),
                flex_shrink: 0.0,
                border: UiRect::top(px_fixed(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.030, 0.055, 0.45)),
            BorderColor::all(tokens::BORDER.with_alpha(0.75)),
            Pickable::IGNORE,
        ))
        .id();
    let tag = commands
        .spawn((
            Text::new(super::door::VERSION_TAG_GLYPH.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.82),
            TextColor(tokens::ACCENT.with_alpha(0.85)),
            Pickable::IGNORE,
        ))
        .id();
    let line = commands
        .spawn((
            Text::new(full_build_line()),
            crate::hud::tf(kit.fonts, surfaces::whole_pixels(kit.m.small * 0.94)),
            TextColor(tokens::MUTED),
            TextLayout::justify(Justify::Center),
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(band).add_children(&[tag, line]);
    band
}
