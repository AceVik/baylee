//! The screen that is shown while the client is waiting for something.
//!
//! Waiting is not rare here and some of it is long: signing in, fetching a
//! pool of fourteen hundred cards, and — the one that actually needs saying —
//! sitting down at a table, where the gateway orders an engine and the seat's
//! socket may wait up to thirty seconds for it to attach. A screen that shows
//! nothing during that is a screen that looks broken, and a player who
//! believes it is broken presses the button again.
//!
//! So the veil says three things: that the client is alive (the ground moves),
//! what it is waiting for (a line of text the caller supplies), and that time
//! is passing (the dots). It is one resource wide — [`Loading`] — because the
//! lobby, the deck builder and the duel all wait for different things and none
//! of them should have to own a screen to say so.
//!
//! **A wait and a veil are not the same thing.** Asking for one is free and
//! happens on the frame a request goes out; drawing one takes the screen away
//! from the player, so it waits [`GRACE`] first. Offline every lobby request
//! is answered in this process and its reply is read on the next frame, so a
//! click on *play offline*, on *edit*, on anything at all raised the veil for
//! one or two frames — thirty milliseconds of "One moment" over a screen that
//! had already finished. Nothing can be read in that time, so what it says is
//! not "wait": it is a flash, and a flash on every click is the whole of the
//! offline lobby's flicker.

use bevy::prelude::*;
use bevy::ui::UiTransform;

use crate::ambience;
use crate::hud::{UiFonts, palette, tf};
use crate::vista::{self, Framed, Vista, VistaMaterial};

/// The colour the veil dims what is behind it to.
///
/// Not opaque: the screen underneath stays faintly visible, so a player can
/// see they have not been thrown back to the beginning.
const VEIL: Color = Color::srgba(0.02, 0.03, 0.04, 0.86);

/// How fast the dots pulse, in cycles per second.
const PULSE_RATE: f32 = 0.9;

/// How long something has to have been waited for before the veil is raised.
///
/// A quarter of a second sits under the delay a click is felt at and far under
/// every wait the veil was written for — a gateway answering, a pool of
/// fourteen hundred cards, an engine being ordered — so the only waits it
/// takes off the screen are the ones that were over before they could be read.
///
/// There is deliberately no matching *minimum* time to leave it up. That would
/// keep a veil on screen after the thing it names has finished, and the rule
/// the line of text is already chosen by — a bar that is lying is worse than a
/// bar that is absent — says the same about a veil.
const GRACE: f32 = 0.25;

/// What the client is waiting for, if anything.
///
/// Set it to show the veil, clear it to take the veil away. Deliberately one
/// line of text and not a percentage: almost nothing the client waits for can
/// honestly report progress, and a bar that is lying is worse than a bar that
/// is absent.
///
/// [`Loading::what`] is what is being *waited for*; the veil is what is
/// *drawn*, and it follows only once the wait has lasted [`GRACE`]. A caller
/// says what it is doing and never has to guess how long it will take.
#[derive(Resource, Default)]
pub struct Loading {
    what: Option<String>,
}

impl Loading {
    /// Shows the veil, saying what is being waited for.
    pub fn show(&mut self, what: impl Into<String>) {
        let what = what.into();
        if self.what.as_deref() != Some(what.as_str()) {
            self.what = Some(what);
        }
    }

    /// Takes the veil away.
    pub fn clear(&mut self) {
        self.what = None;
    }

    /// What is being waited for, if anything.
    #[must_use]
    pub fn what(&self) -> Option<&str> {
        self.what.as_deref()
    }
}

/// The veil's root, so it can be found and despawned.
#[derive(Component)]
struct Veil;

/// One of the dots, and where in the cycle it sits.
#[derive(Component)]
struct Pulse {
    /// Offset into the cycle, so the three do not beat as one.
    phase: f32,
}

/// Shows the veil while something is being waited for.
pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        ambience::install(app);
        crate::vista::install(app);
        app.init_resource::<Loading>()
            .add_systems(Update, (raise, pulse));
    }
}

/// Adds [`LoadingPlugin`] unless another plugin already did.
pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<LoadingPlugin>() {
        app.add_plugins(LoadingPlugin);
    }
}

/// Spawns the veil once [`Loading`] has been set for [`GRACE`], and drops it
/// when it is cleared.
///
/// Rebuilt only when what is drawn changes — which is at most twice per wait,
/// so the retained tree costs nothing and the dots below animate on entities
/// that survive the whole wait.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
fn raise(
    mut commands: Commands,
    time: Res<Time>,
    loading: Res<Loading>,
    fonts: Option<Res<UiFonts>>,
    veil: Query<Entity, With<Veil>>,
    scene: Option<ResMut<Assets<VistaMaterial>>>,
    mut shown: Local<Option<String>>,
    mut asked: Local<f32>,
) {
    // The grace is counted on *whether* something is being waited for and not
    // on what it is called, so a wait that changes its words while it runs —
    // "One moment", then "Taking your seat" — swaps the line rather than
    // starting the clock again and taking the veil back down.
    if loading.what.is_some() {
        *asked += time.delta_secs();
    } else {
        *asked = 0.0;
    }
    // Compared against what is actually on screen rather than trusting
    // `Res::is_changed`: a caller that sets the same wait from a system
    // running every frame touches the resource every frame, and rebuilding
    // the veil that often would reset the dots to the start of their cycle —
    // three dots that never move, which is precisely the impression the veil
    // exists to avoid.
    let want = (*asked >= GRACE).then(|| loading.what.clone()).flatten();
    if *shown == want && want.is_some() != veil.is_empty() {
        return;
    }
    for entity in &veil {
        commands.entity(entity).despawn();
    }
    let (Some(what), Some(fonts)) = (want.clone(), fonts) else {
        // Without fonts nothing can be drawn yet; leaving `shown` alone means
        // this is tried again on the next frame.
        if want.is_none() {
            *shown = None;
        }
        return;
    };
    *shown = want;

    let root = commands
        .spawn((
            Veil,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(VEIL),
            // Over everything, and it swallows clicks on purpose: a control
            // pressed during a wait would queue an action for a screen that
            // is about to be replaced.
            GlobalZIndex(400),
        ))
        .id();
    // The wait's own scene (#295): the front door's passage held open. It
    // dims what is behind it by itself, so the plain veil gives way to it.
    if let Some(mut scene) = scene {
        let surface = vista::surface(&mut commands, &mut scene, Vista::Wait);
        commands
            .entity(root)
            .insert(BackgroundColor(Color::NONE))
            .add_child(surface);
    }

    // A panel of its own rather than text straight onto the veil: the screen
    // underneath stays faintly visible on purpose, and a line of text laid
    // directly over a form reads as part of the form.
    let card = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                padding: UiRect::axes(Val::Px(34.0), Val::Px(26.0)),
                border_radius: BorderRadius::all(Val::Px(14.0)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Framed(Vista::Wait),
            Pickable::IGNORE,
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(what),
            tf(&fonts, 20.0),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let dots = commands
        .spawn((
            Node {
                column_gap: Val::Px(10.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for i in 0..3 {
        let dot = commands
            .spawn((
                Node {
                    width: Val::Px(10.0),
                    height: Val::Px(10.0),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(palette::ACCENT),
                Pulse {
                    phase: i as f32 / 3.0,
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(dots).add_child(dot);
    }
    commands.entity(card).add_children(&[label, dots]);
    commands.entity(root).add_child(card);
}

/// Beats the dots.
///
/// Scale and alpha together rather than either alone: alpha alone disappears
/// against a moving ground, and scale alone reads as a wobble.
fn pulse(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut dots: Query<(&Pulse, &mut UiTransform, &mut BackgroundColor)>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    for (dot, mut transform, mut colour) in &mut dots {
        let wave = if still {
            0.5
        } else {
            let turn = (time.elapsed_secs() * PULSE_RATE + dot.phase).fract();
            // A cosine, not a sawtooth: the dot has to come back as smoothly
            // as it went.
            0.5 - 0.5 * (turn * std::f32::consts::TAU).cos()
        };
        transform.scale = Vec2::splat(0.7 + 0.5 * wave);
        colour.0 = palette::ACCENT.with_alpha(0.35 + 0.65 * wave);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Setting and clearing is the whole contract, and `show` has to be
    /// idempotent: callers set it from systems that run every frame.
    #[test]
    fn the_veil_is_asked_for_by_name() {
        let mut loading = Loading::default();
        assert_eq!(loading.what(), None);
        loading.show("Signing in");
        assert_eq!(loading.what(), Some("Signing in"));
        loading.clear();
        assert_eq!(loading.what(), None);
    }

    /// The veil is built when a wait is asked for and dropped when it ends,
    /// and — the part that broke a first version — a wait asked for again
    /// every frame rebuilds nothing, so the dots keep their cycle.
    #[test]
    fn the_veil_is_built_once_per_wait() {
        let mut app = veil_app();

        tick(&mut app, 16);
        assert_eq!(veils(&mut app), 0, "nothing is waited for yet");

        app.world_mut().resource_mut::<Loading>().show("Waiting");
        tick(&mut app, 300);
        assert_eq!(veils(&mut app), 1);
        let built = app
            .world_mut()
            .query_filtered::<Entity, With<Veil>>()
            .iter(app.world())
            .next()
            .unwrap();

        for _ in 0..3 {
            app.world_mut().resource_mut::<Loading>().show("Waiting");
            tick(&mut app, 16);
        }
        assert_eq!(veils(&mut app), 1);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<Veil>>()
                .iter(app.world())
                .next(),
            Some(built),
            "the same veil, not a new one every frame"
        );

        app.world_mut().resource_mut::<Loading>().clear();
        tick(&mut app, 16);
        assert_eq!(veils(&mut app), 0);
    }

    /// A wait that is over before it could be read draws nothing at all.
    ///
    /// This is the offline lobby's flicker: every request there is answered in
    /// this process and read back on the next frame, so the veil went up and
    /// came down again inside two frames on every click. The counter-test is
    /// the other half of the claim — the same wait, held past [`GRACE`], is
    /// still said out loud.
    #[test]
    fn a_wait_too_short_to_read_is_never_drawn() {
        let mut app = veil_app();

        app.world_mut().resource_mut::<Loading>().show("One moment");
        tick(&mut app, 16);
        tick(&mut app, 16);
        assert_eq!(veils(&mut app), 0, "two frames is a flash, not a wait");
        app.world_mut().resource_mut::<Loading>().clear();
        tick(&mut app, 16);
        assert_eq!(veils(&mut app), 0, "and it never appears afterwards");

        app.world_mut().resource_mut::<Loading>().show("One moment");
        tick(&mut app, 300);
        assert_eq!(veils(&mut app), 1, "a wait long enough to read is drawn");
    }

    /// A wait that changes its words mid-flight keeps the veil it already has.
    ///
    /// The clock runs on *whether* something is being waited for, not on what
    /// it is called: the lobby says "One moment" while it fetches and "Taking
    /// your seat" once it has, and a grace that restarted there would take the
    /// veil away in the middle of the one wait that is genuinely long.
    #[test]
    fn changing_the_words_does_not_start_the_grace_again() {
        let mut app = veil_app();

        app.world_mut().resource_mut::<Loading>().show("One moment");
        tick(&mut app, 300);
        assert_eq!(veils(&mut app), 1);

        app.world_mut()
            .resource_mut::<Loading>()
            .show("Taking your seat");
        tick(&mut app, 16);
        assert_eq!(veils(&mut app), 1, "the same wait, in different words");
    }

    /// An app with just enough in it to raise the veil.
    fn veil_app() -> App {
        let mut app = App::new();
        app.init_resource::<Loading>()
            .init_resource::<Time>()
            .insert_resource(UiFonts {
                text: Handle::default(),
                medium: Handle::default(),
                bold: Handle::default(),
                italic: Handle::default(),
                medium_italic: Handle::default(),
                serif: Handle::default(),
                serif_italic: Handle::default(),
                icons: Handle::default(),
                mana: Handle::default(),
            })
            .add_systems(Update, raise);
        app
    }

    /// One frame, `ms` milliseconds after the one before it.
    fn tick(app: &mut App, ms: u64) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(ms));
        app.update();
    }

    /// How many veils are on screen.
    fn veils(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<Entity, With<Veil>>()
            .iter(app.world())
            .count()
    }
}
