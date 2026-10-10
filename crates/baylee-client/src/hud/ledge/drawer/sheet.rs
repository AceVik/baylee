//! The drawer as a decision sheet (the owner, 08.10.2026): a head that names
//! the question and its source, a body, the shelf's answers as its foot,
//! and a fold down to a pill.
//!
//! *"Make it clean and structured in the shell's design language: a header
//! naming what is being chosen …, the SOURCE shown with its card picture …,
//! hovering the picture opens the usual large hover preview, clear
//! legal-target hints, the actions … in a consistent footer with their key
//! caps. Make it minimisable … so the table underneath is fully visible and
//! clickable for picking targets; clicking the pill or the key restores it;
//! it never auto-dismisses the question."*
//!
//! - **The head** is the question as the shelf would say it (the title), the
//!   source's own sentence under it, and the source's picture at its left —
//!   a [`ChoicePreview`], so the pointer on it opens the table's preview of
//!   that object and a press on it answers nothing. The source is what the
//!   view names (`PlayerView::targeting`); a question the view gives no
//!   source for has a head without a picture rather than a guessed one.
//! - **The foot** is the shelf itself. The sheet grows out of the shelf, so
//!   the shelf's row of answers, each with its key cap, is the sheet's foot
//!   for every question; while the sheet stands the shelf drops its own
//!   sentence (the head says it), and the answers stay where a player's hand
//!   already is.
//! - **The fold** ([`baylee_client_core::decisionfold`]) is the sheet's own
//!   control at the head's right, and `X` (`Action::FoldDecision`). Folded,
//!   the sheet is a pill at the window's right edge over the strips — the
//!   source's picture, the question, the restore mark and its cap — and the
//!   table under it is the table, every target on it pickable. The question
//!   stands; the fold is the question's and the next one opens unfolded.
//!
//! The same head, fold, pill and footer button hold up another seat's
//! reveal (`hud::revealed`, the owner's word of 08.10.2026): one sheet
//! component, whichever sheet it is.

#[allow(clippy::wildcard_imports)] // the drawer's shared vocabulary
use super::*;
use baylee_client_core::images::{ArtSize, ImageKey};

/// The head's picture: a card's width and height at the sheet's scale.
const THUMB_W: f32 = 46.0;
const THUMB_H: f32 = THUMB_W * 88.0 / 63.0;

/// The pill's picture.
const PILL_THUMB_W: f32 = 20.0;
const PILL_THUMB_H: f32 = PILL_THUMB_W * 88.0 / 63.0;

/// The head's title and the source's sentence under it.
const TITLE_PT: f32 = 14.0;
const DETAIL_PT: f32 = 11.5;

/// The pill's words.
const PILL_PT: f32 = 12.0;

/// The fold control's square.
const FOLD: f32 = 24.0;

/// What the sheet's head says.
#[derive(Clone, PartialEq, Debug)]
pub(in crate::hud) struct Head {
    /// What is being chosen: the question, as the shelf would say it.
    pub(in crate::hud) title: String,
    /// The source's own words: whose question it is and the sentence that
    /// asks it.
    pub(in crate::hud) detail: Vec<String>,
    /// The source whose picture the head shows and the pill keeps.
    pub(in crate::hud) source: Option<Source>,
    /// Whether the sheet is folded to its pill.
    pub(in crate::hud) folded: bool,
}

/// The source of a question, as the view names it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::hud) struct Source {
    /// The object the preview opens.
    pub(in crate::hud) object: ObjectId,
    /// Its picture, when it has one.
    pub(in crate::hud) art: Option<ImageKey>,
}

/// The head's picture, which opens the table's preview of the source.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct SheetSource {
    /// The source object.
    pub object: ObjectId,
}

/// The folded sheet: a press restores it.
#[derive(Component)]
pub struct SheetPill;

/// Whether `prompt` is a question the sheet stands for: everything the
/// drawer draws that is a question of this seat's own, but combat (answered
/// on the creatures) and the cast choosers (on the parchment beside the
/// card).
#[must_use]
pub(crate) const fn is_sheet(prompt: &Prompt) -> bool {
    matches!(
        prompt,
        Prompt::ChooseTargets { .. }
            | Prompt::ChooseSubtype { .. }
            | Prompt::ChooseCardName
            | Prompt::ChooseColor { .. }
            | Prompt::ChooseNumber { .. }
            | Prompt::ChoosePlayer { .. }
            | Prompt::ChoosePile { .. }
            | Prompt::LegendRule
            | Prompt::ChooseCards { .. }
            | Prompt::ChooseManaAbility { .. }
            | Prompt::ChooseDamageSource { .. }
            | Prompt::ChooseDamageEffect { .. }
            | Prompt::AllocatePrevention { .. }
            | Prompt::TextReplacement { .. }
            | Prompt::Discard { .. }
            | Prompt::BottomCards { .. }
    )
}

/// Whether a sheet question is this seat's to answer right now, folded or
/// not: asked of this seat, not held by the zone dialog, the game going.
pub(crate) fn sheet_asked(duel: &Duel) -> bool {
    duel.ending().is_none()
        && duel.is_my_turn_to_act()
        && duel.cast_menu.is_none()
        && !duel.browser.answers_here(duel.interaction.as_ref())
        && duel
            .interaction
            .as_ref()
            .is_some_and(|i| is_sheet(&i.prompt()))
}

/// Whether the sheet stands open over the shelf, which is what takes the
/// shelf's own sentence away (the head says it).
pub(crate) fn sheet_up(duel: &Duel) -> bool {
    sheet_asked(duel) && !duel.decision_fold.is_folded(duel.decision_seq())
}

/// The head, or `None` when no sheet stands.
pub(super) fn head_of(duel: &Duel, lang: Lang, texts: &crate::cardtext::CardTexts) -> Option<Head> {
    if !sheet_asked(duel) {
        return None;
    }
    let view = duel.view.as_ref()?;
    let interaction = duel.interaction.as_ref()?;
    let mut title = super::shelf_headline(duel, lang, texts)?;
    // A cast sent before its mana is still being cast: one sentence says
    // the spell, the question and that the payment follows, rather than a
    // payment line beside a target line.
    if let Some(card) = duel
        .mana_run
        .as_ref()
        .and_then(crate::ManaRun::casting_first)
        && let Some(spell) = view
            .object(card)
            .or_else(|| view.targeting.as_ref().map(|context| &context.source))
    {
        let name = crate::face::name_of(spell, view, texts);
        title = Phrase::CastFirstQuestion.fill(lang, &[&name, &title]);
    }
    let targets = matches!(interaction.pending(), Pending::ChooseTargets { .. });
    let detail = if targets {
        crate::choices::target_question(interaction, view, lang, texts, duel.statics.as_ref())
    } else {
        Vec::new()
    };
    let source = view
        .targeting
        .as_ref()
        .filter(|_| targets)
        .map(|context| Source {
            object: context.source.id,
            art: baylee_client_core::board::art_of(
                &context.source,
                ArtSize::Small,
                crate::cardart::registry(),
            ),
        });
    Some(Head {
        title,
        detail,
        source,
        folded: duel.decision_fold.is_folded(duel.decision_seq()),
    })
}

/// The head: picture, title and the source's words, the fold at the right
/// (`fold`: its action and key cap) and a close cross after it where the
/// sheet can be put away (`close`).
pub(in crate::hud) fn spawn_head(
    commands: &mut Commands,
    fonts: &UiFonts,
    head: &Head,
    picture: Option<Handle<Image>>,
    (fold, cap): (MenuAction, Option<&str>),
    close: Option<MenuAction>,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                column_gap: px(10),
                padding: UiRect::bottom(px(6)),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(crate::shellkit::tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(source) = head.source {
        let thumb = thumbnail(commands, source.object, picture, THUMB_W, THUMB_H);
        commands.entity(row).add_child(thumb);
    }
    let words = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = commands
        .spawn((
            SheetTitle,
            Text::new(head.title.clone()),
            tf_bold(fonts, TITLE_PT),
            TextColor(crate::shellkit::tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(words).add_child(title);
    for line in &head.detail {
        let said = crate::manaui::spawn_rich(
            commands,
            fonts,
            line,
            DETAIL_PT,
            crate::shellkit::tokens::MUTED,
        );
        commands.entity(said).insert(Pickable::IGNORE);
        commands.entity(words).add_child(said);
    }
    let fold = fold_button(commands, fonts, fold, cap, false);
    commands.entity(row).add_children(&[words, fold]);
    if let Some(action) = close {
        let cross = square_button(commands, fonts, action, glyph::CLOSE);
        commands.entity(row).add_child(cross);
    }
    row
}

/// A square control of the head's (the close cross): the fold's own frame.
fn square_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    action: MenuAction,
    mark: char,
) -> Entity {
    let (fill, edge, ink) = PANEL_KEY;
    commands
        .spawn((
            MenuButton { action },
            Node {
                width: px(FOLD),
                height: px(FOLD),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
            Feel::new(fill),
            children![(
                Text::new(mark.to_string()),
                icon_tf(fonts, 11.0),
                TextColor(ink),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// One answer in a sheet's own foot, with its key cap: what a sheet that
/// grows out of no shelf (a reveal) stands its answers on.
pub(in crate::hud) fn footer_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    action: MenuAction,
    label: &str,
    cap: Option<&str>,
) -> Entity {
    let (fill, edge, ink) = PANEL_KEY;
    let button = commands
        .spawn((
            MenuButton { action },
            Node {
                height: px(FOOT_H),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(px(10)),
                column_gap: px(6),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
            Feel::new(fill),
        ))
        .id();
    if let Some(cap) = cap {
        let legend = commands
            .spawn((
                Text::new(cap.to_string()),
                tf(fonts, 10.0),
                TextColor(crate::shellkit::tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(button).add_child(legend);
    }
    let words = commands
        .spawn((
            Text::new(label.to_string()),
            tf_bold(fonts, 12.5),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(button).add_child(words);
    button
}

/// A sheet's foot button's height.
pub(in crate::hud) const FOOT_H: f32 = 28.0;

/// The sheet's title, for the tests and `/state`.
#[derive(Component)]
pub struct SheetTitle;

/// The source's picture: the card's art, pointer-transparent inside a node
/// that previews the source under the pointer and answers no press. Until
/// the art is there (or with none to load) the card's place is a quiet
/// card-sized ground, so the head does not jump when it arrives.
fn thumbnail(
    commands: &mut Commands,
    object: ObjectId,
    image: Option<Handle<Image>>,
    width: f32,
    height: f32,
) -> Entity {
    let slot = commands
        .spawn((
            SheetSource { object },
            ChoicePreview { object },
            Node {
                width: px(width),
                height: px(height),
                flex_shrink: 0.0,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::DOCK_GROUND),
            BorderColor::all(crate::shellkit::tokens::BORDER),
        ))
        .id();
    if let Some(image) = image {
        let art = commands
            .spawn((
                ImageNode::new(image),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(slot).add_child(art);
    }
    slot
}

/// The fold control: a down chevron on an open sheet, an up one on the pill,
/// with its key cap under a pointer.
fn fold_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    action: MenuAction,
    cap: Option<&str>,
    folded: bool,
) -> Entity {
    let (fill, edge, ink) = PANEL_KEY;
    let button = commands
        .spawn((
            MenuButton { action },
            Node {
                height: px(FOLD),
                min_width: px(FOLD),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(px(6)),
                column_gap: px(5),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
            Feel::new(fill),
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new(
                if folded {
                    glyph::CHEVRON_UP
                } else {
                    glyph::CHEVRON_DOWN
                }
                .to_string(),
            ),
            icon_tf(fonts, 11.0),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(button).add_child(mark);
    if let Some(cap) = cap {
        let legend = commands
            .spawn((
                Text::new(cap.to_string()),
                tf(fonts, 10.0),
                TextColor(crate::shellkit::tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(button).add_child(legend);
    }
    button
}

/// The folded sheet: the source's picture, the question, the restore mark.
/// The whole pill restores the sheet (`fold`: the action and its cap);
/// `margin` places it where its sheet's pill stands.
pub(in crate::hud) fn spawn_pill(
    commands: &mut Commands,
    fonts: &UiFonts,
    head: &Head,
    picture: Option<Handle<Image>>,
    (fold, cap): (MenuAction, Option<&str>),
    margin: UiRect,
) -> Entity {
    let pill = commands
        .spawn((
            SheetPill,
            MenuButton { action: fold },
            Node {
                max_width: px(300),
                height: px(PILL_THUMB_H + 10.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                padding: UiRect::axes(px(6), px(4)),
                margin,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(crate::shellkit::tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(crate::shellkit::tokens::OPAQUE),
            BorderColor::all(palette::CANDLE.with_alpha(0.75)),
            Feel::new(crate::shellkit::tokens::OPAQUE),
            BoxShadow(vec![ShadowStyle {
                color: palette::SHADOW,
                x_offset: px(0.0),
                y_offset: px(-3.0),
                spread_radius: px(0.0),
                blur_radius: px(10.0),
            }]),
        ))
        .id();
    fill_pill(commands, fonts, head, picture, cap, pill);
    pill
}

/// Writes the pill's contents into `pill` (which holds none): the source's
/// picture, the question, the restore mark and its cap. Its own function so
/// a pill whose reading changed while it stood is rewritten in place rather
/// than left empty.
pub(in crate::hud) fn fill_pill(
    commands: &mut Commands,
    fonts: &UiFonts,
    head: &Head,
    picture: Option<Handle<Image>>,
    cap: Option<&str>,
    pill: Entity,
) {
    let (_, _, ink) = PANEL_KEY;
    if let Some(source) = head.source {
        let thumb = thumbnail(commands, source.object, picture, PILL_THUMB_W, PILL_THUMB_H);
        // Inside the pill the picture is part of the press: the pill restores.
        commands
            .entity(thumb)
            .remove::<ChoicePreview>()
            .insert(Pickable::IGNORE);
        commands.entity(pill).add_child(thumb);
    }
    let words = commands
        .spawn((
            Text::new(head.title.clone()),
            tf_bold(fonts, PILL_PT),
            TextColor(ink),
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
            Node {
                min_width: px(0),
                flex_shrink: 1.0,
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let restore = commands
        .spawn((
            Text::new(glyph::CHEVRON_UP.to_string()),
            icon_tf(fonts, 11.0),
            TextColor(palette::CANDLE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(pill).add_children(&[words, restore]);
    if let Some(cap) = cap {
        let legend = commands
            .spawn((
                Text::new(cap.to_string()),
                tf(fonts, 10.0),
                TextColor(crate::shellkit::tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(pill).add_child(legend);
    }
}
