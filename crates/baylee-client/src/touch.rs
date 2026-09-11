//! The finger on a card in the hand, drawn.
//!
//! [`baylee_client_core::touch`] is the whole of the arithmetic and this is
//! the wiring: a map from `ObjectId` to [`Touch`], two messages read to write
//! into it, and one system that spends what it says on the two things a
//! `bevy_ui` node can carry — its `top`, and the alpha of a black pane laid
//! over the card's art.
//!
//! # Why a pane and not a tint
//!
//! A card's face is a `CardMaterial`, and its look rides a hashable **key**
//! that is baked once and cached; nothing in this client writes a card
//! uniform per frame, which is the constraint `cardmat`'s own header states
//! and the reason a GL backend could ever draw any of this. A press that
//! darkened the material would be a fresh material and a fresh handle sixty
//! times a second for as long as a finger was down. So the darkening is a
//! `BackgroundColor` on a child node instead — exactly what `Feel` writes for
//! a button, on the one surface `bevy_ui` animates for free.
//!
//! # Why the press is read from a message and not from `PickingInteraction`
//!
//! `Feel` reads the interaction component, and that works because a button is
//! the entity the pointer is over. A hand card is not: the art, the text and
//! the rail are child nodes, and a child swallows the hover from its parent —
//! which is why [`crate::input::pointer`] resolves a click through
//! `find_in_lineage` rather than off the clicked entity. The press is read the
//! same way, through the same lineage, so the two cannot disagree about which
//! card was touched.

use baylee_client_core::touch::{Answer, Touch};
use baylee_core::ids::ObjectId;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Every card that is not lying flat in the row.
///
/// Keyed by the card and not by the entity, because the hand is rebuilt on
/// every hover change and the entity under a finger is replaced while the
/// finger is still on it. An empty map is the ordinary state: an entry is
/// made when a card is touched and forgotten again as soon as it has nothing
/// left to draw.
#[derive(Resource, Default)]
pub struct Touched {
    /// The cards with something to draw.
    cards: HashMap<ObjectId, Touch>,
    /// The card the finger is currently down on.
    ///
    /// Held because a release names the card it landed on, and a press
    /// dragged off one card and released over another has to lift the *first*
    /// one. Without it a card could be left pressed for ever.
    under_the_finger: Option<ObjectId>,
}

impl Touched {
    /// Where `object` should be drawn, in pixels below the row.
    ///
    /// The fall-back is the pose the tree would have drawn anyway, so a card
    /// nothing has touched costs nothing and reads the same as it always did.
    #[must_use]
    pub fn lift_of(&self, object: ObjectId, rest: f32) -> f32 {
        self.cards.get(&object).map_or(rest, Touch::lift)
    }

    /// Puts the finger down on `object`, lifting whatever it was on before.
    fn press(&mut self, object: ObjectId) {
        self.lift_the_finger();
        self.cards.entry(object).or_default().down();
        self.under_the_finger = Some(object);
    }

    /// Answers the tap the finger is making, if it is making one.
    fn release(&mut self, answer: Answer) {
        if let Some(object) = self.under_the_finger.take()
            && let Some(touch) = self.cards.get_mut(&object)
        {
            touch.up(answer);
        }
    }

    /// Takes the finger off whatever it was on without answering anything.
    fn lift_the_finger(&mut self) {
        if let Some(object) = self.under_the_finger.take()
            && let Some(touch) = self.cards.get_mut(&object)
        {
            touch.released_elsewhere();
        }
    }
}

/// Reads the press and the release, and says which card each one landed on.
///
/// The release carries no [`Answer`] of its own: the answer is
/// [`crate::input::pointer`]'s, written straight into the map when it resolves
/// the click, and this system only has to handle the release that resolves
/// *nothing* — a press dragged off the card, or let go over the felt.
pub fn watch_the_finger(
    mut downs: MessageReader<Pointer<Press>>,
    mut ups: MessageReader<Pointer<Release>>,
    hand_cards: Query<&crate::hud::HandCardVisual>,
    parents: Query<&ChildOf>,
    mut touched: ResMut<Touched>,
) {
    for down in downs.read() {
        if let Some(card) = crate::input::find_in_lineage(down.entity, &hand_cards, &parents) {
            touched.press(card.object);
        } else {
            touched.lift_the_finger();
        }
    }
    // A release over the card the finger is on is left alone: the click that
    // follows it is what says whether the tap took, and it arrives on the
    // same frame. Everything else is a press that asked nothing.
    for up in ups.read() {
        let landed = crate::input::find_in_lineage(up.entity, &hand_cards, &parents);
        if landed.map(|c| c.object) != touched.under_the_finger {
            touched.lift_the_finger();
        }
    }
}

/// Answers the tap the finger is making.
///
/// Called by [`crate::input::pointer`] the moment a hand card's click has
/// been resolved, because the function that decides what a tap did is the one
/// that can say so — reconstructing it here from what changed in `Duel` would
/// be a second reading of the same branch, kept in step by hand.
pub fn answer(touched: &mut Touched, answer: Answer) {
    touched.release(answer);
}

/// A black pane over one hand card's face, which is how a press is darkened.
#[derive(Component, Clone, Copy)]
pub struct Shade {
    /// The card it covers.
    pub object: ObjectId,
}

/// Moves every touched card and spends what it says.
///
/// Runs after the HUD rebuild so a node spawned this frame is written before
/// it is ever drawn, and the rest pose is read from `Duel::armed` rather than
/// from the board model: *armed* is the one fact that moves a card's resting
/// place, it is already the truth the tree is built from, and reading it in
/// one place is what keeps the animated pose and the built one from drifting.
pub fn settle(
    time: Res<Time>,
    duel: Res<crate::Duel>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut touched: ResMut<Touched>,
    mut nodes: Query<(&crate::hud::HandCardVisual, &mut Node)>,
    mut shades: Query<(&Shade, &mut BackgroundColor)>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let dt = time.delta_secs();
    let armed = duel.armed.as_ref().map(|a| a.object);
    let touched = &mut *touched;
    for (object, touch) in &mut touched.cards {
        let rest = if armed == Some(*object) {
            -crate::hud::ARMED_RAISE
        } else {
            0.0
        };
        touch.rests_at(rest);
        touch.advance(dt, still);
    }
    // A card that has come to rest is forgotten, and one still under the
    // finger never is — the finger is the reason the entry exists.
    let held = touched.under_the_finger;
    touched
        .cards
        .retain(|object, touch| Some(*object) == held || !touch.is_settled());

    for (card, mut node) in &mut nodes {
        if let Some(touch) = touched.cards.get(&card.object) {
            node.top = Val::Px(touch.lift());
        }
    }
    for (shade, mut colour) in &mut shades {
        let alpha = touched.cards.get(&shade.object).map_or(0.0, Touch::shade);
        colour.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
    }
}

/// The systems, *run* — a press that is declared and never wired is a bug
/// this client has shipped before, and every assertion here is on a `Node` or
/// a `BackgroundColor` rather than on `update()` having returned.
#[cfg(test)]
mod running {
    use super::*;
    use bevy::picking::events::{Pointer, Press, Release};
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::window::{PrimaryWindow, WindowRef};

    /// One hand card in a world with the two systems in it.
    ///
    /// The card is built the way the hand bar builds one: a node carrying
    /// [`crate::hud::HandCardVisual`], with the shade pane as its child. The
    /// `Time` is advanced by hand, so every number below is a duration and
    /// not a frame count.
    fn harness() -> (App, ObjectId, Entity, Entity) {
        let mut app = App::new();
        app.add_message::<Pointer<Press>>()
            .add_message::<Pointer<Release>>()
            .init_resource::<Time>()
            .init_resource::<Touched>()
            .init_resource::<crate::Duel>()
            .add_systems(Update, (watch_the_finger, settle).chain());
        let mut window = Window::default();
        window.resolution.set(1728.0, 1052.0);
        app.world_mut().spawn((window, PrimaryWindow));
        let object = ObjectId::new(7, 0);
        let card = app
            .world_mut()
            .spawn((
                crate::hud::HandCardVisual { object },
                Node {
                    top: Val::Px(0.0),
                    ..default()
                },
            ))
            .id();
        let shade = app
            .world_mut()
            .spawn((Shade { object }, BackgroundColor(Color::NONE)))
            .id();
        app.world_mut().entity_mut(card).add_children(&[shade]);
        (app, object, card, shade)
    }

    /// Where the pointer is, as the picking backend would report it.
    fn at(app: &mut App) -> Location {
        use bevy::camera::NormalizedRenderTarget;
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .expect("the harness made a window");
        let target = WindowRef::Entity(window)
            .normalize(Some(window))
            .expect("a window is a render target");
        Location {
            target: NormalizedRenderTarget::Window(target),
            position: Vec2::ZERO,
        }
    }

    fn press(app: &mut App, entity: Entity) {
        let location = at(app);
        let event = Press {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
            count: 1,
        };
        app.world_mut()
            .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
    }

    fn release(app: &mut App, entity: Entity) {
        let location = at(app);
        let event = Release {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
        };
        app.world_mut()
            .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
    }

    /// Runs one frame `seconds` long.
    fn tick(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        app.update();
    }

    fn top_of(app: &App, card: Entity) -> f32 {
        match app.world().entity(card).get::<Node>().expect("a node").top {
            Val::Px(px) => px,
            other => panic!("the card's top is {other:?}"),
        }
    }

    fn alpha_of(app: &App, shade: Entity) -> f32 {
        app.world()
            .entity(shade)
            .get::<BackgroundColor>()
            .expect("a background")
            .0
            .alpha()
    }

    #[test]
    fn a_press_on_a_hand_card_sinks_it_and_darkens_it() {
        let (mut app, _, card, shade) = harness();
        tick(&mut app, 1.0 / 60.0);
        assert!(
            top_of(&app, card).abs() < f32::EPSILON,
            "at rest, flat in the row"
        );
        assert!(alpha_of(&app, shade).abs() < f32::EPSILON);

        press(&mut app, card);
        tick(&mut app, 0.060);
        assert!(top_of(&app, card) > 1.0, "{}", top_of(&app, card));
        assert!(alpha_of(&app, shade) > 0.0);
    }

    /// The counter-test for the one above: a press over nothing must not sink
    /// a card, or the assertion there would pass on a system that lifted
    /// every card in the hand on every click anywhere.
    #[test]
    fn a_press_somewhere_else_leaves_the_hand_alone() {
        let (mut app, _, card, shade) = harness();
        let elsewhere = app.world_mut().spawn(Node::default()).id();
        press(&mut app, elsewhere);
        tick(&mut app, 0.060);
        assert!(top_of(&app, card).abs() < f32::EPSILON);
        assert!(alpha_of(&app, shade).abs() < f32::EPSILON);
    }

    /// A tap that nothing answered comes back to the row, slowly, and a card
    /// let go over the felt comes back at the ordinary rate.
    #[test]
    fn a_refused_tap_comes_back_heavily_and_a_dragged_off_press_does_not() {
        let (mut app, object, card, _) = harness();
        press(&mut app, card);
        tick(&mut app, 0.5);
        release(&mut app, card);
        app.world_mut()
            .resource_scope(|_, mut touched: Mut<Touched>| {
                answer(&mut touched, Answer::Refused);
            });
        tick(&mut app, 0.100);
        let refused = top_of(&app, card);

        let (mut app, _, card, _) = harness();
        press(&mut app, card);
        tick(&mut app, 0.5);
        // Let go over something that is not the card: nothing was asked, so
        // nothing is refused.
        let elsewhere = app.world_mut().spawn(Node::default()).id();
        release(&mut app, elsewhere);
        tick(&mut app, 0.100);
        let dragged_off = top_of(&app, card);

        assert!(
            refused > dragged_off * 2.0,
            "refused {refused} px, dragged off {dragged_off} px"
        );
        let _ = object;
    }

    /// The rebuild test, and the reason a [`Touch`] lives in a resource.
    ///
    /// The hand bar is rebuilt whenever the pointer crosses a card, so the
    /// node under the finger is despawned and a fresh one takes its place
    /// mid-press. The card must not start again from the row — which is
    /// exactly what an animation held on the entity would do.
    #[test]
    fn a_rebuild_under_the_finger_does_not_start_the_card_again() {
        let (mut app, object, card, shade) = harness();
        press(&mut app, card);
        tick(&mut app, 0.030);
        let before = top_of(&app, card);
        assert!(before > 0.0);

        app.world_mut().entity_mut(card).despawn();
        // Spawned where the card is, which is what `spawn_hand_bar` asks the
        // resource for.
        let lift = app.world().resource::<Touched>().lift_of(object, 0.0);
        let fresh = app
            .world_mut()
            .spawn((
                crate::hud::HandCardVisual { object },
                Node {
                    top: Val::Px(lift),
                    ..default()
                },
            ))
            .id();
        tick(&mut app, 1.0 / 60.0);
        assert!(
            top_of(&app, fresh) >= before,
            "the card fell back to {} from {before}",
            top_of(&app, fresh)
        );
        let _ = shade;
    }
}
