#[allow(clippy::wildcard_imports)]
use super::*;

use crate::shellkit::tokens::{RADIUS_CONTROL, RADIUS_PILL};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum IconTone {
    Neutral,
    Active,
    Frost,
    FrostIdle,
    Gold,
    Mythic,
    Danger,
}

impl IconTone {
    pub(super) const fn colors(self) -> (Color, Color, Color) {
        match self {
            Self::Neutral => (
                Color::srgba(0.045, 0.062, 0.086, 0.78),
                tokens::BORDER,
                tokens::MUTED,
            ),
            Self::Active => (
                Color::srgba(0.10, 0.23, 0.34, 0.92),
                Color::srgba(0.42, 0.82, 0.98, 0.84),
                Color::srgb(0.74, 0.93, 1.0),
            ),
            Self::Frost => (
                Color::srgba(0.07, 0.22, 0.36, 0.94),
                Color::srgba(0.56, 0.90, 1.0, 0.92),
                Color::srgb(0.78, 0.96, 1.0),
            ),
            Self::FrostIdle => (
                Color::srgba(0.05, 0.09, 0.14, 0.82),
                Color::srgba(0.45, 0.76, 0.94, 0.42),
                Color::srgb(0.68, 0.86, 0.96),
            ),
            Self::Gold => (
                Color::srgba(0.11, 0.08, 0.03, 0.88),
                Color::srgba(0.95, 0.78, 0.34, 0.78),
                Color::srgb(1.0, 0.87, 0.44),
            ),
            Self::Mythic => (
                Color::srgba(0.16, 0.08, 0.04, 0.90),
                Color::srgba(1.0, 0.58, 0.28, 0.82),
                Color::srgb(1.0, 0.70, 0.40),
            ),
            Self::Danger => (
                Color::srgba(0.20, 0.06, 0.07, 0.84),
                Color::srgba(0.92, 0.34, 0.38, 0.62),
                Color::srgb(0.98, 0.54, 0.54),
            ),
        }
    }
}

fn spawn_icon_face(
    commands: &mut Commands,
    kit: Kit,
    icon: char,
    badge: Option<&str>,
    tone: IconTone,
    allowed: bool,
) -> Entity {
    let (rest_bg, rest_rim, rest_ink) = tone.colors();
    let ground = if allowed {
        rest_bg
    } else {
        tokens::CONTROL.with_alpha(0.55)
    };
    let rim = if allowed { rest_rim } else { tokens::BORDER };
    let ink = if allowed { rest_ink } else { tokens::DISABLED };
    let has_badge = badge.is_some_and(|text| !text.is_empty());
    let face = commands
        .spawn((
            crate::shellkit::Role::Button,
            Node {
                min_width: kit.m.px(if has_badge { 36.0 } else { 30.0 }),
                min_height: kit.m.px(28.0),
                padding: UiRect::axes(kit.m.px(if has_badge { 9.0 } else { 7.0 }), px_fixed(0.0)),
                column_gap: kit.m.px(5.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(if has_badge {
                    RADIUS_PILL
                } else {
                    RADIUS_CONTROL
                })),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(rim),
            Pickable::IGNORE,
        ))
        .id();
    commands.spawn((
        Text::new(icon.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.small * 0.94),
        TextColor(ink),
        Pickable::IGNORE,
        ChildOf(face),
    ));
    if let Some(badge_text) = badge.filter(|text| !text.is_empty()) {
        commands.spawn((
            Text::new(badge_text),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(ink),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
            ChildOf(face),
        ));
    }
    if allowed {
        let shadow = if tone == IconTone::Frost {
            BoxShadow(vec![ShadowStyle {
                color: Color::srgba(0.38, 0.84, 1.0, 0.30),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(0.0),
                spread_radius: px_fixed(0.5),
                blur_radius: px_fixed(10.0),
            }])
        } else {
            surfaces::elevation_control()
        };
        commands.entity(face).insert((
            shadow,
            crate::ambience::Feel {
                lift: 0.0,
                ..crate::ambience::Feel::new(ground)
            },
            crate::ambience::FeelBorder {
                rest: rim,
                hot: crate::ambience::lighter(rim, 0.30),
                press: tokens::GOLD,
            },
            crate::ambience::FeelShadow,
        ));
    } else {
        commands
            .entity(face)
            .insert(surfaces::elevation_control_disabled());
    }
    face
}

/// Compact, functionally color-coded icon button with optional short count/price badge and hover tooltip.
#[allow(clippy::too_many_arguments)]
pub(super) fn icon_control(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    icon: char,
    badge: Option<&str>,
    tone: IconTone,
    tooltip: &str,
    action: Action,
    available: bool,
    unavailable_reason: Option<&str>,
) -> Entity {
    let lang = kit.lang();
    let busy_gate = if sends_at_once(action) {
        ui.model.busy()
    } else {
        ui.holding()
    };
    let allowed = available
        && !busy_gate
        && (ui.confirmation.is_none() || matches!(action, Action::Accept | Action::Cancel));
    let face = spawn_icon_face(commands, kit, icon, badge, tone, allowed);
    let wrapper = controls::hit(commands, kit, face, Press::Echoes(action));
    commands.entity(wrapper).insert((
        super::keys::stop(action, ui),
        crate::shellkit::focus::Current(words::selected(action, ui)),
    ));
    if !allowed {
        commands.entity(wrapper).insert(controls::Disabled);
        let reason = unavailable_reason.unwrap_or_else(|| Phrase::EchoesUnavailable.text(lang));
        let full_tip = if tooltip.is_empty() {
            reason.to_owned()
        } else {
            format!("{tooltip} — {reason}")
        };
        controls::attach_tooltip(commands, wrapper, kit, &full_tip);
    } else if !tooltip.is_empty() {
        controls::attach_tooltip(commands, wrapper, kit, tooltip);
    }
    commands.entity(root).add_child(wrapper);
    wrapper
}

#[allow(clippy::too_many_arguments)]
pub(super) fn control(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    words: &str,
    action: Action,
    available: bool,
) {
    let lang = kit.lang();
    let busy_gate = if sends_at_once(action) {
        ui.model.busy()
    } else {
        ui.holding()
    };
    let allowed = available
        && !busy_gate
        && (ui.confirmation.is_none() || matches!(action, Action::Accept | Action::Cancel));
    let weight = if matches!(action, Action::Abandon | Action::Withdraw) {
        Weight::Danger
    } else if matches!(
        action,
        Action::Accept
            | Action::Starter(..)
            | Action::Buy
            | Action::Play(_)
            | Action::Resume
            | Action::SaveDeck
            | Action::ClaimReward
    ) {
        Weight::Gold
    } else if words::selected(action, ui) {
        Weight::Primary
    } else {
        Weight::Secondary
    };
    let button = controls::button_compact(
        commands,
        kit,
        Some(if words::selected(action, ui) {
            glyph::CHECK
        } else {
            words::icon(action)
        }),
        words,
        weight,
        if allowed {
            Live::Yes
        } else {
            Live::No(Phrase::EchoesUnavailable.text(lang))
        },
        Press::Echoes(action),
    );
    commands.entity(button).insert((
        super::keys::stop(action, ui),
        crate::shellkit::focus::Current(words::selected(action, ui)),
    ));
    commands.entity(root).add_child(button);
}

/// A press that goes straight to the account worker rather than through the
/// shop's queued commands: while any read holds the worker it is drawn held,
/// because the worker would refuse it (and the press would be lost).
pub(super) const fn sends_at_once(action: Action) -> bool {
    matches!(
        action,
        Action::Starter(..)
            | Action::Accept
            | Action::Retry
            | Action::Play(_)
            | Action::Resume
            | Action::Listing(_)
    )
}

/// The shelf places stage seven's crafting offer may replace: the last two of
/// however many the run's generation shows (six and seven, or ten and eleven).
pub(super) fn mythic_slots(slots: usize) -> impl Iterator<Item = u8> {
    (slots.saturating_sub(2)..slots).filter_map(|slot| u8::try_from(slot).ok())
}

pub(super) fn can(ui: &Ui, command: &Command) -> bool {
    !ui.model.uncertain()
        && (matches!(command, Command::Edit { .. } | Command::Refit { .. })
            || ui.choices.deck_unchanged(ui.model.run()))
        && ui
            .model
            .run()
            .zip(ui.model.publication())
            .is_some_and(|(run, cohort)| allowed(run, cohort, command))
}

pub(super) fn portrait(commands: &mut Commands, root: Entity, kit: Kit, goods: Goods) {
    portrait_at(commands, root, kit, goods, 84.0);
}

pub(super) fn featured_portrait(commands: &mut Commands, root: Entity, kit: Kit, goods: Goods) {
    portrait_at(commands, root, kit, goods, 84.0);
}

pub(super) fn effect(commands: &mut Commands, root: Entity, kit: Kit, goods: Goods) {
    paragraph(
        commands,
        root,
        kit,
        &words::effect(goods, kit.lang()),
        tokens::INK,
    );
}

pub(super) fn paragraph(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    words: &str,
    color: Color,
) {
    let text = crate::shellkit::surfaces::prose(commands, kit, words, true);
    commands.entity(text).insert((
        TextColor(color),
        Node {
            width: percent(100),
            min_width: px_fixed(0.0),
            flex_shrink: 0.0,
            // Leave room for glyph overhang after the desktop UI scale is applied.
            padding: UiRect::right(kit.m.px(4.0)),
            ..default()
        },
        TextLayout {
            linebreak: LineBreak::WordOrCharacter,
            ..default()
        },
    ));
    commands.entity(root).add_child(text);
}

fn portrait_at(commands: &mut Commands, root: Entity, kit: Kit, goods: Goods, size: f32) {
    let band = row(commands, root, kit);
    commands
        .entity(band)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_wrap = FlexWrap::NoWrap;
            node.align_items = AlignItems::Center;
        });
    let image = commands
        .spawn((
            crate::itemart::Portrait(goods),
            Node {
                width: kit.m.px(size),
                height: kit.m.px(size),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(band).add_child(image);
    let copy = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                row_gap: kit.m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(band).add_child(copy);
    let name = merchandise::caption(
        commands,
        copy,
        kit,
        &words::name(goods, kit.lang()),
        tokens::INK,
    );
    commands.entity(name).insert((
        crate::hud::tf_bold(kit.fonts, kit.m.text),
        TextLayout {
            linebreak: LineBreak::WordOrCharacter,
            ..default()
        },
    ));
    if let Goods::Relic(definition) = goods {
        label(
            commands,
            copy,
            kit,
            words::rarity(definition.rarity, kit.lang()),
            tokens::MUTED,
        );
    }
}

pub(super) fn stat(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    icon: char,
    caption: &str,
    value: &str,
) {
    let card = crate::lobby::parts::badge_with(
        commands,
        kit,
        icon,
        &format!("{value} · {caption}"),
        tokens::INK,
    );
    commands.entity(root).add_child(card);
}

pub(super) fn tile(commands: &mut Commands, root: Entity, kit: Kit) -> Entity {
    let tile = panel(commands, root, kit);
    commands
        .entity(tile)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.width = Val::Auto;
            n.flex_basis = if kit.m.frame == crate::shellkit::Frame::Phone {
                percent(100)
            } else {
                percent(47)
            };
            n.flex_grow = 1.0;
            n.min_width = px(0);
            n.align_self = AlignSelf::Stretch;
        });
    tile
}

/// The deck's number as the player saw it: its place among the account's
/// offered starters (a drafted cohort offers three of a larger pool).
fn shown_starter(ui: &Ui, cohort: Uuid, starter: u8) -> String {
    ui.model
        .cohorts()
        .iter()
        .find(|published| published.id == cohort)
        .and_then(|published| published.offered.iter().position(|&s| s == starter))
        .unwrap_or(usize::from(starter))
        .saturating_add(1)
        .to_string()
}

/// What a confirmation says, whether it may still be accepted, and whether it
/// destroys something.
fn confirming(ui: &Ui, lang: Lang) -> (String, bool, bool) {
    match ui.confirmation.as_ref().expect("shown confirmation") {
        Confirmation::Listing(revision) => (
            Phrase::EchoesRankingConsent.text(lang).to_string(),
            !ui.model.uncertain()
                && ui
                    .model
                    .ranking()
                    .is_some_and(|b| b.listing.revision == *revision),
            false,
        ),
        Confirmation::Withdrawal => (
            Phrase::EchoesWithdrawConfirm.text(lang).to_string(),
            !ui.model.uncertain(),
            true,
        ),
        Confirmation::Enrol {
            cohort,
            starter,
            consent,
        } => (
            Phrase::EchoesConsent.fill(
                lang,
                &[&shown_starter(ui, *cohort, *starter), &consent.to_string()],
            ),
            !ui.model.uncertain() && *consent != 0 && *consent == ui.model.consent(),
            false,
        ),
        Confirmation::Choice {
            run,
            revision,
            command,
        } => (
            words::command(command, ui, lang),
            ui.model
                .run()
                .is_some_and(|current| current.id == *run && current.revision == *revision)
                && can(ui, command),
            matches!(
                command,
                Command::Abandon {} | Command::Sell { .. } | Command::SellCard { .. }
            ),
        ),
    }
}

pub(super) fn confirmation(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui) {
    let lang = kit.lang();
    let (text, valid, destructive) = confirming(ui, lang);
    let copy = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(10.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    paragraph(commands, copy, kit, &text, tokens::INK);
    let cancel = controls::button_compact(
        commands,
        kit,
        Some(glyph::CLOSE),
        Phrase::ChallengeCancel.text(lang),
        Weight::Secondary,
        Live::Yes,
        Press::Echoes(Action::Cancel),
    );
    commands
        .entity(cancel)
        .insert(super::keys::stop(Action::Cancel, ui));
    let confirm = controls::button_compact(
        commands,
        kit,
        Some(glyph::CHECK),
        Phrase::ChallengeConfirm.text(lang),
        if destructive {
            Weight::Danger
        } else {
            Weight::Gold
        },
        if valid && !ui.model.busy() {
            Live::Yes
        } else {
            Live::No(Phrase::EchoesUnavailable.text(lang))
        },
        Press::Echoes(Action::Accept),
    );
    commands
        .entity(confirm)
        .insert(super::keys::stop(Action::Accept, ui));
    let surface = surfaces::sheet_box(
        commands,
        kit,
        surfaces::SheetWidth::Small,
        Phrase::ChallengeConfirm.text(lang),
        &[copy],
        &[cancel, confirm],
    );
    commands
        .entity(surface)
        .insert(Press::Shared(crate::lobby::SharedPress::PickerNothing));
    let scrim = surfaces::sheet(commands, surface);
    commands.entity(scrim).insert(Press::Echoes(Action::Cancel));
    commands.entity(root).add_child(scrim);
}
