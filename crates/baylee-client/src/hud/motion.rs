//! The one curve everything in the HUD that *arrives* is drawn on.
//!
//! §7 of the design measures the whole interface against the ability sheet —
//! "160 ms ease-out-back with 3% overshoot" — and then says that everything
//! on the shelf is *quieter* than that. The point of a file rather than a
//! second copy of five constants is that the two claims stay one claim: the
//! sheet, the drawer and a mana pip arriving all read the numbers from here,
//! so a curve that is retuned is retuned everywhere it was ever compared to.
//!
//! What is **not** here is the state. A thing that opens needs a `t` and a
//! direction, and each of them keeps its own — the sheet's [`SheetZoom`], the
//! drawer's [`DrawerZoom`] — because they are despawned by different systems
//! at different times and a shared component would have to be told which.
//!
//! [`SheetZoom`]: super::sheet::SheetZoom
//! [`DrawerZoom`]: super::ledge::drawer::DrawerZoom

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// How long a thing takes to open, in seconds.
///
/// Short enough to be over before a player has finished looking down at it —
/// the sheet answers a click, and an answer that takes a quarter of a second
/// to arrive is a delay rather than a movement.
pub(super) const ZOOM_IN: f32 = 0.16;

/// And to close.
///
/// Shorter still, because the two are not the same event: opening is an
/// answer arriving and closing is it being dismissed, and a dismissal that
/// took as long as the answer reads as reluctance.
pub(super) const ZOOM_OUT: f32 = 0.10;

/// The size a thing grows from, and shrinks back to.
///
/// Not zero. A sheet that grows out of nothing is a puff of smoke; this is a
/// page being laid down — it was always this size, and the movement is the
/// last eighth of it arriving.
pub(super) const ZOOM_FROM: f32 = 0.88;

/// The overshoot's shape, as the `c₁` of the usual ease-out-back.
///
/// The curve runs 0 → 1 over the range [`ZOOM_FROM`]..1 and its peak is
/// `4c³ / 27(c+1)²` of that range past the end. **Three** is the value where
/// that closed form collapses to exactly a quarter — `4·27 / 27·16` — so the
/// overshoot is a clean `(1 − ZOOM_FROM)/4`, which is 3% of full size.
///
/// The textbook `1.70158` is for a curve whose range *is* the whole size; at
/// this range it would overshoot by a third of a per cent and there would be
/// no snap at all.
pub(super) const ZOOM_BACK: f32 = 3.0;

/// The share of an opening that a thing attached to the opening waits out.
///
/// The sheet's nub is attached to something — the sheet's own edge — and a
/// sheet at 90% has that edge 5% of its height away from where the nub is
/// drawn. Rather than animate the gap away, the tail arrives once the paper
/// is nearly full size, which also reads right: the sheet opens, and *then*
/// it points.
pub(super) const ZOOM_TAIL: f32 = 0.55;

/// The opening's curve: ease-out-back, running 0 → 1 with an overshoot.
///
/// `1 + (c+1)u³ + cu²` with `u = t − 1`, whose peak is `4c³/27(c+1)²` past
/// the end — see [`ZOOM_BACK`] for why that number and not the usual one.
/// [`opening`] maps the whole curve onto [`ZOOM_FROM`]..1, so the overshoot
/// is that share of the *range* and not of the size.
pub(super) fn pop(t: f32) -> f32 {
    let u = t - 1.0;
    1.0 + (ZOOM_BACK + 1.0) * u * u * u + ZOOM_BACK * u * u
}

/// The scale a thing that is opening is drawn at.
pub(super) fn opening(t: f32) -> f32 {
    ZOOM_FROM + (1.0 - ZOOM_FROM) * pop(t)
}

/// The scale a thing that is closing is drawn at.
///
/// Accelerating away, and deliberately not [`pop`] run backwards: the
/// opening's overshoot would read as a bounce on the way out, which is a
/// movement asking to be watched by something that is leaving.
pub(super) fn shutting(t: f32) -> f32 {
    1.0 - (1.0 - ZOOM_FROM) * t * t
}

/// How far into its own ramp a thing attached to an opening is.
///
/// Zero until [`ZOOM_TAIL`] of the opening is done, then 0 → 1 over what is
/// left of it.
pub(super) fn tail(t: f32) -> f32 {
    ((t - ZOOM_TAIL) / (1.0 - ZOOM_TAIL)).clamp(0.0, 1.0)
}

/// Advances one `t` by a frame, or puts it at the end for a player who has
/// turned motion off.
///
/// The same answer `table::glide` and `ShownRig` both give: `reduce_motion`
/// does not mean "no movement happened", it means the movement is already
/// over on the frame it started.
pub(super) fn step(t: f32, span: f32, delta: f32, still: bool) -> f32 {
    if still {
        1.0
    } else {
        (t + delta / span).min(1.0)
    }
}

/// The translation that keeps a scaled node's **bottom** edge where it was.
///
/// A [`UiTransform`] scales about the node's centre, so at scale `s` the
/// bottom edge has risen by `(1 − s)·h/2` and the node has to be pushed back
/// down by the same amount. It is written as a **percentage** on purpose:
/// `Val::Percent` in a `UiTransform`'s translation resolves against the
/// node's own computed size (`Val2`'s own field docs say so, and
/// `compute_affine` is handed `layout_size` as the base), so the number is
/// exact without anyone reading a `ComputedNode` — and therefore without the
/// logical-versus-physical question that reading one would raise.
///
/// This is what separates a drawer from a sheet. A sheet is laid down and may
/// grow from its middle; a drawer is a lip of the shelf and grows out of it.
pub(super) fn from_bottom(scale: f32) -> Val2 {
    Val2::new(Val::ZERO, Val::Percent(50.0 * (1.0 - scale)))
}

/// The same, for a node pinned at the window's **right** margin rather than
/// centred on the shelf: the game menu, and the mana pool since #264.
///
/// [`from_bottom`] lets a node shrink toward its own horizontal middle, which
/// is right for the drawer: it is centred, so its middle is where it came
/// from. A strip is fixed at one margin, and a node that scales about its
/// centre while one edge is pinned by the layout appears to *slide* inward as
/// it grows — a second movement, in a direction nothing is going. Pinning the
/// bottom-right corner is the same arithmetic on the other axis: the corner
/// that stays still while it grows is the one it hangs off.
///
/// Its left-hand twin went when the pool moved to this side and nothing at
/// the left margin grew any more. A node that does again takes this with
/// the horizontal sign flipped, written out as a function of its own rather
/// than as a signed parameter here: a sign is a thing a caller gets
/// backwards, a name is not.
pub(super) fn from_bottom_right(scale: f32) -> Val2 {
    Val2::new(
        Val::Percent(50.0 * (1.0 - scale)),
        Val::Percent(50.0 * (1.0 - scale)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The curve's ends are the ends, and its overshoot is the documented one.
    ///
    /// [`ZOOM_BACK`]'s whole doc comment is an arithmetic claim — that `3.0`
    /// is the `c` where `4c³/27(c+1)²` collapses to exactly a quarter — and a
    /// claim like that is either measured here or is a comment nobody can
    /// check. The counter-test is the textbook constant: at this range
    /// `1.70158` overshoots by less than a percent of the range, which is the
    /// reason the number was changed in the first place.
    #[test]
    fn the_opening_overshoots_by_a_quarter_of_what_it_travels() {
        assert!((pop(0.0) - 0.0).abs() < 1e-5, "it starts where it starts");
        assert!((pop(1.0) - 1.0).abs() < 1e-5, "and ends where it ends");

        let peak = (0..=1000u16)
            .map(|i| pop(f32::from(i) / 1000.0))
            .fold(f32::MIN, f32::max);
        assert!(
            (peak - 1.25).abs() < 0.005,
            "the range's quarter is the overshoot: {peak:.4}"
        );

        // And what it would have been with the constant meant for a curve
        // that travels the whole size.
        let textbook = |t: f32| {
            let u = t - 1.0;
            1.0 + (1.70158 + 1.0) * u * u * u + 1.70158 * u * u
        };
        let flat = (0..=1000u16)
            .map(|i| textbook(f32::from(i) / 1000.0))
            .fold(f32::MIN, f32::max);
        assert!(
            flat < 1.11,
            "this test's premise is that the textbook constant has no snap at \
             this range, and it reached {flat:.4}"
        );
    }

    /// Scaled from the bottom, the bottom edge does not move.
    ///
    /// The arithmetic a comment cannot hold: a `UiTransform` maps a local
    /// point `y` to `s·y + t`, and the node's own bottom edge is at `+h/2` in
    /// a box centred on the origin. Read back as the percentage it is written
    /// as, because that is the form the layout resolves.
    #[test]
    fn a_drawer_grows_out_of_the_shelf_and_not_out_of_itself() {
        let height = 40.0;
        for scale in [ZOOM_FROM, 0.94, 1.0, 1.03] {
            let Val::Percent(pct) = from_bottom(scale).y else {
                panic!("the bottom anchor has to be a percentage of the node");
            };
            let shift = pct / 100.0 * height;
            let bottom = scale.mul_add(height / 2.0, shift);
            assert!(
                (bottom - height / 2.0).abs() < 1e-4,
                "at {scale} the bottom edge moved to {bottom} from {}",
                height / 2.0
            );
        }
        // The counter-test: without the shift it moves, which is what a sheet
        // is allowed to do and a drawer is not.
        let adrift = ZOOM_FROM * height / 2.0;
        assert!(
            (adrift - height / 2.0).abs() > 2.0,
            "this test's premise is that a node scaled about its middle \
             leaves the shelf, and it moved {:.2} px",
            height / 2.0 - adrift
        );
    }

    /// A player who has turned motion off is at the end on the first frame,
    /// and everyone else is not.
    #[test]
    fn reduce_motion_is_the_end_of_the_movement_and_not_its_absence() {
        assert!((step(0.0, ZOOM_IN, 0.0, true) - 1.0).abs() < f32::EPSILON);
        let one = step(0.0, ZOOM_IN, 1.0 / 60.0, false);
        assert!(one > 0.0 && one < 1.0, "a frame is a frame: {one}");
        assert!(
            (step(0.99, ZOOM_OUT, 1.0, false) - 1.0).abs() < f32::EPSILON,
            "and a long frame does not run past the end"
        );
    }
}
