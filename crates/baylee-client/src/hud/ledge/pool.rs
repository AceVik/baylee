//! The mana pool: the one row on this shelf whose contents arrive and leave.
//!
//! §4.1 gives it a movement — "a new entry pops in 160 ms ease-out-back from
//! 0.88, one that is spent fades out in 100 ms, and a number that changes
//! jumps" — and step 4 could not build it. The reason is written down there
//! and is the whole shape of this file: [`super::sync_ledge`] despawns every
//! child of the shelf on each rebuild, so there was no entity that outlived
//! the change that was supposed to be animated. A pip that popped would pop
//! again on the next sentence, and a pip that was spent would simply be
//! absent on the next frame.
//!
//! So the left column is **retained**. It is spawned once with the shelf,
//! skipped by the rebuild, and filled by [`sync_pool`] against a
//! [`PoolRevision`] of its own — the fourth time this client has reached for
//! that answer, after `HudRevision`, `BarRevision` and `DrawerRevision`, and
//! for the same reason each time: two things change on different clocks and a
//! single counter has to lie about one of them.
//!
//! What that buys is worth naming, because the cheaper version looks almost
//! identical. Seeding a fresh row's `t` from the previous reading would give
//! the same *first* frame, and then any unrelated rebuild inside the next
//! 160 ms — the sentence changes at every priority — would cut the movement
//! short or take a fading entry off the screen early. Here an entry is one
//! entity from the moment it arrives to the moment it is gone, and nothing
//! about the shelf around it can interrupt that.
//!
//! The reconciliation is by **what the mana is**, not by position:
//! [`PoolEntry`] is the colour and whether it is restricted, which is exactly
//! the key `baylee_client_core::manapool::row` groups by. Order comes back out
//! of [`PoolEntry::rank`] rather than out of the list, so an entry on its way
//! out keeps its place in WUBRG order instead of being shuffled to the end
//! while it fades.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

use baylee_client_core::manapool::Floating;
use baylee_core::mana::ManaColor;

/// The retained left column.
///
/// A marker on the column and not on the row inside it, because there is no
/// row inside it: the label, the em dash and the entries are all children of
/// this one node, laid out by its own `column_gap`.
#[derive(Component)]
pub struct PoolColumn;

/// Which mana an entry is about.
///
/// The reconciler's key. Two entries of one colour exist at once — the plain
/// mana and the restricted mana (CR 106.6) — and they are two entries that
/// arrive and leave independently, so the restriction is part of the identity
/// rather than a property of it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct PoolEntry {
    color: ManaColor,
    restricted: bool,
}

impl PoolEntry {
    /// Where this entry stands in the row.
    ///
    /// The same order `manapool::row` writes — WUBRG+C, then the restricted
    /// mana in that same order — computed from the entry rather than read off
    /// the list, so an entry that is no longer in the list still knows where
    /// it belongs. That is the whole reason a fading pip does not jump to the
    /// end of the row on the frame it is spent.
    fn rank(self) -> usize {
        usize::from(self.restricted) * ManaColor::ALL.len() + self.color.index()
    }
}

/// The numeral beside a pip, so a count that changes can be written in place.
///
/// §4.1: a number that changes jumps. Numbers are read, not watched.
#[derive(Component)]
pub struct PoolCount;

/// The em dash of an empty pool.
#[derive(Component)]
pub struct PoolDash;

/// The label the row opens with.
#[derive(Component)]
pub struct PoolLabel;

/// Where an entry is in its own arrival or departure.
#[derive(Component, Default)]
pub struct PipZoom {
    /// 0 at the start of the movement, 1 at its end.
    t: f32,
    /// Whether this is the way out.
    closing: bool,
}

/// What a node under a departing entry was coloured.
///
/// A fade needs something to fade *from*, and a UI node has no opacity of its
/// own — so the colours are read off the subtree on the frame the entry is
/// spent and multiplied down from there. Captured rather than recomputed
/// because a pip's own colours belong to `crate::manaui` and are none of this
/// file's business.
#[derive(Component)]
pub struct Lit {
    fill: Option<Color>,
    edge: Option<BorderColor>,
    ink: Option<Color>,
}

/// Everything on the row that is not an entry: the word and the em dash.
///
/// An alias because the pair is named in two places and `clippy::pedantic`
/// counts a nested `Or` as a type worth naming — which it is.
type Furniture<'w, 's> = Query<'w, 's, (), Or<(With<PoolLabel>, With<PoolDash>)>>;

/// Every colour a node is wearing, of the three a node can wear.
type Painted<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static BackgroundColor>,
        Option<&'static BorderColor>,
        Option<&'static TextColor>,
    ),
>;

/// What the pool row is showing.
///
/// The language is in it because the label is a word; the counts are in it
/// because a count that changes is written even though nothing arrives or
/// leaves.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct PoolRevision {
    pool: Vec<Floating>,
    lang: Option<Lang>,
}

/// Spawns the column, once, with the shelf.
///
/// The left column: what this seat has floating, on the shelf's left edge.
///
/// The one zone with no card in it, and until the chip it stood in there was
/// nowhere on screen for it at all. That absence hid a defect rather than
/// merely being untidy: a land with two mana abilities taps for whichever one
/// the client's planner can read, and with nothing drawn there was no way to
/// see which had fired — Jasmine Dragon Tea Shop made `{C}` every time and
/// looked exactly like a land making the Ally mana it had been tapped for.
///
/// **The label always stands**, empty pool or not, watching seat or not, and
/// that overturns the chip's own documented rule ("drawn while the seat has
/// something to answer, hidden when it is only watching"). The rule was right
/// for a box floating over the table, where an empty pool cost the board a
/// piece of itself; the shelf is a *place*, its left edge is reserved whatever
/// stands on it ([`super::LEFT_RESERVED`]), and a label that blinked in and
/// out at every priority would be movement carrying no information. What the
/// chip decided and this keeps: the count is a **numeral** beside the disc and
/// never a row of repeated discs — colour alone must not carry meaning, and
/// six discs is a number a player has to stop and count.
pub(in crate::hud) fn spawn_pool_column(commands: &mut Commands) -> Entity {
    commands.spawn((PoolColumn, column_node(Side::Left))).id()
}

/// Fills the pool row, and starts every arrival and departure in it.
///
/// Everything but the entries is rebuilt — the label is one node and the em
/// dash is two states of one — and the entries are reconciled by
/// [`PoolEntry`], which is the part that has to survive.
#[allow(clippy::too_many_arguments)] // one retained row, like the shelf's own
pub fn sync_pool(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    mut revision: ResMut<PoolRevision>,
    column: Query<(Entity, Option<&Children>), With<PoolColumn>>,
    mut entries: Query<(&PoolEntry, &mut PipZoom)>,
    kids: Query<&Children>,
    mut counts: Query<&mut Text, With<PoolCount>>,
    furniture: Furniture,
    dashes: Query<(), With<PoolDash>>,
    painted: Painted,
) {
    let Ok((column, standing)) = column.single() else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    let next = PoolRevision {
        pool: duel
            .view
            .as_ref()
            .and_then(|v| v.seat(v.seat))
            .map(|s| baylee_client_core::manapool::row(&s.mana_pool))
            .unwrap_or_default(),
        lang: Some(lang),
    };

    let children: Vec<Entity> = standing.into_iter().flatten().copied().collect();
    // Everything the row is carrying, split by which way it is going. A
    // leaving entry is still on the screen and still takes up its slot, and
    // it is deliberately *not* a candidate for the key it used to hold: mana
    // spent and then made again is an arrival, not a change of mind.
    let mut live: Vec<(Entity, PoolEntry)> = Vec::new();
    let mut leaving: Vec<(Entity, PoolEntry)> = Vec::new();
    for &child in &children {
        if let Ok((key, zoom)) = entries.get(child) {
            if zoom.closing {
                leaving.push((child, *key));
            } else {
                live.push((child, *key));
            }
        }
    }
    let wanted: Vec<PoolEntry> = next
        .pool
        .iter()
        .map(|f| PoolEntry {
            color: f.color,
            restricted: f.restricted,
        })
        .collect();
    let shown: Vec<PoolEntry> = live.iter().map(|(_, key)| *key).collect();
    // The dash waits for the last fade, so "nothing floating" is never said
    // over a pip that is still on screen saying otherwise.
    let dash_wanted = wanted.is_empty() && leaving.is_empty();
    let dashed = children.iter().any(|c| dashes.get(*c).is_ok());
    // The second half is the tree, as everywhere on this shelf: an entry that
    // has finished fading is despawned by `zoom_the_pool` and leaves a
    // reading that is still true and a row that is no longer what it says.
    if *revision == next && shown == wanted && dashed == dash_wanted {
        return;
    }
    *revision = next;

    for &child in &children {
        if furniture.get(child).is_ok() {
            commands.entity(child).despawn();
        }
    }

    let mut ordered: Vec<(usize, Entity)> = Vec::new();
    for floating in &revision.pool {
        let key = PoolEntry {
            color: floating.color,
            restricted: floating.restricted,
        };
        let entry = if let Some((entry, _)) = live.iter().find(|(_, k)| *k == key) {
            // Already there. Only the numeral can have changed, and a numeral
            // jumps.
            let numeral = format!("\u{00d7}{}", floating.count);
            let written = kids
                .get(*entry)
                .into_iter()
                .flatten()
                .copied()
                .find(|c| counts.contains(*c));
            if let Some(written) = written
                && let Ok(mut text) = counts.get_mut(written)
                && text.0 != numeral
            {
                text.0 = numeral;
            }
            *entry
        } else {
            spawn_entry(&mut commands, &fonts, floating)
        };
        ordered.push((key.rank(), entry));
    }
    for (entry, key) in &leaving {
        ordered.push((key.rank(), *entry));
    }
    // Anything still live that the pool no longer names is spent.
    for (entry, key) in &live {
        if wanted.contains(key) {
            continue;
        }
        if let Ok((_, mut zoom)) = entries.get_mut(*entry) {
            zoom.closing = true;
            zoom.t = 0.0;
        }
        capture(&mut commands, *entry, &kids, &painted);
        ordered.push((key.rank(), *entry));
    }
    ordered.sort_by_key(|(rank, _)| *rank);

    let mut row = vec![label(&mut commands, &fonts, lang)];
    if dash_wanted {
        row.push(dash(&mut commands, &fonts));
    }
    row.extend(ordered.into_iter().map(|(_, entry)| entry));
    commands.entity(column).replace_children(&row);
}

/// Advances every arrival and departure, and takes a spent entry off the row.
///
/// The arrival is the sheet's own curve, which §4.1 borrows on purpose — a
/// mana has *arrived*, and that is the one thing on this shelf that is an
/// arrival rather than a change. The departure is not the same curve
/// backwards: it is a fade, because a pip that shrank away would be a second
/// movement to watch for something that is merely gone.
#[allow(clippy::too_many_arguments)] // three colours, and a fade needs all of them
pub fn zoom_the_pool(
    mut commands: Commands,
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut pips: Query<(Entity, &mut PipZoom, &mut UiTransform)>,
    kids: Query<&Children>,
    mut fill: Query<&mut BackgroundColor>,
    mut edge: Query<&mut BorderColor>,
    mut ink: Query<&mut TextColor>,
    lit: Query<&Lit>,
) {
    let still = prefs.all().reduce_motion;
    for (entry, mut zoom, mut transform) in &mut pips {
        let span = if zoom.closing {
            motion::ZOOM_OUT
        } else {
            motion::ZOOM_IN
        };
        zoom.t = motion::step(zoom.t, span, time.delta_secs(), still);
        if zoom.closing {
            if zoom.t >= 1.0 {
                commands.entity(entry).despawn();
                continue;
            }
            // The whole subtree, node by node, from what it was.
            let left = 1.0 - zoom.t;
            let mut stack = vec![entry];
            while let Some(node) = stack.pop() {
                stack.extend(kids.get(node).into_iter().flatten().copied());
                let Ok(was) = lit.get(node) else { continue };
                if let Some(colour) = was.fill
                    && let Ok(mut now) = fill.get_mut(node)
                {
                    now.0 = colour.with_alpha(colour.alpha() * left);
                }
                if let Some(colour) = was.edge
                    && let Ok(mut now) = edge.get_mut(node)
                {
                    *now = BorderColor {
                        top: colour.top.with_alpha(colour.top.alpha() * left),
                        right: colour.right.with_alpha(colour.right.alpha() * left),
                        bottom: colour.bottom.with_alpha(colour.bottom.alpha() * left),
                        left: colour.left.with_alpha(colour.left.alpha() * left),
                    };
                }
                if let Some(colour) = was.ink
                    && let Ok(mut now) = ink.get_mut(node)
                {
                    now.0 = colour.with_alpha(colour.alpha() * left);
                }
            }
            continue;
        }
        transform.scale = Vec2::splat(motion::opening(zoom.t));
    }
}

/// Records what a subtree is coloured, so the fade has a start.
///
/// One walk on the frame the mana is spent, and from then on `zoom_the_pool`
/// multiplies these down. Read off the tree rather than rebuilt from the
/// palette because a pip's own colours belong to [`crate::manaui`], and a fade
/// that knew them would be a second place to change when a symbol changes.
fn capture(commands: &mut Commands, entry: Entity, kids: &Query<&Children>, painted: &Painted) {
    let mut stack = vec![entry];
    while let Some(node) = stack.pop() {
        stack.extend(kids.get(node).into_iter().flatten().copied());
        let Ok((fill, edge, ink)) = painted.get(node) else {
            continue;
        };
        if fill.is_none() && edge.is_none() && ink.is_none() {
            continue;
        }
        commands.entity(node).insert(Lit {
            fill: fill.map(|c| c.0),
            edge: edge.copied(),
            ink: ink.map(|c| c.0),
        });
    }
}

/// The row's own word, which stands whether or not anything is floating.
fn label(commands: &mut Commands, fonts: &UiFonts, lang: Lang) -> Entity {
    commands
        .spawn((
            PoolLabel,
            Text::new(Phrase::ManaPool.text(lang).to_string()),
            tf(fonts, POOL_LABEL_PT),
            TextColor(palette::DIALOG_SOFT),
            Node {
                // The column's own gap is the step between entries; the label
                // is not one of them and takes the wider step of §4.1.
                margin: UiRect::right(px(POOL_LABEL_GAP - POOL_ENTRY_GAP)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// An em dash rather than a row of zeroes: "nothing floating" is one fact, not
/// six. Set at the numerals' size because it stands where a numeral would, and
/// in the one ink on this shelf that means absence.
fn dash(commands: &mut Commands, fonts: &UiFonts) -> Entity {
    commands
        .spawn((
            PoolDash,
            Text::new("\u{2014}".to_string()),
            tf(fonts, POOL_COUNT_PT),
            TextColor(palette::LEDGE_DEAD),
            Pickable::IGNORE,
        ))
        .id()
}

/// One entry: a pip and how many of it.
fn spawn_entry(commands: &mut Commands, fonts: &UiFonts, floating: &Floating) -> Entity {
    let group = commands
        .spawn((
            PoolEntry {
                color: floating.color,
                restricted: floating.restricted,
            },
            PipZoom::default(),
            // Already small, so the first frame it is drawn is the first
            // frame of its arrival rather than a flash at full size.
            UiTransform::from_scale(Vec2::splat(motion::ZOOM_FROM)),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(POOL_PIP_GAP),
                // A restriction is drawn as a rim round the **pair** and
                // not round the disc, because the symbol has to keep
                // meaning its own colour: this mana *is* white, it simply
                // cannot pay for everything white pays for. §4.1 changes
                // the rim's *colour* and leaves that reasoning where it
                // was: the chip drew this rim in the brass of an active
                // card, which §3.2 names with this very pip as its
                // example — brass is a light at a card's edge.
                padding: UiRect::axes(px(3), px(1)),
                // Always a border, coloured only when there is something
                // to say: `BoxSizing::BorderBox` takes the border out of
                // the content box, so a rim that appeared would otherwise
                // narrow the entry it appeared on and shuffle the row.
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BorderColor::all(if floating.restricted {
                palette::DIALOG_SOFT
            } else {
                Color::NONE
            }),
            Pickable::IGNORE,
        ))
        .id();
    let pip = crate::manaui::spawn_pip(commands, fonts, floating.pip, POOL_PIP);
    let count = commands
        .spawn((
            PoolCount,
            Text::new(format!("\u{00d7}{}", floating.count)),
            tf_bold(fonts, POOL_COUNT_PT),
            TextColor(palette::DIALOG_INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(group).add_children(&[pip, count]);
    group
}
