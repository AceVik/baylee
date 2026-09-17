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
    /// The finger has come off the card it was on and nothing has answered.
    ///
    /// A release over the same card is left to the click that follows it,
    /// which is where the answer is known — but bevy raises a
    /// `Pointer<Click>` only when the press and the release land on the same
    /// **entity**, and this hand is rebuilt on every hover change and on
    /// every view that arrives. A tree rebuilt between the two swallows the
    /// click, nothing is ever written back, and the tap the player made never
    /// happened at all.
    ///
    /// [`Self::swallowed_tap`] is what that costs now: the end of
    /// [`crate::input::pointer`] reads it, once the frame's clicks are spent,
    /// and sends the tap through the same branch a click would have gone
    /// through. [`settle`] keeps a lift of its own for a flag that somehow
    /// outlives all of that, which is this module's own promise — a card
    /// never stays sunk — rather than the tap's.
    let_go: bool,
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
        self.let_go = false;
        if let Some(object) = self.under_the_finger.take()
            && let Some(touch) = self.cards.get_mut(&object)
        {
            touch.up(answer);
        }
    }

    /// The finger came off over `card` — `None` for the felt, or for a node
    /// that is not a card in the row.
    ///
    /// Coming off the card it was on is not an answer and is not a refusal
    /// either: it is the half of a tap whose other half is the click. So it
    /// is recorded and not acted on, and [`Self::let_go`] says what happens
    /// when the click never comes.
    fn release_over(&mut self, card: Option<ObjectId>) {
        if card.is_some() && card == self.under_the_finger {
            self.let_go = true;
        } else {
            self.lift_the_finger();
        }
    }

    /// The card whose tap nobody heard, if there is one.
    ///
    /// A finger that went down on a card and came up on the same card with no
    /// `Pointer<Click>` in between is a tap the player made and the tree ate.
    /// It has to be read *after* the frame's clicks and not before them: a tap
    /// that **was** heard sets this too, until the click clears it, so acting
    /// on it first would play every land twice.
    #[must_use]
    pub fn swallowed_tap(&self) -> Option<ObjectId> {
        self.let_go.then_some(self.under_the_finger).flatten()
    }

    /// Takes the finger off whatever it was on without answering anything.
    fn lift_the_finger(&mut self) {
        self.let_go = false;
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
    hand_cards: Query<&crate::hud::HandCardVisual, With<crate::hud::HandRowCard>>,
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
        touched.release_over(landed.map(|c| c.object));
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
    mut nodes: Query<(&crate::hud::HandCardVisual, &mut Node), With<crate::hud::HandRowCard>>,
    mut shades: Query<(&Shade, &mut BackgroundColor)>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let dt = time.delta_secs();
    let armed = duel.armed.as_ref().map(|a| a.object);
    let hovered = duel.hovered;
    let touched = &mut *touched;

    // A backstop, and it is meant to be one. `input::pointer` ends by reading
    // `Touched::swallowed_tap` and answering it, so in the wired client the
    // flag is always down by the time this runs; what is left here is this
    // module's own promise — a card never stays sunk — held for a schedule
    // that draws the hand without running the input path, which is exactly
    // what the tests below are. The ordinary rate rather than the heavy one,
    // because a card that got this far was never refused: nothing asked.
    if touched.let_go {
        touched.lift_the_finger();
    }

    // A card the *keyboard* armed has never been touched, so it has no entry
    // and would simply appear eight pixels out of the row. Making the entry
    // here is what lets it travel, and disarming travel back; it is made only
    // for a card the hand is actually drawing, so an armed permanent on the
    // felt puts nothing in this map.
    for object in [armed, hovered].into_iter().flatten() {
        if nodes.iter().any(|(card, _)| card.object == object) {
            touched.cards.entry(object).or_default();
        }
    }

    for (object, touch) in &mut touched.cards {
        let rest = if armed == Some(*object) {
            -crate::hud::ARMED_RAISE
        } else if hovered == Some(*object) {
            -crate::hud::HOVER_RAISE
        } else {
            0.0
        };
        touch.rests_at(rest);
        touch.advance(dt, still);
    }
    for (card, mut node) in &mut nodes {
        if let Some(touch) = touched.cards.get(&card.object) {
            node.top = Val::Px(touch.lift());
        }
    }
    for (shade, mut colour) in &mut shades {
        let alpha = touched.cards.get(&shade.object).map_or(0.0, Touch::shade);
        colour.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
    }

    // A card that has come to rest is forgotten, and two are never: the one
    // under the finger, because the finger is the reason the entry exists,
    // and the armed one, because an armed card comes to rest *out* of the row
    // and forgetting it there would hand the next press a `Touch` starting
    // from zero — a card that dropped eight pixels to meet a finger that had
    // not moved.
    //
    // After the drawing and not before it: an entry is forgotten on the frame
    // it settles, and a node born this frame carries whatever the *previous*
    // frame's pose was, so pruning first leaves that stale pose on the screen
    // until something else rebuilds the row.
    let held = touched.under_the_finger;
    touched.cards.retain(|object, touch| {
        Some(*object) == held
            || Some(*object) == armed
            || Some(*object) == hovered
            || !touch.is_settled()
    });
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
    /// The card is built the way the hand zone builds one: a node carrying
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
        let card = hand_card(&mut app, object, 0.0);
        let shade = app
            .world_mut()
            .spawn((Shade { object }, BackgroundColor(Color::NONE)))
            .id();
        app.world_mut().entity_mut(card).add_children(&[shade]);
        (app, object, card, shade)
    }

    /// One node in the hand row, born where the card already is.
    ///
    /// Both components, because that is what `spawn_hand_zone` puts on it and
    /// the pair is the point: the wider one is what a click and a hover are
    /// resolved through, the marker is what says this node is the row's.
    fn hand_card(app: &mut App, object: ObjectId, top: f32) -> Entity {
        app.world_mut()
            .spawn((
                crate::hud::HandCardVisual { object },
                crate::hud::HandRowCard,
                Node {
                    top: Val::Px(top),
                    ..default()
                },
            ))
            .id()
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

    /// Arms `object` the way a key does — with no finger anywhere near it.
    fn arm(app: &mut App, object: ObjectId) {
        app.world_mut().resource_mut::<crate::Duel>().armed = Some(crate::Armed {
            object,
            deed: crate::Deed::Play,
        });
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
    /// The hand zone is rebuilt whenever the pointer crosses a card, so the
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
        // Spawned where the card is, which is what `spawn_hand_zone` asks the
        // resource for.
        let lift = app.world().resource::<Touched>().lift_of(object, 0.0);
        let fresh = hand_card(&mut app, object, lift);
        tick(&mut app, 1.0 / 60.0);
        assert!(
            top_of(&app, fresh) >= before,
            "the card fell back to {} from {before}",
            top_of(&app, fresh)
        );
        let _ = shade;
    }

    /// The same rebuild, across the *whole* tap — and the card must come back.
    ///
    /// Bevy raises a `Pointer<Click>` only when the press and the release
    /// land on the same **entity**. A hand rebuilt between the two therefore
    /// answers nothing at all, and nothing is what the card was left holding:
    /// sunk and dark until the player pressed something else. Sitting between
    /// this and the test above is the whole of the bug — one of them wants
    /// the entry kept across the rebuild and the other wants the finger let
    /// go of despite it.
    #[test]
    fn a_rebuild_across_the_whole_tap_does_not_leave_the_card_pressed() {
        let (mut app, object, card, _) = harness();
        press(&mut app, card);
        tick(&mut app, 0.030);
        assert!(top_of(&app, card) > 0.0, "the press was seen");

        // The rebuild takes the shade pane with it, as a rebuild does: the
        // pane is the card node's child.
        app.world_mut().entity_mut(card).despawn();
        let lift = app.world().resource::<Touched>().lift_of(object, 0.0);
        let fresh = hand_card(&mut app, object, lift);
        let shade = app
            .world_mut()
            .spawn((Shade { object }, BackgroundColor(Color::NONE)))
            .id();
        app.world_mut().entity_mut(fresh).add_children(&[shade]);
        // The finger comes up over the same card, on the node that replaced
        // the one it went down on. No click follows.
        release(&mut app, fresh);
        tick(&mut app, 1.0);

        assert!(
            top_of(&app, fresh).abs() < 0.05,
            "the card is still {} px down",
            top_of(&app, fresh)
        );
        assert!(
            alpha_of(&app, shade).abs() < 0.005,
            "the card is still {} dark",
            alpha_of(&app, shade)
        );
    }

    #[test]
    fn hovering_lifts_a_card_and_leaving_returns_it_to_the_row() {
        let (mut app, object, card, _) = harness();
        app.world_mut().resource_mut::<crate::Duel>().hovered = Some(object);
        tick(&mut app, 1.0 / 60.0);
        assert!(top_of(&app, card) < 0.0 && top_of(&app, card) > -crate::hud::HOVER_RAISE);
        tick(&mut app, 1.0);
        assert!((top_of(&app, card) + crate::hud::HOVER_RAISE).abs() < 0.05);
        app.world_mut().resource_mut::<crate::Duel>().hovered = None;
        tick(&mut app, 1.0);
        assert!(top_of(&app, card).abs() < 0.05);
    }

    /// A card armed from the keyboard travels out of the row instead of
    /// appearing out of it.
    ///
    /// It is the case the hand-written `Default` was written for, and until
    /// the entry was made here the reason on it was fiction: nothing ever
    /// gave an untouched card a rest to travel to.
    #[test]
    fn a_card_the_keyboard_armed_travels_out_of_the_row() {
        let (mut app, object, card, _) = harness();
        arm(&mut app, object);
        tick(&mut app, 1.0 / 60.0);
        let first = top_of(&app, card);
        assert!(
            first < 0.0 && first > -crate::hud::ARMED_RAISE,
            "one frame in, the card is at {first} and not yet home"
        );
        tick(&mut app, 1.0);
        assert!(
            (top_of(&app, card) + crate::hud::ARMED_RAISE).abs() < 0.05,
            "it arrived at {}",
            top_of(&app, card)
        );
    }

    /// The second tap on an armed card gives way from where the card *is*.
    ///
    /// An armed card at rest is settled, so the entry it travelled on is
    /// forgotten — and a press that then made a fresh one would start it at
    /// the row, dropping the card the whole armed raise in one frame to meet
    /// a finger that had not moved.
    #[test]
    fn a_press_on_a_resting_armed_card_does_not_drop_it_to_the_row() {
        let (mut app, object, card, _) = harness();
        arm(&mut app, object);
        tick(&mut app, 1.0);
        tick(&mut app, 1.0);
        assert!(
            (top_of(&app, card) + crate::hud::ARMED_RAISE).abs() < 0.05,
            "armed and at rest at {}",
            top_of(&app, card)
        );

        press(&mut app, card);
        tick(&mut app, 1.0 / 60.0);
        let sunk = top_of(&app, card);
        assert!(
            sunk < -crate::hud::ARMED_RAISE + Touch::SINK + 0.5,
            "the card jumped to {sunk} instead of giving way from -{}",
            crate::hud::ARMED_RAISE
        );
    }

    /// One slot in the stack panel: the wider component and not the marker.
    fn stack_slot(app: &mut App, object: ObjectId) -> Entity {
        app.world_mut()
            .spawn((
                crate::hud::HandCardVisual { object },
                Node {
                    top: Val::Px(0.0),
                    ..default()
                },
            ))
            .id()
    }

    /// Casting a card moves it from the hand to the stack, and the touch it
    /// was given in the hand must not follow it there.
    ///
    /// The stack panel's rows carry `HandCardVisual` too — that is what makes
    /// a spell on the stack hover, preview and answer a click like any other
    /// card — and an `ObjectId` survives the zone change, so the card that was
    /// pressed a tenth of a second ago is drawn by a node with the same id on
    /// it.
    #[test]
    fn a_stack_slot_is_not_moved_by_a_finger_that_was_in_the_hand() {
        let (mut app, object, card, _) = harness();
        press(&mut app, card);
        tick(&mut app, 0.030);
        assert!(top_of(&app, card) > 0.0, "the press was seen");

        app.world_mut().entity_mut(card).despawn();
        let slot = stack_slot(&mut app, object);
        tick(&mut app, 1.0 / 60.0);
        assert!(
            top_of(&app, slot).abs() < f32::EPSILON,
            "the stack slot moved to {}",
            top_of(&app, slot)
        );
    }

    /// And a press on a stack slot is not a finger in the hand.
    ///
    /// Asserted through `lift_of`, which is the question `spawn_hand_zone`
    /// asks — so what it says is what the row would be built from.
    #[test]
    fn a_press_on_a_stack_slot_puts_no_finger_on_anything() {
        let (mut app, _, card, _) = harness();
        let onstack = ObjectId::new(9, 0);
        let slot = stack_slot(&mut app, onstack);
        press(&mut app, slot);
        tick(&mut app, 0.060);
        let lift = app.world().resource::<Touched>().lift_of(onstack, 0.0);
        assert!(lift.abs() < f32::EPSILON, "the slot was pressed to {lift}");
        assert!(
            top_of(&app, card).abs() < f32::EPSILON,
            "and it was not the hand card that gave way either"
        );
    }
}
