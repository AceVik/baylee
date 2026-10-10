//! Stack arrival and promotion motion.

#[allow(clippy::wildcard_imports)] // the stack panel shares one drawing vocabulary
use super::*;

/// Reconciles what is being tracked against what is on screen.
///
/// Three rules, and each is a different answer to "this key was not here last
/// frame".
///
/// A row that steps **down** is not an arrival. When a spell lands on a stack
/// that already had one, yesterday's top is drawn queued from this frame on —
/// a different key for the same object, which would otherwise be seeded at
/// nothing and fade in beside the newcomer, so two rows would announce
/// themselves and only one of them would be new. Carrying the progress across
/// the key change leaves it standing where it was, and it is the *current*
/// progress and not `1.0` because the house AI answers within a frame or two
/// of priority: the common case is a spell demoted while it is still
/// arriving, and seeding that at rest would snap it to full.
///
/// A row that steps **up** is a promotion — the object above it resolved —
/// and that one is *not* carried across, because it is the movement the
/// player is meant to see. It is marked instead, and moves the other way; see
/// [`PROMOTE_LIFT`]. The marking has to happen here, before the retain below
/// drops the queued key the question is asked about.
///
/// Anything else is new and starts at nothing.
pub(in crate::hud) fn track(motion: &mut StackMotion, live: &[StackKey]) {
    for &key in live {
        let StackKey::Entry(id, false) = key else {
            continue;
        };
        let known = motion.rows.iter().any(|rise| rise.key == key);
        let stepped_down = motion
            .rows
            .iter()
            .find(|rise| rise.key == StackKey::Entry(id, true))
            .map(|rise| rise.at);
        if !known && let Some(progress) = stepped_down {
            motion.rows.push(Rise {
                key,
                at: progress,
                promoted: false,
                cooled: 1.0,
            });
        }
    }

    let promoted: Vec<StackKey> = live
        .iter()
        .copied()
        .filter(|key| {
            let StackKey::Entry(id, true) = *key else {
                return false;
            };
            !motion.rows.iter().any(|rise| rise.key == *key)
                && motion
                    .rows
                    .iter()
                    .any(|rise| rise.key == StackKey::Entry(id, false))
        })
        .collect();

    motion.rows.retain(|rise| live.contains(&rise.key));
    for &key in live {
        if !motion.rows.iter().any(|rise| rise.key == key) {
            let promoted = promoted.contains(&key);
            motion.rows.push(Rise {
                key,
                at: 0.0,
                promoted,
                // Only a promotion has a rail to cool. Seeded at nothing for
                // everything else, the slower ramp would keep `moving` true
                // for a third of a second after the row had settled, and
                // every node in the panel would go on being written to with
                // nothing left to say.
                cooled: if promoted { 0.0 } else { 1.0 },
            });
        }
    }
}

/// Eases every arriving row towards its resting state.
///
/// Runs after [`sync_overlay`] and deliberately: a row spawned this frame is
/// spawned at *rest*, and without the ordering it would be drawn once at full
/// strength before this system ever saw it — a one-frame flash at the top of
/// the panel, which is the one place in the interface a player is watching.
///
/// [`sync_overlay`]: super::sync_overlay
pub fn ease_the_stack_in(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut motion: ResMut<StackMotion>,
    mut fades: Query<(
        &Arriving,
        Option<&mut TextColor>,
        Option<&mut BackgroundColor>,
    )>,
    mut rows: Query<(&Arriving, &ArrivingRow, &mut UiTransform, &mut BorderColor)>,
) {
    // Which rows are on screen. Anything else has resolved, or the panel has
    // gone with the last object on it, and its progress goes with it — an id
    // kept after its row left would greet the *next* spell of that name with
    // no animation at all.
    //
    // `fades` is the wider query — a row carries an [`Arriving`] too — so it
    // is the one that sees every key.
    let mut live: Vec<StackKey> = Vec::new();
    for (arriving, ..) in &fades {
        if !live.contains(&arriving.key) {
            live.push(arriving.key);
        }
    }
    track(&mut motion, &live);

    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let ease = |rate: f32| {
        if still {
            1.0
        } else {
            1.0 - (-rate * time.delta_secs()).exp()
        }
    };
    let step = ease(ARRIVE_RATE);
    let settle = ease(SETTLE_RATE);
    let mut moving = false;
    for rise in &mut motion.rows {
        if rise.at >= 1.0 && rise.cooled >= 1.0 {
            continue;
        }
        rise.at = (rise.at + (1.0 - rise.at) * step).min(1.0);
        if rise.at > 0.999 {
            rise.at = 1.0;
        }
        rise.cooled = (rise.cooled + (1.0 - rise.cooled) * settle).min(1.0);
        if rise.cooled > 0.999 {
            rise.cooled = 1.0;
        }
        moving = true;
    }
    // Every node is spawned at rest, so a panel that has finished arriving
    // needs nothing written to it at all — including on the frame it is
    // rebuilt under the pointer.
    if !moving {
        return;
    }

    for (arriving, ink, fill) in &mut fades {
        let alpha = arriving.alpha(motion.progress(arriving.key));
        match arriving.paints {
            Paints::Ink => {
                if let Some(mut ink) = ink {
                    ink.0 = ink.0.with_alpha(alpha);
                }
            }
            Paints::Fill | Paints::Veil => {
                if let Some(mut fill) = fill {
                    fill.0 = fill.0.with_alpha(alpha);
                }
            }
        }
    }
    for (arriving, row, mut transform, mut border) in &mut rows {
        let rise = motion.rise(arriving.key);
        let progress = rise.map_or(1.0, |rise| rise.at);
        // Which way the row came, which is the whole of the difference
        // between a spell being cast and a spell resolving. An arrival drops
        // in from above (negative y is up); a promotion comes up from the
        // slot below and grows, because that is what happened to it.
        let (lift, from) = if rise.is_some_and(|rise| rise.promoted) {
            (PROMOTE_LIFT, PROMOTE_SCALE)
        } else {
            (-row.lift, row.from)
        };
        transform.translation.y = px(lift * (1.0 - progress));
        transform.scale = Vec2::splat(from + (1.0 - from) * progress);
        // The rail lands bright and cools into its own colour, on the slower
        // ramp, so the light is still settling after the row has stopped —
        // an accent mark appearing one slot down from where it was is the
        // resolution, told without a ghost of the object that left.
        let rail = match rise {
            Some(rise) if rise.promoted => palette::INK.mix(&row.rail, rise.cooled),
            _ => row.rail,
        };
        *border = BorderColor::all(rail.with_alpha(row.rail.alpha() * progress));
    }
}
