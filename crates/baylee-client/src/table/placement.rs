//! Where every card lies: placements, tucks under a host, transforms.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

/// Puts the printed back on a material that was built without one.
///
/// Both halves or neither: `has_art` is what the shader reads to decide
/// between sampling the picture and filling with the tint, and it follows the
/// *handle* rather than the look — so a material given a texture and not the
/// flag draws exactly what it drew before, which is a fault that looks like
/// the image never arriving.
pub(super) fn dress_in_the_back(material: &mut CardMaterial, back: Handle<Image>) {
    material.art = Some(back);
    material.params.has_art = 1.0;
}

/// Places table-space coordinates into the world.
///
/// Table space has `+y` running away from the local seat; the world has the
/// camera on `+z`, so the two are mirrored on that axis.
pub(crate) fn to_world(table: Vec2, height: f32) -> Vec3 {
    Vec3::new(table.x, height, -table.y)
}

/// The transform of one card.
pub(super) fn card_transform(
    slot: &SeatSlot,
    position: Vec2,
    tapped: bool,
    lift: f32,
) -> Transform {
    // Lay the quad flat, then turn it so it faces its owner, then tap it.
    let mut rotation =
        Quat::from_rotation_y(-slot.facing) * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    if tapped {
        rotation = Quat::from_rotation_y(-slot.facing)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
            * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    }
    Transform {
        translation: to_world(position, TABLE_Y + CARD_LIFT + lift),
        rotation,
        scale: Vec3::ONE,
    }
}

/// One card width down the card, in table units.
///
/// The mesh is [`CARD_HEIGHT`] tall and the shaders measure a card as
/// 1/[`cardrail::CARD_ASPECT`] card widths, and the two differ in the fourth
/// place. Anything placed on the card in the shaders' units goes down it by
/// this, so the strip's bottom edge lands on the seam the card shader draws
/// and not a ten-thousandth beside it.
pub(crate) const DOWN_THE_CARD: f32 = CARD_HEIGHT * cardrail::CARD_ASPECT;

/// How one card of a pile's hover fan is turned.
///
/// A rotation of its own rather than two more arguments to
/// [`card_transform`], because the two are never both true: a card lifted out
/// of a graveyard is not a permanent and has no tap state to compose with.
///
/// The order is the one a card is already built by — lay the quad flat, then
/// turn it to face its owner — with the lying-flat quarter turn short of
/// square by the pose's tilt, so the edge furthest from the owner rises. Which
/// edge that is on *screen* is the pose's business and not this function's;
/// see [`baylee_client_core::FanPose::tilt`].
pub(super) fn fan_rotation(slot: &SeatSlot, pose: baylee_client_core::FanPose) -> Quat {
    Quat::from_rotation_y(-slot.facing)
        * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2 + pose.tilt)
        * Quat::from_rotation_z(pose.yaw)
}

/// Where every group in the current board model belongs.
#[allow(clippy::struct_excessive_bools)] // five facts about one card, each read on its own
pub(super) struct Placement {
    pub(super) object: ObjectId,
    pub(super) slot: SeatSlot,
    pub(super) position: Vec2,
    /// Where this card stands in its row, as a height — see [`LANE_RISE`].
    pub(super) lift: f32,
    pub(super) tapped: bool,
    /// Whether this card has flying, and therefore stands off the felt — see
    /// [`airborne`].
    ///
    /// Read off the group's badges, which is where the client is told: the
    /// engine sends a keyword bitset and `KeywordBadge::from_bits` is what
    /// turns it into facts. Resolved here for the reason the offer and the
    /// corner are — this is where the group's members are.
    pub(super) flying: bool,
    /// Whether this card is an indestructible permanent, and so stands in
    /// its steel rim ([`shellmat`]). Read off the group's badges as flying
    /// is; false for every pile, where indestructible means nothing.
    pub(super) indestructible: bool,
    /// The dome this card stands under, if it has hexproof or shroud
    /// ([`shellmat::Dome::of`]); read and meaningful as `indestructible` is.
    pub(super) dome: Option<shellmat::Dome>,
    /// Whether this card has defender, and so stands behind its wall
    /// ([`shellmat::wall_mesh`]); read and meaningful as `indestructible` is.
    pub(super) defender: bool,
    pub(super) count: usize,
    /// What the count badge says ([`cardplate::count_word`]): how many
    /// permanents this card stands for when it is a merged group on the
    /// battlefield, and zero for a lone card, a pile and a fanned card — a
    /// pile's size is `count` above and is drawn as the deck under it. A
    /// host whose row folds what lies under it wears the attachment mark in
    /// the badge's place instead ([`cardplate::attached_word`], #305); it
    /// never piles, so it never has a count to show.
    pub(super) badge: u32,
    pub(super) art: Option<ImageKey>,
    pub(super) offer: crate::cardmat::Offer,
    pub(super) corner: baylee_client_core::cardplate::Corner,
    pub(super) selected: bool,
    /// Set while this card stands in a pile's hover fan, and then it carries
    /// the pose and the pile the card was lifted out of.
    ///
    /// The pile is here for the way *back*. A fan closes by leaving the board
    /// model, and a card that leaves with no zone change behind it is
    /// despawned where it stands — right for a graveyard's old top card,
    /// covered by the one that landed on it, and wrong for seven cards that
    /// have to drop back into the pile they came out of.
    pub(super) fan: Option<(baylee_client_core::FanPose, PileKind)>,
    /// The keyword strip's word ([`cardrail::badge_bits`]): zero for a card
    /// wearing no marks, which is every card that is not a permanent.
    pub(super) marks: u32,
    /// Whether the creature cannot attack or tap this turn (CR 302.6,
    /// `CardGroup::summoning_sick`): the wave over it, and the plate's
    /// moon-grey ink.
    pub(super) sick: bool,
    /// The identity crests at the strip's end ([`cardcrest::marks`]).
    pub(super) crests: [Option<usize>; cardcrest::MAX_CRESTS],
    /// Whether the card laid after this one in its row covers its lower
    /// right, where the print writes its power and toughness — a fanned
    /// lane — so the plate has to say them (`Corner::shows_plate`).
    pub(super) covered: bool,
    /// What the row leaves this card's plate
    /// ([`LanePacking::plate_room`](baylee_client_core::layout::LanePacking::plate_room)):
    /// [`PlateRoom::OPEN`](cardplate::PlateRoom::OPEN) off the battlefield,
    /// where no plate shows.
    pub(super) room: cardplate::PlateRoom,
    /// How much higher the next card of this card's row stands, which the
    /// strip lies a share of: see [`STRIP_STEP_SHARE`].
    pub(super) rung: f32,
    /// Whether the card is drawn: false for the cards of a scrolled row
    /// outside the run it shows (`LanePacking::window`).
    pub(super) shown: bool,
}

/// The place a pile stands for, for the zone machinery that speaks in places.
///
/// `None` for a library: nothing ever moves *to* one that this table can
/// draw, and nothing is ever lifted out of one that has an object behind it.
pub(super) fn place_of(kind: PileKind, player: PlayerId) -> Option<Place> {
    match kind {
        PileKind::Graveyard => Some(Place::Graveyard(player)),
        PileKind::Exile => Some(Place::Exile(player)),
        PileKind::Command | PileKind::Command2 => Some(Place::Command(player)),
        PileKind::Library => None,
    }
}

/// How far above its row a card stands because of what the card *is*.
///
/// Zero for everything but a creature with flying, which has a height of its
/// own and a slow bob around it ([`airborne`]). The height is returned rather
/// than applied: it joins the row's rise and the deck under the card in the
/// one lift [`card_transform`] is given, so it travels through
/// [`Motion`]/[`glide`] like every other reason a card is where it is. An
/// offset added *after* the glide would be pulled back by the glide on the
/// next frame and would compound.
///
/// `held` freezes the bob at its resting height — the height stays, only the
/// movement stops. It is true while the pointer is on the card, while the
/// card is chosen or armed, and while the player has motion turned off. The
/// first three are the same reading: a card someone is looking at should hold
/// still, and a card bobbing under a pointer that is not moving is the one
/// way this could take a click away from a player.
pub(super) fn float_of(placement: &Placement, held: bool, elapsed: f32) -> f32 {
    if !placement.flying {
        0.0
    } else if held {
        airborne::RESTING
    } else {
        airborne::height(elapsed, airborne::phase(placement.object))
    }
}

/// How far a card tucked under another (#305) peeks out past its host,
/// towards the middle of the table: the border and the name bar of a print,
/// which is what the owner asked to see of it. Under a tapped host it is
/// the card's long edge instead, turned with it, and the name is back when
/// the host untaps.
pub(super) const ATTACH_PEEK: f32 = CARD_HEIGHT * 0.11;
/// How far under its host a tucked card lies, and under the one tucked
/// before it: a fraction of a row's whole rise, and far above what the depth
/// buffer resolves at `CameraRig::MAX_DISTANCE`.
const ATTACH_DROP: f32 = LANE_RISE * 0.25;
/// How far under its host everything tucked under it lies, all together:
/// half the height a face stands over Defender's wall
/// ([`shellmat::WALL_HEIGHT`]), so the wall of the card beside it, which
/// stands on the felt where a tucked card peeks out, stays under every print.
const ATTACH_DEPTH: f32 = (shellmat::RIM_DROP - shellmat::WALL_HEIGHT) * 0.5;
const _: () = assert!(ATTACH_DROP <= ATTACH_DEPTH);

/// Lays the cards tucked under `host` (#305) down under it, each peeking out
/// past the one before towards the middle of the table, and says whether
/// the row folded them out of sight instead.
///
/// The room is measured and never assumed: from the host's front edge to the
/// first thing ahead of it — the band the seat's bar is written on, in front
/// of the creature row, and the next row in front of any other. Every card
/// peeks out a whole `ATTACH_PEEK` or none does: a sliver shows no name,
/// and it would lie where the host's mark stands. Where not all of them fit
/// whole, all lie flush under the host and it wears the mark instead
/// ([`cardplate::attached_word`]), which says how many, and its hover lays
/// them out beside the preview. Measured when this was written
/// (25.09.2026): an unstaged creature row shows four whole peeks under an
/// untapped host at every table, and a tapped host a whole one in every
/// row and never two (below); untapped, the support
/// and land rows show a whole one in a duel at 16:10 and 16:9, and fold at
/// 4:3 (0.053 of room) and at a ring (0.0093; its rows stand 0.0185 apart),
/// as does a staged creature there. A tapped host turns what is under it
/// with it (client-41, on #305): upright behind a tapped card, an aura would
/// stand 0.2 past it on both sides, into the air where the next row's plate
/// stands. And it keeps them inside the footprint the card has untapped,
/// which holds one peek: a second reached into the band over the top edge
/// where the next card's badge stands in a duel, and lay under it.
///
/// What a tucked card shows is its name and nothing written on it: no strip,
/// no plate and no shell, since each of those stands on or around a card and
/// this one lies under another's print. Its hover preview is the card.
#[allow(clippy::too_many_arguments)] // the host's placement, handed over as it was built
fn tuck(
    out: &mut Vec<Placement>,
    duel: &Duel,
    slot: &SeatSlot,
    lane: baylee_client_core::layout::LaneKind,
    host: &baylee_client_core::board::CardGroup,
    position: Vec2,
    lift: f32,
    tapped: bool,
    shown: bool,
) -> bool {
    if host.attached.is_empty() {
        return false;
    }
    let forward = slot.forward();
    let depth = if tapped { CARD_WIDTH } else { CARD_HEIGHT };
    let ahead = if lane == baylee_client_core::layout::LaneKind::Creatures {
        (slot.center + forward * (slot.half_extent.y - tabletop::MAT_LEDGE)).dot(forward)
    } else {
        slot.lane_center(lane).dot(forward) + slot.lane_height() * 0.5
    };
    let mut room = (ahead - position.dot(forward) - depth * 0.5).max(0.0);
    if tapped {
        // Inside the card's footprint untapped, over whose top edge the next
        // card's badge stands in a duel ([`BadgePlace::Above`]).
        room = room.min((CARD_HEIGHT - CARD_WIDTH) * 0.5);
    }
    let n = host.attached.len() as f32;
    let folded = room + 1e-4 < ATTACH_PEEK * n;
    let peek = if folded { 0.0 } else { ATTACH_PEEK };
    let drop = ATTACH_DROP.min(ATTACH_DEPTH / n);
    for (k, card) in (1_u16..).zip(&host.attached) {
        let k = f32::from(k);
        out.push(Placement {
            object: card.representative,
            slot: *slot,
            position: position + forward * (peek * k),
            lift: lift - drop * k,
            tapped,
            flying: false,
            indestructible: false,
            dome: None,
            defender: false,
            count: 1,
            badge: 0,
            art: card.art,
            offer: crate::cardmat::Offer::on(duel.proposing(), &card.members, card.activatable)
                .reaching(
                    card.members
                        .iter()
                        .all(|id| duel.ability_reach.contains(id)),
                ),
            corner: baylee_client_core::cardplate::Corner::default(),
            selected: duel
                .interaction
                .as_ref()
                .is_some_and(|i| card.members.iter().any(|member| i.is_selected(*member))),
            fan: None,
            marks: 0,
            sick: false,
            crests: [None; cardcrest::MAX_CRESTS],
            covered: true,
            // No plate: a tucked card says nothing of its own.
            room: cardplate::PlateRoom::OPEN,
            rung: drop,
            shown,
        });
    }
    folded
}

/// Computes placements for the whole table.
///
/// Pure geometry over the board model, so the ordering is the model's ordering
/// and therefore stable frame to frame — which is what makes the diff below
/// cheap and stops cards from swapping places when nothing happened.
#[allow(clippy::too_many_lines)] // one walk of the model, in the model's order
pub(super) fn placements(duel: &Duel) -> Vec<Placement> {
    let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.layout.as_ref()) else {
        return Vec::new();
    };
    // Which pile, if any, the pointer has spread open. Read once for the
    // whole table: at most one pile is ever open, because the pointer is over
    // at most one card.
    let fanned = board.fanned_pile(duel.hovered);
    // Who is in the fight. The same `Combat::read` the lines are drawn from,
    // so a card that has stepped out of its row and the line leaving it can
    // never disagree about whether the declaration exists — they are two
    // readings of one answer rather than two answers.
    let combat = duel
        .view
        .as_ref()
        .map(|view| Combat::read(view, duel.interaction.as_ref()));
    let mut out = Vec::new();
    for pod in &board.pods {
        let Some(slot) = layout.slot(pod.player) else {
            continue;
        };
        for lane in &pod.lanes {
            let center = slot.lane_center(lane.kind);
            // A merged card holds its cell whole, so its badge lies on no
            // neighbour; a row that cannot hold them and still fan legibly
            // shows a run of whole cards and scrolls (the owner, 25.09).
            let packing = lane.pack(slot);
            let window = packing.window(duel.rows.first((pod.player, lane.kind)));
            // The row's rise, shared out over however many cards are on it.
            let steps = lane.groups.len().saturating_sub(1).max(1) as f32;
            // A group is one card standing for several, and combat is
            // declared per creature — so the step is asked of the members
            // and not of the representative. It cannot normally differ: a
            // declared attacker, sent or only proposed, is taken out of its
            // group by the board model for exactly this reason
            // (`board::Proposal`). `any` rather than `all` because if that
            // ever stops being true, a fighting card stepping forward is the
            // better failure. Asked of the whole row first, because where
            // a plate may stand depends on its neighbours'.
            let staged: Vec<bool> = lane
                .groups
                .iter()
                .map(|group| {
                    combat
                        .as_ref()
                        .is_some_and(|c| group.members.iter().any(|m| c.staged(*m)))
                })
                .collect();
            let tapped: Vec<bool> = lane.groups.iter().map(|g| g.status.is_tapped()).collect();
            for (i, (group, offset)) in lane.groups.iter().zip(packing.offsets.iter()).enumerate() {
                let along = Vec2::new(slot.facing.cos(), -slot.facing.sin());
                let stage = if staged[i] { STAGE_STEP } else { 0.0 };
                let position = center + along * (*offset + window.shift) + slot.forward() * stage;
                // Later in the row is higher, so a fanned lane shingles the
                // way a hand of cards does — each card over the one before
                // it, and never in bands of both.
                let lift = LANE_RISE * i as f32 / steps;
                let turned = tapped[i];
                let shown = window.shown.contains(&i);
                let shells = shellmat::Shells::of(group);
                out.push(Placement {
                    object: group.representative,
                    slot: *slot,
                    position,
                    lift,
                    marks: cardrail::badge_bits(&group.badges),
                    sick: shells.wave,
                    crests: cardcrest::marks(group.provenance, group.commander),
                    // The last card shown has nothing laid over it, and
                    // nothing lies over a merged card's cell.
                    covered: packing.covered(i, &window),
                    room: packing.plate_room(i, &window, &tapped, &staged),
                    shown,
                    rung: LANE_RISE / steps,
                    tapped: turned,
                    // A group is one card standing for several and every
                    // member of it has the same keywords — `ObjectSummaryKey`
                    // carries them, so two Serra Angels of which one has lost
                    // flying are two groups.
                    flying: group.badges.contains(&KeywordBadge::Flying),
                    indestructible: shells.steel,
                    dome: shells.dome,
                    defender: shells.wall,
                    count: group.count(),
                    badge: cardplate::count_word(group.count()),
                    art: group.art,
                    // Resolved here rather than in the sync loop, because
                    // here is where the group's *members* are: a plan taps
                    // one particular Forest, and the card drawn for it may
                    // be standing for four.
                    offer: crate::cardmat::Offer::on(
                        duel.proposing(),
                        &group.members,
                        group.activatable,
                    )
                    .reaching(
                        group
                            .members
                            .iter()
                            .all(|id| duel.ability_reach.contains(id)),
                    ),
                    // Power, toughness, marked damage and the counters — the
                    // rules facts printed on every real card and drawn nowhere
                    // in this client on a card showing art. Resolved here for
                    // the same reason the offer is: the group is here.
                    corner: baylee_client_core::cardplate::Corner::of(group),
                    // Chosen for the pending choice — resolved here for the
                    // same reason again, and through `is_selected` rather
                    // than `selected()`. In the two combat modes the answer
                    // being built is a list of *pairs*, and `selected()` is
                    // empty however many attackers have been declared;
                    // `is_selected` is the method that reads the pairs. The
                    // sync loop used to ask the empty list, which is why a
                    // declared attacker lay flat on the table and combat
                    // drew nothing at all.
                    selected: duel
                        .interaction
                        .as_ref()
                        .is_some_and(|i| group.members.iter().any(|member| i.is_selected(*member))),
                    fan: None,
                });
                let host = out.len() - 1;
                if tuck(
                    &mut out, duel, slot, lane.kind, group, position, lift, turned, shown,
                ) {
                    out[host].badge = cardplate::attached_word(group.attached.len());
                }
            }
        }

        // The piles standing beside the ground. A pile whose top card is an
        // object is a placement like any other, and deliberately so:
        // `index.cards` is keyed by `ObjectId`, and an id survives a zone
        // change — so a creature that dies *glides* off its lane and onto the
        // graveyard through the update-in-place branch below, instead of
        // blinking out of one place and into another. It also inherits the
        // material cache, the hover lift, the arming glow and the selection
        // lift, every one of which would have to be written a second time in
        // a renderer of its own. "Nothing on the table is positioned
        // directly" is the rule; a pile is not an exception to it.
        //
        // A library has no object, and neither has an empty pile. Those two
        // are drawn by `sync_zones`, which needs no card behind them.
        for pile in &pod.piles {
            // The hover fan. It replaces the pile's own top card rather than
            // standing beside it — the top card *is* the first card of the
            // fan, and drawing both would put one card in two places and give
            // `index.cards` two entities for one id.
            //
            // The pile's thickness goes on the **lowest** card of the fan,
            // which is the one still standing over the pile's own place: it
            // is the rest of the cards, and carrying it up with the top card
            // instead would lift the whole deck into the air with it.
            if fanned == Some((pod.player, pile.kind)) && !pile.fan.is_empty() {
                let len = pile.fan.len();
                let under = usize::try_from(pile.count).unwrap_or(usize::MAX);
                for (i, card) in pile.fan.iter().enumerate() {
                    let pose = slot.fan_pose(pile.kind, i, len, duel.hovered == Some(card.object));
                    out.push(Placement {
                        object: card.object,
                        slot: *slot,
                        position: pose.at,
                        lift: pose.lift,
                        tapped: false,
                        // A card the pointer has lifted out of a graveyard is
                        // not a permanent and has no keywords to draw, the
                        // same reason its corner is the empty one.
                        flying: false,
                        indestructible: false,
                        dome: None,
                        defender: false,
                        count: if i + 1 == len {
                            under.saturating_sub(len - 1).max(1)
                        } else {
                            1
                        },
                        badge: 0,
                        art: card.art,
                        offer: pile_offer(duel, card.object),
                        corner: baylee_client_core::cardplate::Corner::default(),
                        selected: duel
                            .interaction
                            .as_ref()
                            .is_some_and(|i| i.is_selected(card.object)),
                        fan: Some((pose, pile.kind)),
                        marks: 0,
                        sick: false,
                        crests: [None; cardcrest::MAX_CRESTS],
                        covered: false,
                        room: cardplate::PlateRoom::OPEN,
                        shown: true,
                        rung: 0.0,
                    });
                }
                continue;
            }

            let Some(top) = pile.top else {
                continue;
            };
            out.push(Placement {
                object: top,
                slot: *slot,
                position: slot.pile_center(pile.kind),
                // A pile stands beside the ground and overlaps nothing, so
                // there is nothing here for the row's rise to separate it
                // from.
                lift: 0.0,
                // A card in a graveyard is not a permanent and has no tap
                // state to draw; the same goes for its power and toughness,
                // which is why the corner is the empty one rather than
                // `Corner::of`. Drawing a 4/4 on a card that is no longer a
                // creature would be inventing a fact.
                tapped: false,
                flying: false,
                indestructible: false,
                dome: None,
                defender: false,
                count: usize::try_from(pile.count).unwrap_or(usize::MAX),
                badge: 0,
                art: pile.art,
                offer: pile_offer(duel, top),
                corner: baylee_client_core::cardplate::Corner::default(),
                selected: duel
                    .interaction
                    .as_ref()
                    .is_some_and(|i| i.is_selected(top)),
                fan: None,
                marks: 0,
                sick: false,
                crests: [None; cardcrest::MAX_CRESTS],
                covered: false,
                room: cardplate::PlateRoom::OPEN,
                shown: true,
                rung: 0.0,
            });
        }
    }
    // A parked seat's cards (DESIGN-v8 §0) stand where its slot is and are
    // not drawn: hidden where they wait, as a scrolled row's are, so a seat
    // brought back onto the felt glides in rather than being spawned.
    for placement in &mut out {
        if placement.slot.parked
            && !duel
                .tear
                .as_ref()
                .is_some_and(|t| t.draws(placement.slot.player))
        {
            placement.shown = false;
        }
        // A board riding a turning piece of the table rides over it.
        if let Some(tear) = duel.tear.as_ref() {
            placement.lift += tear.lift(placement.slot.player);
        }
    }
    out
}

/// The offer drawn on a card lying in a pile, or lifted out of one by a
/// hover: [`crate::Duel::reach_of`], in the two lights it answers with.
///
/// Until #242 both pile sites passed `false` here, so nothing in a pile was
/// ever lit — not the Opt Snapcaster Mage had just made castable, and not a
/// commander standing in the command zone with the lands to pay for it.
fn pile_offer(duel: &Duel, object: ObjectId) -> crate::cardmat::Offer {
    let reach = duel.reach_of(object);
    crate::cardmat::Offer::on(
        duel.proposing(),
        &[object],
        reach == Some(crate::Reach::Offered),
    )
    .reaching(reach == Some(crate::Reach::Taps))
}
