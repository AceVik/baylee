//! The scene index: which entity draws which object, zone and face.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

/// Where every object was in the view before this one, and what has moved
/// since anything last drew.
///
/// A resource of its own rather than a field on [`SceneIndex`], because the
/// two answer different questions and are cleared at different moments: the
/// index is what the scene *is*, this is what the game was. Both are wiped
/// when a table is torn down.
///
/// It holds the moves as well as the tracker because a view produces its
/// batch **once** — `Tracker::observe` answers a view it has already read
/// with nothing — and two systems need that batch: [`crate::sheen`] to say
/// which door an arriving card came in through, and [`sync_scene`] to send a
/// departing one out of the right one. Whichever asks first fills the list;
/// only `sync_scene` empties it, and only once it is past the guards that
/// would otherwise swallow the batch a bailed frame was holding.
#[derive(Resource, Default)]
pub struct ZoneWatch {
    tracker: Tracker,
    undrawn: Vec<zones::Move>,
}

impl ZoneWatch {
    /// Reads a view, if it has not been read, and keeps whatever it said.
    pub fn observe(&mut self, view: &baylee_view::PlayerView) {
        self.undrawn.extend(self.tracker.observe(view));
    }

    /// The moves nothing has drawn yet.
    #[must_use]
    pub fn undrawn(&self) -> &[zones::Move] {
        &self.undrawn
    }

    /// The moves nothing has drawn yet, and they are drawn now.
    pub fn take(&mut self) -> Vec<zones::Move> {
        std::mem::take(&mut self.undrawn)
    }

    /// Forgets everything, for a table that is being torn down.
    pub fn clear(&mut self) {
        self.tracker.clear();
        self.undrawn.clear();
    }
}

/// The text a card's face is showing on the table, and what it was made
/// from ([`SceneIndex::faces`]).
pub(super) struct ShownFace {
    /// The snapshot it was built from.
    pub(super) seq: u64,
    /// Whether its lines were fitted by the font's widths and not the
    /// average's ([`face::Widths`]).
    pub(super) measured: bool,
    /// How many lines its name took: its name bar's depth, which its
    /// material draws.
    pub(super) lines: usize,
    /// The `Text2d` children.
    pub(super) texts: Vec<Entity>,
}

impl ShownFace {
    /// Whether it is still the face to show: built from this snapshot, and
    /// measured the way the table can measure now.
    pub(super) fn is(&self, seq: u64, measured: bool) -> bool {
        self.seq == seq && self.measured == measured
    }
}

/// What a card's text face is this frame (#259).
pub(super) enum FaceNow {
    /// The face on the card is still the one to show; its name is on this
    /// many lines.
    Kept(usize),
    /// A face fitted afresh: the old one's text comes off and this one's
    /// goes on.
    Fitted(Box<(CardFace, face::WorldFit)>),
    /// None shown, and nothing yet to build one from.
    Unbuilt,
}

impl FaceNow {
    /// How many lines its name takes: its name bar's depth.
    pub(super) fn lines(&self) -> usize {
        match self {
            Self::Kept(lines) => *lines,
            Self::Fitted(fitted) => fitted.1.lines(),
            Self::Unbuilt => 1,
        }
    }
}

/// Whether the face `shown` on a card is still the one to show, and if not,
/// the one to put there instead, built by `build` and fitted by `widths`.
pub(super) fn face_now(
    shown: Option<&ShownFace>,
    seq: u64,
    widths: &face::Widths<'_>,
    build: impl FnOnce() -> Option<CardFace>,
) -> FaceNow {
    if let Some(shown) = shown.filter(|shown| shown.is(seq, widths.measured())) {
        return FaceNow::Kept(shown.lines);
    }
    build().map_or(FaceNow::Unbuilt, |built| {
        let fit = face::WorldFit::of(&built, widths);
        FaceNow::Fitted(Box::new((built, fit)))
    })
}

/// The look of a card standing its text face in the window: its colours,
/// what it is, and how deep its name bar is ([`textface::face_word`]), over
/// the flat colour its identity used to be drawn in.
pub(super) fn face_look(
    object: Option<&baylee_view::PublicObject>,
    lines: usize,
    finish: FinishTreatment,
) -> CardLook {
    let colors = object.map_or(ColorSet::EMPTY, |o| o.colors);
    CardLook::flat(face::table_color(colors), finish).with_face(face_word(object, lines))
}

/// The face word of `object` with its name on `lines` lines: what the
/// material draws, and what the text standing on it is inked for.
pub(super) fn face_word(object: Option<&baylee_view::PublicObject>, lines: usize) -> u32 {
    textface::face_word(
        object.map_or(ColorSet::EMPTY, |o| o.colors),
        object.map_or(TypeSet::EMPTY, |o| o.types),
        object.map_or(SubtypeSet::EMPTY, |o| o.subtypes),
        textface::Depths::table(lines),
    )
}

/// Entities currently drawn, keyed by the object they represent.
#[derive(Resource, Default)]
pub struct SceneIndex {
    pub(super) cards: HashMap<ObjectId, Entity>,
    /// One material per *look*, shared by every card wearing it — a board of
    /// forty plain Islands is one material, not forty. A foil Island is a
    /// second: that is a difference the shader draws, and since #298 the
    /// print, its finish and the light passing over it are all it draws.
    pub(super) materials: HashMap<CardLook, Handle<CardMaterial>>,
    pub(super) quad: Option<Handle<Mesh>>,
    pub(super) blank: Option<Handle<CardMaterial>>,
    /// Text entities of the constructed face, per card currently showing one,
    /// with the snapshot they were built from.
    ///
    /// Held here rather than found by query because the face comes and goes
    /// with a held key: the entities have to be removed as cheaply as they
    /// were made, and a card that no longer wants one must not keep a stale
    /// line of text glued to it. The sequence number is what rebuilds a face
    /// whose card changed (an anthem, a counter, a clone) without rebuilding
    /// every face every frame.
    pub(super) faces: HashMap<ObjectId, ShownFace>,
    /// The strip lying on each card with something to say (#274, #298): what
    /// it says and the row step it was put on for, and the strip itself.
    ///
    /// Held here for the reason [`Self::faces`] is: a strip comes and goes
    /// with what the rules do to the card, and has to be taken off as cheaply
    /// as it was put on.
    pub(super) marks: HashMap<ObjectId, (cardrail::Strip, f32, Entity)>,
    /// One strip material per thing a strip says, shared by every card
    /// saying it — a lane of twelve Soldiers with the same keywords is one.
    pub(super) marks_materials: HashMap<cardrail::Strip, Handle<MarksMaterial>>,
    /// The quad every strip is drawn on: [`cardrail::quad_rect`], one mesh
    /// for the whole table, since the shader sizes the strip inside it.
    pub(super) marks_quad: Option<Handle<Mesh>>,
    /// The count badge at each merged card's corner (#261): what it was put
    /// on for (the count, the row step, where it stands and whether its card
    /// is tapped) and the badge itself. Held for the strip's reason.
    pub(super) badges: HashMap<ObjectId, (BadgeKey, Entity)>,
    /// One badge material per count and place, shared by every card saying
    /// it.
    pub(super) badge_materials: HashMap<(u32, BadgePlace, bool), Handle<BadgeMaterial>>,
    /// The quad every badge is drawn on: [`cardplate::badge_quad_rect`], one
    /// mesh for the whole table, since the shader sizes the body inside it.
    pub(super) badge_quad: Option<Handle<Mesh>>,
    /// The plate at each card whose plate shows (the owner, 25.09): what it
    /// was put on for ([`PlateKey`]) and the plate itself. Held for the
    /// strip's reason.
    pub(super) plates: HashMap<ObjectId, (PlateKey, Entity)>,
    /// One plate material per thing a plate says, shared by every card
    /// saying it.
    pub(super) plate_materials: HashMap<PlateWords, Handle<PlateMaterial>>,
    /// The quad every plate is drawn on: [`cardplate::plate_quad`], one mesh
    /// for the whole table, since the shader lays the body out inside it.
    pub(super) plate_quad: Option<Handle<Mesh>>,
    /// The offer's light on the felt under each card this client is offering
    /// something for (#298): the offers and the depth it was put at, and the
    /// light itself. Held for the strip's reason, and it comes and goes with
    /// priority, which is far more often.
    pub(super) floors: HashMap<ObjectId, (u32, f32, Entity)>,
    /// One light material per combination of offers: at most sixteen.
    pub(super) floor_materials: HashMap<u32, Handle<FloorMaterial>>,
    /// The quad every light is drawn on: [`floormat::quad_size`], one mesh
    /// for the whole table.
    pub(super) floor_quad: Option<Handle<Mesh>>,
    /// The shells round each protected permanent: indestructible's steel
    /// and hexproof's or shroud's dome, each with the ring it lies down to,
    /// all children of the card, and how each stands. [`fit_the_shells`]
    /// decides that every frame, from where every card is.
    pub(super) shells: HashMap<ObjectId, Shell>,
    /// One material per look of shell.
    pub(super) shell_materials: HashMap<ShellLook, Handle<ShellMaterial>>,
    /// The rim's mesh, one for the whole table.
    pub(super) rim_mesh: Option<Handle<Mesh>>,
    /// A ring's mesh for each part of its band.
    pub(super) ring_meshes: HashMap<Band, Handle<Mesh>>,
    /// Each dome's mesh at each of its steps.
    pub(super) dome_meshes: HashMap<(shellmat::Dome, usize), Handle<Mesh>>,
    /// Each dome's shadow on the felt at each step it stands on the felt at.
    pub(super) dome_shades: HashMap<(shellmat::Dome, usize), Handle<Mesh>>,
    /// Defender's wall, one for the whole table, and its shadow.
    pub(super) wall_mesh: Option<Handle<Mesh>>,
    pub(super) wall_shade: Option<Handle<Mesh>>,
    /// Summoning sickness's wave, one for the whole table.
    pub(super) wave_mesh: Option<Handle<Mesh>>,
    /// What stands under each card on the table: the slabs of its deck and
    /// its contact shadow, with the count they were built for (#261). Held for the strip's reason, and because a group grows under
    /// the same top card: a deck built once at spawn kept one slab under a
    /// card that had risen to stand on eleven.
    pub(super) stacks: HashMap<ObjectId, Stack>,
    /// What stood in a pile's hover fan on the **previous** frame, and which
    /// pile each card came out of.
    ///
    /// Previous and not current, which is the whole reason it is kept at all.
    /// A fan closes by leaving the board model, so the frame that has to send
    /// its cards home is a frame on which they are already gone — and a card
    /// that leaves with no zone change behind it is otherwise despawned where
    /// it stands, which for seven cards in the air is seven cards blinking
    /// out of it.
    pub(super) fanned: HashMap<ObjectId, Place>,
    /// Whose library is standing open.
    ///
    /// A library's fan is blank slabs rather than placements — see
    /// [`sync_library_fan`] — so it is the one part of the scene that is not
    /// keyed by `ObjectId` and has to be remembered by hand.
    pub(super) library_fan: Option<PlayerId>,
    /// The slabs of that fan, in order from the top of the deck.
    pub(super) library_fan_cards: Vec<Entity>,
    /// One material per colour identity and look, for cards drawing their
    /// own face rather than artwork.
    pub(super) face_materials: HashMap<CardLook, Handle<CardMaterial>>,
    /// Whether the two caches above were filled to hold still.
    ///
    /// The same trick as [`UiCardMaterials`](crate::cardmat::UiCardMaterials):
    /// `false` is "animated", so the derived `Default` is the right answer,
    /// and the setting lives beside the cache instead of inside its key —
    /// which is the whole point of keeping it here. Were it in the key, every
    /// card on the table would be two entries instead of one and nothing
    /// would ever evict the half no longer wanted.
    ///
    /// A change rewrites the clock on the materials already made rather than
    /// throwing them away. Emptying the maps is the obvious thing to write
    /// and it is strictly worse: the handles are still held by every card
    /// entity, so the discarded materials do not go anywhere — they are
    /// merely rebuilt, once each, on the next frame.
    pub(super) still: bool,
    /// Whether [`Self::blank`] is wearing the printed card back yet.
    ///
    /// `false` until the picture has arrived, which is also the right answer
    /// for a client that never reaches Scryfall at all: the flat colour is
    /// what it goes on drawing.
    pub(super) back_dressed: bool,
    /// A seat's zone: the mat and the glow under it, with the mood they were
    /// last drawn in. Held here for the same reason the cards are — so a
    /// frame in which nothing changed costs a lookup and no allocation.
    pub(super) zones: HashMap<PlayerId, Zone>,
    /// The soft glow every zone shares. It is white with the falloff in its
    /// alpha, so one image serves the whole table and the seat's colour is
    /// the material's tint.
    ///
    /// The mat is *not* here, and used to be. Sharing one image meant the
    /// seat's colour could only be applied as a tint over the whole of it,
    /// which is how a gilt-rimmed board became a sheet of brass; the rim
    /// carries the colour now, so the image is per seat and lives on the
    /// [`Zone`].
    pub(super) glow_image: Option<Handle<Image>>,
    /// The contact shadow every card sits in: one quad, one material, shared
    /// by the whole table. It is a child of the card, so it follows the tap
    /// rotation and the hover lift with nothing to keep in step.
    pub(super) shadow_quad: Option<Handle<Mesh>>,
    pub(super) shadow_material: Option<Handle<StandardMaterial>>,
    /// The empty place a pile stands in, one material per
    /// [`baylee_client_core::PileKind`] in `ALL` order.
    ///
    /// Shared by the whole table, because a well is table furniture and
    /// carries no seat's colour. One material per *kind* rather than one for
    /// all of them because each carries its zone's own mark baked into the
    /// texture — the table is 3D and has no text on it, so the mark is
    /// arithmetic in the well rather than a label over it.
    pub(super) wells: Vec<Handle<StandardMaterial>>,
}

/// One seat's zone on the table.
pub(super) struct Zone {
    /// The place this zone was built for.
    ///
    /// Everything below is *geometry*: a mesh cut to the mat's size, a glow
    /// quad under it, and pile places at fixed points beside it. None of it
    /// is a uniform that can be rewritten, so a seat whose slot has moved or
    /// changed size has to be built again — and until this was here it never
    /// was. The layout is legitimately rebuilt more than once (the first one
    /// is drawn against a guessed canvas aspect, and focusing a seat widens
    /// it and shrinks the rest), so a table laid out a second time drew every
    /// mat at the first layout's size while the cards moved to the second.
    /// Measured: the local seat's half width was `9.864` when its zone was
    /// built and `12.841` when its commander was placed, which is a card
    /// standing two and a half card widths outside the mat it belongs to.
    pub(super) slot: SeatSlot,
    /// How high over the table the zone was carried last (a tearing
    /// table's turning piece, DESIGN-v8): the next carry adds the change.
    pub(super) raised: f32,
    /// The mat itself.
    pub(super) mat: Entity,
    /// The pool of colour under it.
    pub(super) glow: Entity,
    /// Their materials, held rather than looked back up off the entities.
    ///
    /// The two used to be found with a `Query<&MeshMaterial3d<_>>`, which was
    /// a system parameter for something this already knows: it spawned both
    /// of them. Now that they are two *different* material types that query
    /// would have had to be two queries, and `sync_zones` is at clippy's
    /// argument budget.
    pub(super) mat_material: Handle<crate::matmat::MatMaterial>,
    pub(super) glow_material: Handle<StandardMaterial>,
    /// The pile places beside it, and the face-down library standing on one
    /// of them. Empty when the scene index has no card mesh yet.
    pub(super) piles: Vec<Entity>,
    /// How tall the library was drawn, in cards.
    ///
    /// Capped at the height a pile is ever drawn to, so a deck that is
    /// visibly shrinking is rebuilt as it shrinks and one that is far past
    /// the cap is not rebuilt at all. It used to be "had the library run
    /// out" alone, which is the one fact the *places* depend on and says
    /// nothing about the deck standing on one of them.
    pub(super) library_shown: u32,
    /// What the mat was last drawn for.
    pub(super) mood: Mood,
    /// The seat colour in the mat's rim.
    ///
    /// Kept so a seat whose accent changes is redrawn rather than keeping the
    /// colour it was born with. [`seat_accent`] reads `is_local` and
    /// `ring_index`, both of which are stable while a seat is at the table —
    /// but "stable in practice" is exactly the assumption that put a stale
    /// hover and an over-tall panel on screen this week, and a comparison is
    /// cheaper than being right about it.
    pub(super) accent: Color,
}

/// What a zone's colour is saying.
///
/// The rim of a seat's mat is the cheapest place to answer "whose turn is
/// it?" and "who is holding everyone up?" — questions a player asks on every
/// single priority pass, and that otherwise cost a trip to the overlay.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Mood {
    /// Whether this is the viewing seat.
    pub(super) local: bool,
    /// Where the seat stands in the turn.
    pub(super) standing: Standing,
    /// Whether the turn is this seat's, which is a different question from
    /// [`standing`](Self::standing) and has to be kept beside it.
    ///
    /// `Standing` is a rank and collapses the two: a seat holding priority
    /// reads as `Priority` whether or not the turn is theirs, because what
    /// the *brightness* answers is "who is everybody waiting for". The light
    /// running round a mat's rim answers "whose turn is it", and on every
    /// turn where an opponent responds to something the two have different
    /// answers — so a rim light driven off the rank would leave the active
    /// seat and follow the response, which is precisely backwards.
    pub(super) on_turn: bool,
    /// Whether nobody is sitting in this chair — the house is answering for a
    /// player who has gone ([`SeatRole::Away`]).
    ///
    /// On the `Mood` and not passed beside it, which is the whole reason it
    /// is here: `sync_zones` skips a seat whose `mood` and `accent` are both
    /// unchanged, so a flag outside this struct would be written once when
    /// the mat was built and never again. A chair handed to the house
    /// mid-game would keep a solid rim until something else about the seat
    /// happened to move.
    pub(super) held: bool,
}

/// What a seat is doing, in the order the zone cares about it.
///
/// Ordered rather than flagged because these do not stack: the seat being
/// asked is *also* the active seat nine times out of ten, and drawing both
/// would only mean adding two brightnesses together and hoping.
///
/// `Asked` rather than `SeatPod::is_awaited`'s own word, although the two are
/// fed from one field. A pod mirrors its feed and this names a rung, and the
/// rung sits one line from `Waiting` — which means the opposite and would be
/// a word away from it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Standing {
    /// Out of the game.
    Lost,
    /// The seat the engine is waiting on an answer from — a priority pass,
    /// but equally a block to declare or a card to discard.
    Asked,
    /// Their turn, but not currently holding anyone up.
    Active,
    /// Waiting their turn.
    Waiting,
}

impl Mood {
    /// How a seat's pod reads right now.
    pub(super) fn of(pod: &baylee_client_core::board::SeatPod) -> Self {
        Self {
            local: pod.is_local,
            standing: if pod.has_lost {
                Standing::Lost
            } else if pod.is_awaited {
                Standing::Asked
            } else if pod.is_active {
                Standing::Active
            } else {
                Standing::Waiting
            },
            // A seat that is out of the game is not taking a turn, whatever
            // the view last said about the active player.
            on_turn: pod.is_active && !pod.has_lost,
            // `Away` and not `answered_by_the_house`: an AI chair was always
            // an AI chair and there is nothing provisional about it, while
            // this one is a player's seat being covered until they come back.
            held: pod.role == baylee_client_core::board::SeatRole::Away,
        }
    }
}
