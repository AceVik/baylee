//! Held Shift over a Tower item's overlay: the item at every level, and what
//! each level changes or adds.
//!
//! At rest the overlay wears one quiet line at its foot (the Shift key and
//! "Compare levels"); only a relic has levels, so nothing else wears it.
//! While Shift is held, a ladder stands under the description: one block per
//! level, its head `Level N · rarity` on the rarity's pip, the item's own level
//! in the kit's accent with a "current" tag, levels below it quieter. Under
//! each head the level's effect lines, against the level before
//! (`baylee_client_core::echoes::ladder`): a changed number bold in the
//! accent, a line the level before lacks behind a `+`, a line it says word
//! for word quiet.
//!
//! The ladder is built the first time Shift is held over its overlay (a shop
//! holds dozens of overlays nobody compares) and then only shown or hidden.
//! Either way the overlay changes size, so it is hidden for that frame and
//! the measured path (`shellkit::tooltip::place_detail_tooltips`) places it
//! again from its real geometry before it is drawn: no frame at the old
//! anchor, no frame off screen. There is no animation (the overlay has none),
//! so `reduce_motion` has nothing to hold still.
use super::{color, prose, words};
use crate::{
    hud::UiFonts,
    shellkit::{
        ShellMetrics,
        controls::{self, FloatingTooltip, Kit},
        px_fixed,
        surfaces::whole_pixels,
        tokens,
    },
};
use baylee_client_core::{
    echoes::{
        domain::v2::{
            catalog::{Definition, Rarity},
            shop::Goods,
        },
        ladder::{self, Change},
    },
    i18n::{Lang, Phrase},
};
use bevy::prelude::*;

/// The overlay's width at rest, in kit pixels.
pub(super) const REST_WIDTH: f32 = 424.0;
/// Its width while the ladder stands: two columns of levels, each about as
/// wide as the overlay's own text at rest.
const OPEN_WIDTH: f32 = 920.0;
/// The share of ink a level below the item's own keeps.
const BELOW: f32 = 0.62;

/// On an overlay whose item has levels: what to build the ladder from, and
/// whether it stands.
#[derive(Component)]
pub(crate) struct Levels {
    definition: Definition,
    metrics: ShellMetrics,
    german: bool,
    /// The ladder's slot under the description.
    slot: Entity,
    /// The foot's hint, put away while the key it names is held.
    hint: Entity,
    built: bool,
    open: bool,
}

/// The ladder's slot: `Display::None` until Shift is held.
#[derive(Component)]
pub(crate) struct Ladder;

/// The hint at the overlay's foot.
#[derive(Component)]
pub(crate) struct LevelsHint;

/// The ladder's empty slot, under the description of a relic's overlay.
pub(super) fn slot(commands: &mut Commands, kit: Kit, tip: Entity) -> Entity {
    commands
        .spawn((
            Ladder,
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(tokens::SPACE[1] + 2.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(tip),
        ))
        .id()
}

/// The foot's hint, and what the ladder will be built from.
pub(super) fn attach(
    commands: &mut Commands,
    kit: Kit,
    tip: Entity,
    slot: Entity,
    definition: Definition,
) {
    let lang = lang(kit.german);
    let row = commands
        .spawn((
            LevelsHint,
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(tokens::SPACE[0] + 2.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(tip),
        ))
        .id();
    let key = crate::hud::shift_keycap(commands, kit.fonts, kit.m.small * 0.82);
    let words = controls::label(
        commands,
        kit,
        Phrase::TowerLevelsHint.text(lang),
        kit.m.small,
        tokens::MUTED,
    );
    commands.entity(row).add_children(&[key, words]);
    commands.entity(tip).insert(Levels {
        definition,
        metrics: kit.m,
        german: kit.german,
        slot,
        hint: row,
        built: false,
        open: false,
    });
}

const fn lang(german: bool) -> Lang {
    if german { Lang::De } else { Lang::En }
}

/// Every level's effect lines, in order.
fn lines(definition: Definition, lang: Lang) -> [Vec<String>; 4] {
    Rarity::ALL.map(|rarity| {
        words::run_effect(
            Definition {
                rarity,
                ..definition
            },
            lang,
        )
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
    })
}

/// Fills the slot: a hairline, then the four levels two by two (1 2 / 3 4).
/// The first level says all its lines; each later one says what it changes
/// and adds, and how many lines it keeps as they were.
fn build(commands: &mut Commands, kit: Kit, slot: Entity, current: Definition) {
    let lang = lang(kit.german);
    let accent = color(Goods::Relic(current));
    commands.spawn((
        Node {
            width: percent(100),
            height: px_fixed(1.0),
            ..default()
        },
        BackgroundColor(accent.with_alpha(0.22)),
        Pickable::IGNORE,
        ChildOf(slot),
    ));
    let gap = kit.m.px(tokens::SPACE[2]);
    let grid = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::FlexStart,
                column_gap: gap,
                row_gap: gap,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(slot),
        ))
        .id();
    let levels = lines(current, lang);
    let size = kit.m.small;
    let gutter = kit.m.px(size * 0.9);
    for (index, rarity) in Rarity::ALL.into_iter().enumerate() {
        let own = rarity == current.rarity;
        let ink = if rarity.tier() < current.rarity.tier() {
            BELOW
        } else {
            1.0
        };
        // Two to a row: each takes just under half, the gap the rest.
        let block = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: percent(45),
                    min_width: px_fixed(0.0),
                    row_gap: kit.m.px(tokens::SPACE[0]),
                    ..default()
                },
                Pickable::IGNORE,
                ChildOf(grid),
            ))
            .id();
        head(commands, kit, block, current, rarity, own, ink, gutter);
        let now: Vec<&str> = levels[index].iter().map(String::as_str).collect();
        let step = index.checked_sub(1).map(|before| {
            let then: Vec<&str> = levels[before].iter().map(String::as_str).collect();
            ladder::step(&then, &now)
        });
        let mut kept = 0;
        for (at, line) in now.iter().enumerate() {
            let said = step.as_ref().and_then(|step| step.lines.get(at));
            let change = said.map(|said| said.change);
            if change == Some(Change::Same) {
                kept += 1;
                continue;
            }
            let marking = prose::Marking {
                marks: said.map_or(&[][..], |said| said.marks.as_slice()),
                accent: tokens::ACCENT.with_alpha(ink),
                ink,
            };
            let mark = if change == Some(Change::Added) {
                "+"
            } else {
                ""
            };
            let words = prose::spawn_marked_line(commands, kit.fonts, line, size, marking);
            row(commands, kit, block, gutter, mark, ink, words);
        }
        if kept > 0 {
            let words = commands
                .spawn((
                    Text::new(Phrase::TowerLevelsKept.text_count(lang, kept)),
                    crate::hud::tf(kit.fonts, whole_pixels(size)),
                    TextColor(tokens::MUTED.with_alpha(tokens::MUTED.alpha() * ink)),
                    Pickable::IGNORE,
                ))
                .id();
            row(commands, kit, block, gutter, "", ink, words);
        }
    }
}

/// One line under a level's head: `mark` in the gutter (`+` for a new line),
/// `words` wrapping in the rest.
fn row(
    commands: &mut Commands,
    kit: Kit,
    block: Entity,
    gutter: Val,
    mark: &str,
    ink: f32,
    words: Entity,
) {
    let row = commands
        .spawn((
            Node {
                align_items: AlignItems::FlexStart,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(block),
        ))
        .id();
    let plus = commands
        .spawn((
            Text::new(mark),
            crate::hud::tf_bold(kit.fonts, whole_pixels(kit.m.small)),
            TextColor(tokens::ACCENT.with_alpha(ink)),
            Node {
                width: gutter,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(column).add_child(words);
    commands.entity(row).add_children(&[plus, column]);
}

/// A level's head: its rarity's pip, `Level N · rarity`, and on the item's
/// own level the accent and a "current" tag.
#[allow(clippy::too_many_arguments)] // a head and how it is inked
fn head(
    commands: &mut Commands,
    kit: Kit,
    block: Entity,
    current: Definition,
    rarity: Rarity,
    own: bool,
    ink: f32,
    gutter: Val,
) {
    let lang = lang(kit.german);
    let pip_ink = color(Goods::Relic(Definition { rarity, ..current }));
    let row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(tokens::SPACE[0] + 2.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(block),
        ))
        .id();
    // The pip stands in the lines' `+` gutter, centred, so heads and lines
    // share one left edge for their words.
    let well = commands
        .spawn((
            Node {
                width: gutter,
                flex_shrink: 0.0,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(row),
        ))
        .id();
    commands.spawn((
        Node {
            width: kit.m.px(7.0),
            height: kit.m.px(7.0),
            border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
            ..default()
        },
        BackgroundColor(pip_ink.with_alpha(pip_ink.alpha() * ink)),
        Pickable::IGNORE,
        ChildOf(well),
    ));
    let size = whole_pixels(kit.m.small);
    let level = Phrase::LevelLong.fill(lang, &[&rarity.tier().to_string()]);
    let title = commands
        .spawn((
            Text::new(level),
            crate::hud::tf_bold(kit.fonts, size),
            TextColor(if own {
                tokens::ACCENT
            } else {
                tokens::INK.with_alpha(tokens::INK.alpha() * ink)
            }),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
            ChildOf(row),
        ))
        .id();
    commands.spawn((
        TextSpan::new(format!(" · {}", words::rarity(rarity, lang))),
        crate::hud::tf(kit.fonts, size),
        TextColor(pip_ink.with_alpha(pip_ink.alpha() * ink)),
        Pickable::IGNORE,
        ChildOf(title),
    ));
    if own {
        let tag = commands
            .spawn((
                Node {
                    padding: UiRect::axes(kit.m.px(6.0), kit.m.px(1.0)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(tokens::ACCENT.with_alpha(0.12)),
                BorderColor::all(tokens::ACCENT.with_alpha(0.5)),
                Pickable::IGNORE,
                ChildOf(row),
            ))
            .id();
        let word = controls::label(
            commands,
            kit,
            Phrase::TowerLevelCurrent.text(lang),
            kit.m.small * 0.86,
            tokens::ACCENT,
        );
        commands.entity(tag).add_child(word);
    }
}

/// Whether held Shift asks for the ladders: either Shift, and the platform's
/// own keyboard is not typing (the way `flip` reads it; Shift alone is no
/// keymap action, so it reads the keys as the preview's turn does).
fn shift_held(keys: Option<&ButtonInput<KeyCode>>) -> bool {
    keys.is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]))
        && !crate::softkeys::SoftKeyboard::owns_typing()
}

/// The overlays that have levels.
type Tips<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Levels,
        &'static mut Visibility,
        &'static mut Node,
    ),
    (With<FloatingTooltip>, Without<Ladder>),
>;

/// One kind of part inside them (`Part`), never another (`Other`).
type Parts<'w, 's, Part, Other> =
    Query<'w, 's, &'static mut Node, (With<Part>, Without<Levels>, Without<Other>)>;

/// Opens the ladder on every shown overlay while Shift is held and closes it
/// on release. A frame where neither Shift nor an overlay changed writes
/// nothing.
pub(crate) fn compare_levels(
    mut commands: Commands,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    fonts: Option<Res<UiFonts>>,
    claimed: Option<ResMut<crate::flip::ShiftClaimed>>,
    mut tips: Tips,
    mut slots: Parts<Ladder, LevelsHint>,
    mut hints: Parts<LevelsHint, Ladder>,
) {
    let held = shift_held(keys.as_deref());
    let mut any = false;
    for (mut levels, mut shown, mut node) in &mut tips {
        // Shown by the controls this frame (still hidden while it waits for
        // its first measure, which is the moment to grow it).
        let want = held && node.display == Display::Flex;
        any |= want;
        if want == levels.open {
            continue;
        }
        if want && !levels.built {
            let Some(fonts) = fonts.as_deref() else {
                continue;
            };
            let kit = Kit {
                fonts,
                m: levels.metrics,
                german: levels.german,
            };
            build(&mut commands, kit, levels.slot, levels.definition);
            levels.built = true;
        }
        levels.open = want;
        let (ladder, hint) = if want {
            (Display::Flex, Display::None)
        } else {
            (Display::None, Display::Flex)
        };
        if let Ok(mut slot) = slots.get_mut(levels.slot)
            && slot.display != ladder
        {
            slot.display = ladder;
        }
        if let Ok(mut foot) = hints.get_mut(levels.hint)
            && foot.display != hint
        {
            foot.display = hint;
        }
        // Two columns of levels while it stands, the reading width at rest.
        let width = levels
            .metrics
            .px(if want { OPEN_WIDTH } else { REST_WIDTH });
        if node.width != width {
            node.width = width;
        }
        // The overlay changes size: hidden until the measured path places it
        // from its new geometry, later this same frame.
        shown.set_if_neq(Visibility::Hidden);
    }
    if let Some(mut claimed) = claimed
        && claimed.0 != any
    {
        claimed.0 = any;
    }
}

/// Registers [`compare_levels`] once, whichever screen installs the Tower
/// interface first.
pub(super) fn install(app: &mut App) {
    if app.world().contains_resource::<crate::flip::ShiftClaimed>() {
        return;
    }
    app.init_resource::<crate::flip::ShiftClaimed>()
        .add_systems(
            Update,
            compare_levels
                .after(controls::show_tooltips)
                .before(crate::flip::turn),
        );
}
