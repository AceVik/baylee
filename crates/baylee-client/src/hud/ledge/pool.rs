//! What this seat still owes: the strip on the shelf's right end, standing
//! only while a payment window of its own is open (CR 605.3a).
//!
//! It was the mana pool. On 08.10.2026 the owner moved every seat's floating
//! mana onto that seat's plate on the table — *"Move the Manavorrat display
//! into the player details plate … remove the old separate mana pool
//! widget"* — so what floats is read where the seat is (`seatbar::attached`,
//! the plate's third line), for every seat alike. What the strip kept is the
//! one thing the plate cannot say reliably enough: the **remainder** of a
//! cost being paid (`manaplan::remainder`), which a player reads while
//! tapping lands and which must not be under the hand zone or behind a
//! visit. So the strip stands while the seat owes and folds away when the
//! window closes, growing out of the shelf as the drawer does.
//!
//! It hangs off the shelf from [`super::strip_node`] at the right end (#264),
//! retained, spawned once beside the shelf and filled by [`sync_pool`]
//! against a [`PoolRevision`] of its own: the shelf is rebuilt on every
//! sentence and the strip must not be.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// The retained right strip.
#[derive(Component)]
pub struct PoolStrip;

/// What is still owed: the word and the pips, spawned and thrown away as one.
#[derive(Component)]
pub struct PoolOwed;

/// Where the **strip** is in its own arrival or departure.
///
/// A component on the strip and not a resource, which is the shape
/// [`super::drawer::DrawerZoom`] already has: a resource is a third thing
/// every test harness that runs this system has to be told about, and a
/// missing one is a runtime panic that `cargo check` and clippy are both
/// green over.
///
/// It is spawned **shut** — `t` at the end of a close — because the strip is
/// spawned hidden and a fresh `Default` would read as the first frame of an
/// arrival that nobody asked for.
#[derive(Component)]
pub struct StripZoom {
    /// Progress through the current movement, 0 to 1.
    t: f32,
    /// Whether the movement is the fold.
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

/// What the strip is showing.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct PoolRevision {
    lang: Option<Lang>,
    /// What the seat still owes in its open payment window, and `None` the
    /// rest of the time — which is nearly always.
    owed: Option<Owing>,
}

/// What is owed, as drawn: the revision holds what is on the screen, so a
/// pool that pays a pip rebuilds it even when the cost did not change.
#[derive(Clone, PartialEq, Debug)]
enum Owing {
    /// A fixed cost, less what the pool already pays
    /// ([`baylee_client_core::manaplan::remainder`]); empty once it is paid.
    Fixed(Vec<baylee_core::mana::ManaSymbol>),
    /// A number the player chooses (CR 605.3a's optional payments): the word
    /// alone, there being no cost to subtract from.
    AnyAmount,
}

impl Owing {
    /// What `view`'s own seat still owes, if it is the seat being asked.
    ///
    /// Only its own window: `owed` names what the *awaited* seat owes, and an
    /// opponent's ward tax is not owed from this seat's pool.
    fn of(view: &baylee_view::PlayerView) -> Option<Self> {
        if view.awaiting != Some(view.seat) {
            return None;
        }
        Some(match view.owed? {
            baylee_core::mana::ManaPayment::Fixed(cost) => {
                let pool = view
                    .seat(baylee_client_core::decision::resource_player(view))
                    .map(|s| s.mana_pool)
                    .unwrap_or_default();
                Self::Fixed(baylee_client_core::manaplan::remainder(&cost, &pool))
            }
            baylee_core::mana::ManaPayment::AnyAmount { .. } => Self::AnyAmount,
        })
    }
}

/// Spawns the strip, once, beside the shelf: hidden, shut, at the tray's
/// rung, because a payment is read while a maximised zone dialog is open.
pub(in crate::hud) fn spawn_pool_strip(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            PoolStrip,
            StripZoom::default(),
            Node {
                column_gap: px(POOL_ENTRY_GAP),
                ..strip_node(StripSide::Right)
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            ZIndex(Z_TRAY),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .id()
}

/// Writes what is owed, and opens or folds the strip with the window.
pub fn sync_pool(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    mut revision: ResMut<PoolRevision>,
    mut strip: Query<(Entity, &mut StripZoom), With<PoolStrip>>,
) {
    let Ok((strip, mut fold)) = strip.single_mut() else {
        return;
    };
    let lang = Lang::of(&settings.lang);
    let next = PoolRevision {
        lang: Some(lang),
        owed: duel.view.as_ref().and_then(Owing::of),
    };
    if *revision == next {
        return;
    }
    let empty = next.owed.is_none();
    if empty != fold.closing {
        fold.closing = empty;
        fold.t = 0.0;
    }
    // A strip folding away keeps what it said until it is gone.
    if let Some(owing) = &next.owed {
        let group = owed_group(&mut commands, &fonts, lang, owing);
        commands.entity(strip).despawn_children().add_child(group);
    }
    *revision = next;
}

/// Opens and folds the strip: the sheet's own pop on the way in, a shrink on
/// the way out, from its bottom-right corner, and hidden at the end of a
/// fold. Under `reduce_motion` both are at their end at once.
pub fn grow_the_pool(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    mut strips: Query<(&mut StripZoom, &mut UiTransform, &mut Visibility), With<PoolStrip>>,
) {
    let still = prefs.all().reduce_motion;
    for (mut fold, mut transform, mut seen) in &mut strips {
        // At rest, either way: a `Mut` writes on every deref, and a strip
        // standing there would mark itself changed on every frame.
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
        transform.translation = motion::from_bottom_right(scale);
    }
}

/// The word `Owed` and what is still owed, as pips.
///
/// Drawn by the same [`crate::manaui::spawn_pip`] the deck builder draws a
/// printed cost with, at the pool's own pip size, because "owe {2}{G}" and
/// "have {G}" standing in two different registers would be two things a
/// player has to convert between. And it is the **remainder**, not the cost
/// (TODO client item 8): `{2}{G}` with a Forest tapped by hand says `{2}`,
/// so there is nothing left to subtract at all. Paid in full it says `{0}`
/// until the pass settles the window, rather than a word with nothing after
/// it.
///
/// The word is [`palette::LEDGE_SOFT`] like the row's own label and for its
/// measured reason — 5.45 : 1 on the strip's `DIALOG_LIT`, over the 4.5 prose
/// is held to. The pips carry their own colours and are not dimmed: an owed
/// cost is not a disabled thing, it is the question being asked.
fn owed_group(commands: &mut Commands, fonts: &UiFonts, lang: Lang, owing: &Owing) -> Entity {
    let group = commands
        .spawn((
            PoolOwed,
            Node {
                column_gap: px(POOL_ENTRY_GAP),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let word = commands
        .spawn((
            Text::new(
                match owing {
                    Owing::Fixed(_) => Phrase::Owed,
                    Owing::AnyAmount => Phrase::OptionalPayment,
                }
                .text(lang)
                .to_string(),
            ),
            tf(fonts, POOL_LABEL_PT),
            TextColor(palette::LEDGE_SOFT),
            Node {
                margin: UiRect::right(px(POOL_LABEL_GAP - POOL_ENTRY_GAP)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut kids = vec![word];
    if let Owing::Fixed(left) = owing {
        let paid = [baylee_core::mana::ManaSymbol::Generic(0)];
        let left = if left.is_empty() { &paid[..] } else { left };
        kids.extend(left.iter().map(|symbol| {
            crate::manaui::spawn_pip(
                commands,
                fonts,
                baylee_client_core::manapip::pip(*symbol),
                POOL_PIP,
            )
        }));
    }
    commands.entity(group).replace_children(&kids);
    group
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::mana::{ManaCost, ManaSymbol};
    use bevy::ecs::system::RunSystemOnce as _;

    /// The strip and the one system that fills it, over `duel`.
    fn strip_over(duel: Duel) -> App {
        let mut app = App::new();
        app.insert_resource(duel)
            .insert_resource(UiFonts::default())
            .init_resource::<crate::settings::ClientSettings>()
            .init_resource::<PoolRevision>()
            .add_systems(Update, sync_pool);
        app.world_mut()
            .run_system_once(|mut commands: Commands| {
                spawn_pool_strip(&mut commands);
            })
            .expect("the strip is spawned");
        app.update();
        app
    }

    /// The glyphs the owed half of the row draws, in order, one string per
    /// pip; `None` when there is no owed half at all.
    fn owed_glyphs(app: &mut App) -> Option<Vec<String>> {
        let world = app.world_mut();
        let group = world
            .query_filtered::<Entity, With<PoolOwed>>()
            .iter(world)
            .next()?;
        let kids: Vec<Entity> = world
            .entity(group)
            .get::<Children>()
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        // The first child is the word; each one after is a pip's disc, whose
        // own text (or its two halves' texts) is the glyph.
        let glyphs = kids
            .into_iter()
            .skip(1)
            .map(|disc| {
                let mut stack = vec![disc];
                let mut glyph = String::new();
                while let Some(e) = stack.pop() {
                    if let Some(text) = world.entity(e).get::<Text>() {
                        glyph.push_str(&text.0);
                    }
                    if let Some(children) = world.entity(e).get::<Children>() {
                        stack.extend(children.iter());
                    }
                }
                glyph
            })
            .collect();
        Some(glyphs)
    }

    /// The glyph a one-colour or generic `symbol` is drawn with.
    fn glyph(symbol: ManaSymbol) -> String {
        match baylee_client_core::manapip::pip(symbol) {
            baylee_client_core::manapip::Pip::Solid { glyph, .. } => glyph.to_string(),
            other => {
                panic!("{other:?} is not one this test reads")
            }
        }
    }

    fn with_floating_green(mut duel: Duel, green: u32) -> Duel {
        let view = duel.view.as_mut().expect("a view");
        let seat = view.seat;
        view.seats
            .iter_mut()
            .find(|s| s.player == seat)
            .expect("this seat")
            .mana_pool
            .green = green;
        duel
    }

    /// TODO client item 8: the row says what is **still** owed. `{2}{G}`
    /// owed with a Forest's green floating says `{2}`, paid in full it says
    /// `{0}`, and with nothing floating it is the cost.
    #[test]
    fn the_strip_says_what_is_still_owed_not_the_cost() {
        let duel = || {
            let owed = ManaCost::try_parse("{2}{G}").expect("a cost");
            crate::owed_tests::seat_with_two_forests(Some(owed))
        };

        let mut app = strip_over(duel());
        assert_eq!(
            owed_glyphs(&mut app),
            Some(vec![
                glyph(ManaSymbol::Generic(2)),
                glyph(ManaSymbol::Green)
            ]),
            "nothing floats: the whole cost"
        );

        let mut app = strip_over(with_floating_green(duel(), 1));
        assert_eq!(
            owed_glyphs(&mut app),
            Some(vec![glyph(ManaSymbol::Generic(2))]),
            "the floating green paid the green"
        );

        // And it follows the pool on the same strip: a second view.
        *app.world_mut().resource_mut::<Duel>() = with_floating_green(duel(), 3);
        app.update();
        assert_eq!(
            owed_glyphs(&mut app),
            Some(vec![glyph(ManaSymbol::Generic(0))]),
            "paid in full, until the pass settles it"
        );
    }

    /// The owed half is this seat's own window only: a seat watching another
    /// pay is owed nothing from its own pool.
    #[test]
    fn a_watching_seat_is_owed_nothing() {
        let owed = ManaCost::try_parse("{1}").expect("a cost");
        let mut duel = crate::owed_tests::seat_with_two_forests(Some(owed));
        let view = duel.view.as_mut().expect("a view");
        let other = view
            .seats
            .iter()
            .map(|s| s.player)
            .find(|p| *p != view.seat);
        assert!(other.is_some(), "another seat to be asked");
        view.awaiting = other;
        let mut app = strip_over(duel);
        assert_eq!(owed_glyphs(&mut app), None);
    }
}
