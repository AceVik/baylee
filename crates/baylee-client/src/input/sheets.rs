//! The tray and sheet widgets, dragging and gliding a sheet, resizing the preview.

#[allow(clippy::wildcard_imports)] // the input module's shared vocabulary
use super::*;

/// Every widget the zone browser puts on screen, as one system parameter.
///
/// Bundled rather than four more arguments because `pointer` already sits at
/// Bevy's parameter limit — and because they are one thing: the tray, its
/// tabs, its close button, its filter box and its sort control.
///
/// The settings store rides along for the same reason and is the one member
/// that is not a query: the view buttons are the only control on this panel
/// whose state does not live on the `Browser`, and `pointer` has no room left
/// to be handed it separately.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TrayWidgets<'w, 's> {
    pub(super) cards: Query<'w, 's, &'static TrayCard>,
    /// The end of each pile of an arrangement, while a card is held.
    pub(super) slots: Query<'w, 's, &'static crate::hud::ArrangeSlot>,
    pub(super) tabs: Query<'w, 's, &'static TrayTab>,
    pub(super) close: Query<'w, 's, &'static TrayMinimise>,
    /// The tray's own button, which is not on the sheet at all — it
    /// stands on the ledge whether the sheet is up or down. It is in
    /// this bundle rather than beside it because what it operates is
    /// this panel, and a reader looking for who opens the browser
    /// should find every door in one place.
    pub(super) zones: Query<'w, 's, &'static crate::hud::TrayZones>,
    pub(super) sort: Query<'w, 's, &'static TraySort>,
    pub(super) views: Query<'w, 's, &'static crate::hud::TrayView>,
    pub(super) filter: Query<'w, 's, &'static TrayFilter>,
    pub(super) gear: Query<'w, 's, &'static crate::hud::TrayGear>,
    /// Every button the filter builder draws, in one query.
    ///
    /// One and not fourteen, because the model already named what each one
    /// means: `crate::filterui` puts a `FilterAct` on every button it draws
    /// and this hands it straight to the `Browser`. A control added to the
    /// builder is therefore wired by being drawn, which is the whole point of
    /// the vocabulary — this client has shipped a decision function no button
    /// reached.
    pub(super) acts: Query<'w, 's, &'static crate::filterui::FilterAct>,
    pub(super) done: Query<'w, 's, &'static crate::filterui::FilterDone>,
    pub(super) cancel: Query<'w, 's, &'static TrayNone>,
    pub(super) settings: ResMut<'w, crate::settings::ClientSettings>,
    /// The sheet's other size button, and the two things it takes to answer
    /// it. A placement is only meaningful against the band it was measured
    /// in, so the window comes with the button rather than being fetched by
    /// whoever happens to need it — `Placement::maximised`, `is_maximised`
    /// and `fit` all take one, and a band read from somewhere else is a sheet
    /// that maximises to the wrong rectangle.
    pub(super) grow: Query<'w, 's, &'static crate::hud::TrayMaximise>,
    pub(super) windows: Query<'w, 's, &'static Window>,
    /// The HUD's scale, which the band is measured under.
    pub(super) ui: Option<Res<'w, UiScale>>,
    pub(super) glide: ResMut<'w, TrayGlide>,
}

/// The sheet on its way between two rectangles.
///
/// Maximising and restoring used to be one assignment — `settings.zone_browser
/// = Some(next)` and the sheet was simply *there* on the next frame. The owner
/// asked for the movement on 19.09.2026: *"Das Maximiere und reverse soll auch
/// schön animiert sein"*.
///
/// What travels is the **rectangle**, not a `UiTransform`. A transform scales
/// what is already laid out, so a sheet stretched from 900×738 to the band's
/// shape would carry its type, its thumbnails and its row heights with it and
/// arrive as a distorted picture that snaps straight at the end. Writing
/// `Placement::lerp` into the `Node` re-lays the sheet out on every frame,
/// which is what makes the rows stay rows the whole way across.
///
/// The target is written to the store on the **first** frame, not the last:
/// `settings.zone_browser` is where a rebuild reads the sheet's rectangle
/// from, and a store that still held the old one would put the sheet back
/// where it started if a card arrived mid-flight. So the store says where the
/// sheet is going and this says where it is — the same split `tray_drag`
/// already keeps, and the reason `write_placement` exists at all.
/// It is one `Option` and not three fields with a flag among them, because
/// "nothing is moving" has no *from* and no *to* — a resting glide holding
/// two rectangles would need a zero `Placement` that is not a rectangle any
/// sheet could be at, and `Placement` deliberately has no `Default` for that
/// reason.
#[derive(Resource, Default)]
pub struct TrayGlide(pub(super) Option<Flight>);

impl TrayGlide {
    /// Whether the sheet is between two rectangles right now.
    ///
    /// The one thing about this resource anybody outside needs to ask, and
    /// the reason it is asked at all: "the store holds the band" and "the
    /// sheet is on its way to the band" are two different claims, and a test
    /// that only checked the first would pass on the jump this movement
    /// replaced.
    #[must_use]
    pub fn is_flying(&self) -> bool {
        self.0.is_some()
    }
}

/// One movement between two rectangles.
pub(super) struct Flight {
    /// Where the sheet set off from.
    pub(super) from: Placement,
    /// Where it is going, which is also what the store already holds.
    pub(super) to: Placement,
    /// 0 at the start of the movement, 1 at its end.
    pub(super) t: f32,
}

/// Everything on the ability sheet a pointer can land on, bundled for the
/// same reason [`TrayWidgets`] is: `pointer` is at Bevy's parameter limit,
/// and these three are one surface. A row arms, the tenth row turns the
/// page, the cross in the head is the way out.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SheetWidgets<'w, 's> {
    pub(super) rows: Query<'w, 's, &'static AbilityButton>,
    pub(super) pager: Query<'w, 's, &'static crate::hud::SheetPager>,
    pub(super) close: Query<'w, 's, &'static crate::hud::SheetClose>,
}

/// Finds a component on the clicked entity or one of its ancestors —
/// a click on a button's icon or text belongs to the button.
///
/// The query carries its own filter, which every caller but one leaves empty.
/// The exception is [`crate::touch`], and it is why the parameter is here:
/// [`HandCardVisual`] speaks for every card the *HUD* draws, the stack
/// panel's slots included, so a system that is about the hand **row** has to
/// say so — and the query is where saying it also narrows the walk.
pub(crate) fn find_in_lineage<'a, T: Component, F: bevy::ecs::query::QueryFilter>(
    entity: Entity,
    query: &'a Query<'_, '_, &T, F>,
    parents: &Query<&ChildOf>,
) -> Option<&'a T> {
    lineage_bearer(entity, query, parents).map(|(_, found)| found)
}

/// The same walk, answering *which* ancestor carried the component.
///
/// The entity is what a caller needs to ask a second question about the card
/// — where it is on the screen, for one, which is a `GlobalTransform` on that
/// same entity and not on whichever child the pointer happened to land on.
pub(super) fn lineage_bearer<'a, T: Component, F: bevy::ecs::query::QueryFilter>(
    entity: Entity,
    query: &'a Query<'_, '_, &T, F>,
    parents: &Query<&ChildOf>,
) -> Option<(Entity, &'a T)> {
    let mut current = Some(entity);
    for _ in 0..6 {
        let e = current?;
        if let Ok(found) = query.get(e) {
            return Some((e, found));
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}

/// Moves and stretches the zone browser's sheet.
///
/// # Why this is not `Pointer<Drag>`
///
/// Bevy's picking backend has a perfectly good drag gesture, and the lobby
/// uses it. The duel HUD cannot: it is a retained tree rebuilt from scratch
/// whenever [`crate::hud::HudRevision`] changes — a new snapshot, a hover, a
/// selection — so the header entity a drag chain was bound to is despawned
/// mid-gesture and the drag simply stops. A press that records *what* is
/// being held, and a per-frame read of the cursor, survive the rebuild
/// because neither of them holds an entity.
///
/// # Why the geometry is not in the revision
///
/// For the same reason from the other side: routing a drag through
/// `HudRevision` would rebuild two hundred nodes for every pixel of it. The
/// system writes the sheet's own `Node` and the in-memory settings, and the
/// next rebuild — whenever it happens, for whatever reason — reads the
/// settings and lands where the pointer left it. Only the *release* touches
/// the disk.
///
/// There is no easing here and that is deliberate: the house curve is for
/// things that move on their own, and a sheet that lagged the hand dragging
/// it would be wrong at every rate. `reduce_motion` therefore has nothing to
/// gate.
#[allow(clippy::too_many_arguments)] // two message readers, three queries, two stores
pub fn tray_drag(
    mut downs: MessageReader<Pointer<Press>>,
    mut ups: MessageReader<Pointer<Release>>,
    grips: Query<&crate::hud::TrayGrip>,
    corners: Query<&crate::hud::TrayResize>,
    closes: Query<&TrayMinimise>,
    grows: Query<&crate::hud::TrayMaximise>,
    tabs: Query<&TrayTab>,
    parents: Query<&ChildOf>,
    windows: Query<&Window>,
    mut panels: Query<&mut Node, With<crate::hud::TrayPanel>>,
    ui: Option<Res<UiScale>>,
    mut duel: ResMut<Duel>,
    mut settings: ResMut<ClientSettings>,
    mut revision: ResMut<crate::hud::TrayRevision>,
) {
    use crate::hud::{TrayDrag, TrayDragKind};

    // A sheet a *question* opened is not furniture the player arranged: it is
    // centred on whatever window it meets and reads no stored rectangle at all
    // (`Browser::placement`). Dragging it would write a rectangle nobody is
    // looking at into `settings.zone_browser` — the sheet would snap back to
    // the middle at the next rebuild, and the hand-opened sheet would later
    // stand where nobody put it. Only a `ByHand` sheet is furniture.
    if duel.browser.for_choice() {
        duel.tray_drag = None;
        return;
    }

    let cursor = windows
        .single()
        .ok()
        .and_then(|w| crate::hud::scale::cursor(w, ui.as_deref()));
    for down in downs.read() {
        // The minimise button and the zone tabs sit *on* the header, so
        // their lineage carries the grip. The specific control claims the
        // press before the row it stands on does, or putting the sheet away
        // would first nudge it by whatever the hand wobbled between the press
        // and the release — and then save that.
        //
        // The tabs joined that list when they moved into the title row on
        // 14.09.2026. It is the bargain the minimise button already had, and
        // it is the reason the tabs could move at all: a chip that started a
        // drag would carry the whole sheet sideways every time a pile was
        // ticked. The maximise button joined on 19.09.2026 when it moved up
        // out of the corner and into the row beside its neighbour — which is
        // the third time this list has had to grow with the header, and the
        // reason it is one condition over three queries rather than a rule
        // about where a control happens to sit.
        if find_in_lineage(down.entity, &closes, &parents).is_some()
            || find_in_lineage(down.entity, &grows, &parents).is_some()
            || find_in_lineage(down.entity, &tabs, &parents).is_some()
        {
            continue;
        }
        let kind = if find_in_lineage(down.entity, &corners, &parents).is_some() {
            Some(TrayDragKind::Resize)
        } else if find_in_lineage(down.entity, &grips, &parents).is_some() {
            Some(TrayDragKind::Move)
        } else {
            None
        };
        // A press with no cursor is a press from a harness that never moved
        // one; starting a drag from it would take the first real cursor
        // position as a delta and throw the sheet across the band.
        if let (Some(kind), Some(at)) = (kind, cursor) {
            duel.tray_drag = Some(TrayDrag {
                kind,
                origin: at,
                last: at,
            });
        }
    }
    // Whether a drag *ended* this frame, which is not the same as whether the
    // button came up: a release with no drag under it is a click somewhere
    // else on the sheet and has nothing to write. The drag itself is no
    // longer wanted — it was read for how far it had travelled, back when the
    // corner had a second job.
    let mut ended = None;
    for _up in ups.read() {
        ended = ended.or_else(|| duel.tray_drag.take());
    }

    if let (Some(drag), Some(at)) = (duel.tray_drag, cursor) {
        let band = crate::hud::band_of(&windows, ui.as_deref());
        let delta = at - drag.last;
        if delta != Vec2::ZERO {
            let place = settings
                .zone_browser
                .map_or_else(|| Placement::centred(band), |p| p.fit(band));
            let moved = match drag.kind {
                TrayDragKind::Move => place.moved_by((delta.x, delta.y), band),
                TrayDragKind::Resize => place.resized_by((delta.x, delta.y), band),
            };
            settings.zone_browser = Some(moved);
            write_placement(&mut panels, moved);
        }
        duel.tray_drag = Some(TrayDrag { last: at, ..drag });
    }

    // The corner used to maximise as well, on a press and release that
    // travelled less than a `TAP_SLOP` of 4 px between them. It was found
    // this way rather than through a `Pointer<Click>` because a resize *ends*
    // over the corner — the corner travels under the hand — so every drag
    // would have fired one; and it existed at all because the corner drew a
    // ⤢, which is an argument from a mark rather than from a control. The
    // mark and the gesture both moved into the head on 19.09.2026
    // (`hud::TrayMaximise`), where a click is a click and nothing has to be
    // inferred from how far a hand wandered. The corner resizes.
    if let Some(drag) = ended {
        settings.save();
        // A *resize* ends with the sheet a different size than the one it was
        // built at, so the sheet is rebuilt once — the grid's tiles are
        // packed from the width and a stretched sheet keeps the packing it
        // had. A move is not asked for: nothing about the sheet's contents
        // depends on where in the band it stands.
        if drag.kind == TrayDragKind::Resize {
            revision.relayout();
        }
    }
}

/// Flies the sheet between two rectangles, and stops.
///
/// The counterpart of `hud::reveal_tray`: that one carries the sheet in and
/// out of the tray, this one carries it between two sizes. They are two
/// systems because they move two different things — a `UiTransform` there, a
/// `Node` here — and [`TrayGlide`]'s own docs carry why this one cannot be a
/// transform.
///
/// It is in `input` rather than in `hud` for one reason and it is a good one:
/// [`write_placement`] is here, because a drag is the other thing that moves
/// the sheet without going through the revision, and two writers of one
/// `Node` in two modules is how they come to disagree about which of them
/// owns it. A drag and a glide never run together — `tray_drag` takes the
/// press, and the maximise button is excluded from it.
///
/// `reduce_motion` is honoured the way everything else here honours it: the
/// movement still happens, it is simply already over on the frame it started.
pub fn glide_the_sheet(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut glide: ResMut<TrayGlide>,
    mut revision: ResMut<crate::hud::TrayRevision>,
    mut panels: Query<&mut Node, With<crate::hud::TrayPanel>>,
) {
    /// How long the sheet takes to change size.
    ///
    /// The ability sheet's own span (`hud::motion::ZOOM_IN`), because this is
    /// the same claim about the same interface: §7 measures everything
    /// against "160 ms ease-out-back", and a panel resizing is no more
    /// important than a panel arriving.
    const GLIDE: f32 = 0.16;

    let Some(flight) = glide.0.as_mut() else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    flight.t = if still {
        1.0
    } else {
        (flight.t + time.delta_secs() / GLIDE).min(1.0)
    };
    // Ease-out, with no overshoot at all. A rectangle that overshot would put
    // an edge of the sheet outside the band for two frames — `lerp` does not
    // clamp, on purpose — and the one place this movement ends is exactly the
    // rectangle the store already holds.
    let eased = 1.0 - (1.0 - flight.t).powi(3);
    let at = flight.from.lerp(flight.to, eased);
    write_placement(&mut panels, at);
    if flight.t >= 1.0 {
        glide.0 = None;
        // And one rebuild, now that the sheet has stopped. The head's
        // maximise button is drawn from `is_maximised` and the grid's tiles
        // are packed from the sheet's width, and both were decided the last
        // time `sync_tray` ran — which was before any of this moved. Without
        // it a maximised sheet still offers to maximise.
        revision.relayout();
    }
}

/// Puts a placement on whatever sheet is currently drawn.
///
/// The renderer would get there on its own at the next rebuild — the sheet is
/// built from `settings.zone_browser` — but only at the next rebuild, and a
/// drag deliberately does not cause one. See [`tray_drag`]'s own docs for why
/// the geometry is kept out of the revision.
fn write_placement(panels: &mut Query<&mut Node, With<crate::hud::TrayPanel>>, place: Placement) {
    for mut node in panels {
        node.left = px(place.left);
        node.top = px(place.top);
        node.width = px(place.width);
        node.height = px(place.height);
    }
}

/// Drags on the preview's resize handle, and the resize shortcut
/// (Command/Alt + Shift + Up/Down). The size is persisted.
#[allow(clippy::too_many_arguments)] // events + queries + state, all needed
pub fn preview_resize(
    keys: Res<ButtonInput<KeyCode>>,
    mut downs: MessageReader<Pointer<Press>>,
    mut ups: MessageReader<Pointer<Release>>,
    resize: Query<&PreviewResize>,
    parents: Query<&ChildOf>,
    mut motions: MessageReader<MouseMotion>,
    mut duel: ResMut<Duel>,
    mut settings: ResMut<ClientSettings>,
) {
    for down in downs.read() {
        if find_in_lineage(down.entity, &resize, &parents).is_some() {
            duel.resize_drag = true;
        }
    }
    let mut ended = false;
    for _up in ups.read() {
        ended |= duel.resize_drag;
        duel.resize_drag = false;
    }
    if duel.resize_drag {
        let dx: f32 = motions.read().map(|m| m.delta.x).sum();
        if dx != 0.0 {
            settings.preview_scale = (settings.preview_scale + dx * 0.004).clamp(0.5, 1.75);
        }
    } else {
        motions.clear();
    }
    if ended {
        settings.save();
    }

    // Command/Alt + Shift + Up/Down resizes too.
    let meta = keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight)
        || keys.pressed(KeyCode::AltLeft)
        || keys.pressed(KeyCode::AltRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if meta && shift {
        if keys.just_pressed(KeyCode::ArrowUp) {
            settings.preview_scale = (settings.preview_scale + 0.05).clamp(0.5, 1.75);
            settings.save();
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            settings.preview_scale = (settings.preview_scale - 0.05).clamp(0.5, 1.75);
            settings.save();
        }
    }
}
