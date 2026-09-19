//! The mana pool: the strip on the shelf's left end, and the one row here
//! whose contents arrive and leave.
//!
//! It is [`super::tray`]'s mirror — the same [`super::strip_node`] with its
//! own end of the shelf — and it is **hidden while nothing is floating**:
//! *"Es ist hidden, wenn kein Mana im Mana Pool ist und ist nur dann
//! sichtbar, wenn dort Mana drin ist"* (the owner, 19.09.2026). That
//! overturns this file's own rule for the second time, and the turn is worth
//! recording rather than quietly deleting. The chip this began as was drawn
//! "while the seat has something to answer" and blinked out at every
//! opponent's priority; the column that replaced it stood always, on the
//! argument that the shelf's left edge was *reserved* whatever was on it, so
//! leaving it occupied cost nothing. Off the shelf that argument is gone with
//! the reservation — a strip over the table is a floating box again, and a
//! floating box with nothing in it is the piece of table the chip's rule was
//! written about.
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
//! So the strip is **retained**. It is spawned once beside the shelf,
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

/// The retained left strip.
///
/// A marker on the strip and not on a row inside it, because there is no row
/// inside it: the label and the entries are both children of this one node,
/// laid out by its own `column_gap`.
#[derive(Component)]
pub struct PoolStrip;

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

/// The label the row opens with.
#[derive(Component)]
pub struct PoolLabel;

/// Where the **strip** is in its own arrival or departure.
///
/// A component on the strip and not a resource, which is the shape
/// [`super::drawer::DrawerZoom`] already has and is worth saying why: a
/// resource is a third thing every test harness that runs this system has to
/// be told about, and a missing one is a runtime panic that `cargo check` and
/// clippy are both green over. There is one strip; its movement belongs to
/// it.
///
/// It is spawned **shut** — `t` at the end of a close — because the strip is
/// spawned hidden and a fresh `Default` would read as the first frame of an
/// arrival that nobody asked for.
#[derive(Component)]
pub struct StripZoom {
    /// 0 at the start of the movement, 1 at its end.
    t: f32,
    /// Whether this is the way out.
    closing: bool,
}

impl Default for StripZoom {
    fn default() -> Self {
        Self {
            t: 1.0,
            closing: true,
        }
    }
}

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

/// Spawns the strip, once, beside the shelf.
///
/// What this seat has floating, hanging off the shelf's **left** end — the
/// mirror of [`super::tray`], which the owner asked for in those words:
/// *"Der Manavorrat soll auch ein repositioneng bekommen. Es soll symetrisch
/// zum Tray aussehen nur auf der linken Seite"* (19.09.2026). Both strips
/// spawn [`super::strip_node`], so the symmetry is one function rather than
/// two files that agree today.
///
/// It was a *column on* the shelf before that, and the one zone with no card
/// in it before that. That first absence hid a defect rather than merely
/// being untidy: a land with two mana abilities taps for whichever one the
/// client's planner can read, and with nothing drawn there was no way to see
/// which had fired — Jasmine Dragon Tea Shop made `{C}` every time and looked
/// exactly like a land making the Ally mana it had been tapped for.
///
/// It stands at [`Z_TRAY`] and not at [`Z_LEDGE`], which is a judgment by
/// symmetry with one argument of its own: a maximised sheet reaches over the
/// whole band, and the mana a player is holding is most worth reading while
/// they are spending it — which is exactly when a zone dialog may be open.
///
/// What the chip decided and this keeps: the count is a **numeral** beside
/// the disc and never a row of repeated discs — colour alone must not carry
/// meaning, and six discs is a number a player has to stop and count.
pub(in crate::hud) fn spawn_pool_strip(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            PoolStrip,
            StripZoom::default(),
            Node {
                column_gap: px(POOL_ENTRY_GAP),
                ..strip_node(StripSide::Left)
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            ZIndex(Z_TRAY),
            // Spawned **hidden**, because the first frame of a game has an
            // empty pool and a strip that appeared and went away again
            // before anything was floating is exactly the movement carrying
            // no information §4.1 forbids. `sync_pool` shows it on the frame
            // the first mana arrives.
            Visibility::Hidden,
            // Nothing in here is a control, so the whole strip is passed
            // over by the pointer — the tray's root is ignored for the
            // sharper version of the same reason, that the button inside it
            // is the control and the padding around it is not.
            Pickable::IGNORE,
        ))
        .id()
}

/// Fills the pool row, shows and hides the strip, and starts every arrival
/// and departure in it.
///
/// Nothing here is spawned and despawned any more. The entries are reconciled
/// by [`PoolEntry`], which is the part that has to survive a fade; the label
/// is kept for a plainer reason — it says the same word on almost every call,
/// and a text node respawned that often is a blank frame waiting for the day
/// a glyph takes a frame to shape — and it is rebuilt only when the interface
/// changes language, which is the one thing that can change what it says.
///
/// The em dash that used to stand for an empty pool is gone, and the strip's
/// own `Visibility` took its job: *"Es ist hidden, wenn kein Mana im Mana
/// Pool ist und ist nur dann sichtbar, wenn dort Mana drin ist"* (the owner,
/// 19.09.2026). The two are one substitution rather than two changes — the
/// dash appeared under exactly the condition the strip now hides under, so a
/// dash drawn inside a hidden strip would be a node nothing could ever see.
#[allow(clippy::too_many_arguments)] // one retained row, like the shelf's own
pub fn sync_pool(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    mut revision: ResMut<PoolRevision>,
    mut strip: Query<(Entity, Option<&Children>, &Visibility, &mut StripZoom), With<PoolStrip>>,
    mut entries: Query<(&PoolEntry, &mut PipZoom)>,
    kids: Query<&Children>,
    mut counts: Query<&mut Text, With<PoolCount>>,
    labels: Query<(), With<PoolLabel>>,
    painted: Painted,
) {
    let Ok((column, standing, seen, mut fold)) = strip.single_mut() else {
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
    // The strip waits for the last fade, so it is never taken off the screen
    // around a pip that is still on it — and `live` is the third of those
    // three, the one that is easy to leave out. When the pool names nothing,
    // every live entry is about to be marked closing *by this very call*, so
    // reading `leaving` alone answers "is the row empty" on the one frame
    // where the row is at its fullest. Left out of the em dash this replaces,
    // it put the dash on screen for a single frame at the moment of spending
    // and took it away again, which is the movement carrying no information
    // that §4.1 forbids — and left out here it would take the whole strip
    // away over a pip in the middle of its fade.
    let empty = wanted.is_empty() && leaving.is_empty() && live.is_empty();
    // **Showing**, not visible: a strip in the middle of folding away is still
    // on the screen and is already answered for, so reading `Visibility` alone
    // would start the same close on every frame until it finished and reset
    // `t` each time — a fold that never gets past its first frame. It is the
    // same pair `hud::tray`'s `showing = drawn && !closing` is, one level
    // down.
    let showing = *seen != Visibility::Hidden && !fold.closing;
    // The second half is the tree, as everywhere on this shelf: an entry that
    // has finished fading is despawned by `zoom_the_pool` and leaves a
    // reading that is still true and a row that is no longer what it says.
    if *revision == next && shown == wanted && showing != empty {
        return;
    }
    let relabel = revision.lang != next.lang;
    *revision = next;

    // Which way the strip is going. Nothing here shows or hides it —
    // `grow_the_pool` does both at the ends of the movement, because a strip
    // hidden on the frame the last mana was spent is a fold nobody sees.
    if empty != fold.closing {
        fold.closing = empty;
        fold.t = 0.0;
    }
    let head = head(&mut commands, &children, &labels, &fonts, lang, relabel);

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

    let mut row = vec![head];
    row.extend(ordered.into_iter().map(|(_, entry)| entry));
    commands.entity(column).replace_children(&row);
}

/// Grows the strip out of the shelf, and folds it back into it.
///
/// The counterpart of [`super::drawer::zoom_the_drawer`] and the same curve,
/// because the two are the same kind of thing: a lip of the shelf that is
/// sometimes there. What differs is the corner it is pinned at — the drawer
/// is centred and shrinks toward its own middle, this is fixed at the
/// window's left margin and would appear to *slide* inward if it did the
/// same, so [`motion::from_bottom_left`] holds the corner it grows out of.
///
/// And what differs more usefully: the drawer is despawned at the end of its
/// close and this is only **hidden**. The strip is spawned once with the
/// overlay's root and nothing would ever build it again — the same trap the
/// tray would have fallen into — so the end of the fold is a `Visibility`
/// and not a `despawn`.
///
/// The pips inside run their own [`PipZoom`] on the same curve over the same
/// [`motion::ZOOM_IN`], so a strip arriving with its first mana is two
/// movements at once. They are not fought over: both start together, both
/// end together, and the compounded scale reads as one thing arriving rather
/// than as a pip that is late.
pub fn grow_the_pool(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut strips: Query<(&mut StripZoom, &mut UiTransform, &mut Visibility), With<PoolStrip>>,
) {
    let still = prefs.all().reduce_motion;
    for (mut fold, mut transform, mut seen) in &mut strips {
        // At rest, either way. Written as an early continue rather than let
        // the arithmetic run over it, because `Mut` writes on every deref and
        // a strip that is simply standing there would mark itself changed on
        // every frame of every game.
        if fold.t >= 1.0 {
            continue;
        }
        let span = if fold.closing {
            motion::ZOOM_OUT
        } else {
            motion::ZOOM_IN
        };
        fold.t = motion::step(fold.t, span, time.delta_secs(), still);
        if fold.closing {
            if fold.t >= 1.0 {
                *seen = Visibility::Hidden;
                continue;
            }
        } else if *seen != Visibility::Inherited {
            *seen = Visibility::Inherited;
        }
        let scale = if fold.closing {
            motion::shutting(fold.t)
        } else {
            motion::opening(fold.t)
        };
        transform.scale = Vec2::splat(scale);
        transform.translation = motion::from_bottom_left(scale);
    }
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

/// The word the row opens with, kept from the last call where it can be.
///
/// A label is the one node here that says the same thing on almost every call,
/// so it is the one node worth not respawning: `relabel` is true only when the
/// interface has changed language, and that is the only thing that can change
/// what it says.
fn head(
    commands: &mut Commands,
    standing: &[Entity],
    labels: &Query<(), With<PoolLabel>>,
    fonts: &UiFonts,
    lang: Lang,
    relabel: bool,
) -> Entity {
    match standing.iter().copied().find(|c| labels.contains(*c)) {
        Some(kept) if !relabel => kept,
        was => {
            if let Some(leaving) = was {
                commands.entity(leaving).despawn();
            }
            label(commands, fonts, lang)
        }
    }
}

/// The row's own word, which stands whether or not anything is floating.
fn label(commands: &mut Commands, fonts: &UiFonts, lang: Lang) -> Entity {
    commands
        .spawn((
            PoolLabel,
            Text::new(Phrase::ManaPool.text(lang).to_string()),
            tf(fonts, POOL_LABEL_PT),
            // The ink stayed and the **ground** moved out from under it, so
            // the pair was measured again rather than carried over: this used
            // to be `palette::LEDGE_SOFT` on the shelf's own translucent
            // ground, and it is the same ink on the strip's opaque
            // `DIALOG_LIT`, where it reads 5.45 : 1 and clears the 4.5 prose
            // is held to. `DIALOG_SOFT` would be the register-consistent
            // choice beside the tray's icon and is 4.28 : 1 — under that
            // bound, because the tray's glyph is a *mark* held to 3.0 and
            // this is a word. The restricted rim further down stays
            // `DIALOG_SOFT` for exactly that reason: it is a mark too.
            TextColor(palette::LEDGE_SOFT),
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
