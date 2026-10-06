//! Library fans and the stacked slabs under piles and merged groups.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

/// The backs a hover spreads out of a library.
///
/// A system of its own, and it is the one place the fan is not a placement.
/// Every other pile fans *objects* — cards with an `ObjectId`, which
/// `sync_scene` already knows how to cache, glide, light and pick — and a
/// library has none: it is face down to everybody, its owner included
/// (CR 401.2), so `PlayerView` carries it as a count and `ZonePile::fan` is
/// empty there by construction. What is drawn here is therefore blank slabs,
/// as many as [`ZonePile::fan_len`] says, wearing the one material every
/// hidden card in the game wears.
///
/// It is the same *shape* as the fan beside it — the same poses from the same
/// [`SeatSlot::fan_pose`] — which is the point: a library that answered a
/// hover differently from a graveyard would teach a player that some piles
/// are worth pointing at and leave them guessing which. What it says is the
/// count, up to seven, which is the one thing the pile's own thickness cannot
/// say once it has reached its cap.
pub fn sync_library_fan(mut commands: Commands, duel: Res<Duel>, mut index: ResMut<SceneIndex>) {
    let wanted = duel
        .hovered_pile
        .filter(|(_, kind)| *kind == baylee_client_core::PileKind::Library)
        .and_then(|(player, kind)| {
            let board = duel.board.as_ref()?;
            let slot = duel.layout.as_ref()?.slot(player)?;
            let pile = board
                .pod(player)?
                .piles
                .iter()
                .find(|pile| pile.kind == kind)?;
            (pile.fan_len() > 0).then(|| (player, *slot, pile.fan_len()))
        });

    // Nothing to draw, or a different pile than the one standing open: the
    // backs go home the way a fanned card does, on the pile-bound exit with
    // no door on it. `retire` despawns them once the glide has had its time.
    if index.library_fan != wanted.map(|(player, _, _)| player) {
        for entity in std::mem::take(&mut index.library_fan_cards) {
            if let Ok(mut card) = commands.get_entity(entity) {
                card.remove::<PileVisual>()
                    .insert((Pickable::IGNORE, Departing { left: EXIT_LIFE }));
            }
        }
        index.library_fan = None;
    }

    let Some((player, slot, len)) = wanted else {
        return;
    };
    let (Some(quad), Some(blank)) = (index.quad.clone(), index.blank.clone()) else {
        return;
    };

    // Born on the pile and gliding out of it, so the fan grows rather than
    // appearing — `glide` does the moving, as it does for everything else on
    // this table, and a player who has turned motion off gets the pose on the
    // first frame.
    if index.library_fan.is_none() {
        let home = card_transform(
            &slot,
            slot.pile_center(baylee_client_core::PileKind::Library),
            false,
            stack_rise(len),
        );
        index.library_fan_cards = (0..len)
            .map(|i| {
                let pose = slot.fan_pose(baylee_client_core::PileKind::Library, i, len, false);
                let mut at = card_transform(&slot, pose.at, false, pose.lift);
                at.rotation = fan_rotation(&slot, pose);
                commands
                    .spawn((
                        DuelStage,
                        Mesh3d(quad.clone()),
                        MeshMaterial3d(blank.clone()),
                        home,
                        Motion { target: at },
                        // Pickable, and wearing the pile it came out of: once
                        // the fan is open the pointer is on a back and no
                        // longer on the library, and a fan that only the
                        // library itself held open would shut on the first
                        // pixel of travel.
                        PileVisual {
                            player,
                            kind: baylee_client_core::PileKind::Library,
                        },
                    ))
                    .id()
            })
            .collect();
        index.library_fan = Some(player);
    }
}

/// One slab of the deck under a card: depth, not a card. A marker, so the
/// slabs can be found and counted without being taken for the card.
#[derive(Component)]
pub struct StackSlab;

/// What stands under one card: see [`SceneIndex::stacks`].
#[derive(Default)]
pub(super) struct Stack {
    /// The count the deck was built for.
    count: usize,
    /// Whether the pile was laid tapped: a merged pile's cards step out to
    /// its seat's left, which a tapped card's own space names differently.
    tapped: bool,
    /// The slabs, top first.
    slabs: Vec<Entity>,
    /// The contact shadow under the whole deck.
    shadow: Option<Entity>,
}

/// Where the `i`-th of `layers` cards under a merged pile hangs, the pile
/// standing `deck` high (#263): that share of the deck down, and a further
/// [`PILE_JOG`] out to the left of the seat for each, whether or not the
/// pile is tapped (the owner, 25.09). A tapped card is turned a quarter
/// about its face, so its seat's left is its own −y, and the offset is laid
/// there.
fn pile_slab_transform(i: usize, layers: usize, deck: f32, tapped: bool) -> Transform {
    #[allow(clippy::cast_precision_loss)] // at most `PILE_SLABS`
    let (step, share) = (i as f32, i as f32 / layers as f32);
    let out = -step * PILE_JOG * CARD_WIDTH;
    let (x, y) = if tapped { (0.0, out) } else { (out, 0.0) };
    Transform::from_xyz(x, y, -deck * share)
}

/// The colour of the `i`-th card under a merged pile: the back and
/// [`SLAB_EDGE_COLOR`] in turn, so each edge of the staircase stands out
/// from the one above it.
fn pile_slab_color(i: usize) -> Color {
    if i % 2 == 1 {
        SLAB_EDGE_COLOR
    } else {
        BACK_COLOR
    }
}

/// Where the `i`-th of `layers` slabs hangs under a zone pile standing
/// `deck` high, in the card's own space: that share of the deck down, to the right
/// for an odd slab and to the left for an even one, and further out the
/// deeper it lies — from half of [`PILE_JOG`] towards all of it — so every
/// slab's edge shows past the one above it on its side, not only the first.
fn slab_transform(i: usize, layers: usize, deck: f32) -> Transform {
    let side = if i % 2 == 1 { 1.0 } else { -1.0 };
    let share = i as f32 / layers as f32;
    Transform::from_xyz(
        side * PILE_JOG * CARD_WIDTH * (0.5 + 0.5 * share),
        0.0,
        -deck * share,
    )
}

/// Builds what stands under a card — the slabs of its deck and its contact
/// shadow — and rebuilds it when the count changes (#261).
///
/// A pile stands on the cards under it: the top card is drawn at the deck's
/// own height and the rest hangs below it as children, so what a player sees
/// is one block of cardboard with a face on top. The slabs are cards with no
/// print, jogged ([`PILE_JOG`]) so their edges show, in two colours in turn
/// ([`pile_slab_color`], [`slab_color`]) so the layers do: a merged pile's
/// to the left, at most [`PILE_SLABS`] of them, a zone pile's to either
/// side. Children and not loose
/// entities, for the strip's reasons and one more: as loose entities they
/// were never despawned at all, and every card that ever lay on a graveyard
/// left its slabs standing there for the rest of the game.
///
/// The shadow is rebuilt only with the count, and rebuilt rather than moved:
/// [`ground_the_shadows`] remembers a flier's resting shadow by entity, and a
/// moved one would be put back where the smaller deck had it when the card
/// lands. It sits under the whole deck and wider the taller the deck is — a
/// thick pile sits in more shadow than a single card does, which is most of
/// what makes it read as thick at all.
pub(super) fn sync_stack(
    commands: &mut Commands,
    index: &mut SceneIndex,
    materials: &mut Assets<CardMaterial>,
    card: Entity,
    placement: &Placement,
    motion: f32,
) {
    // A merged card on the battlefield, which the badge is only ever on.
    let pile = placement.badge > 0;
    let current = index.stacks.get(&placement.object);
    if current.is_some_and(|stack| {
        stack.count == placement.count && (!pile || stack.tapped == placement.tapped)
    }) {
        return;
    }
    let Some(quad) = index.quad.clone() else {
        return;
    };
    let recount = current.is_none_or(|stack| stack.count != placement.count);
    let mut stack = index.stacks.remove(&placement.object).unwrap_or_default();
    for slab in stack.slabs.drain(..) {
        commands.entity(slab).despawn();
    }
    let under = placement.count.saturating_sub(1);
    // A pile shows at most `PILE_SLABS` cards under it and stands as high
    // as they do: the badge says how many there are.
    let layers = if pile {
        under.min(PILE_SLABS)
    } else {
        stack_layers(under)
    };
    let deck = stack_rise(if pile { layers } else { under });
    if recount {
        if let Some(shadow) = stack.shadow.take() {
            commands.entity(shadow).despawn();
        }
        if let Some((mesh, material)) = index.shadow_quad.clone().zip(index.shadow_material.clone())
        {
            let shadow = commands
                .spawn((
                    CardShadow,
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0.0, 0.0, -(CARD_LIFT * 0.5 + deck)).with_scale(Vec3::new(
                        1.0 + deck * DECK_SHADOW_SPREAD,
                        1.0 + deck * DECK_SHADOW_SPREAD,
                        1.0,
                    )),
                    // Between the felt and the card, and a click near a
                    // card's edge means the table.
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(card).add_child(shadow);
            stack.shadow = Some(shadow);
        }
    }
    for i in 1..=layers {
        // Nothing sees more of a slab than its jog and its wall: the card on
        // top covers the rest.
        let color = if pile {
            pile_slab_color(i)
        } else {
            slab_color(i)
        };
        let look = CardLook::flat(color, FinishTreatment::Plain);
        let material = index
            .face_materials
            .entry(look)
            .or_insert_with(|| materials.add(material(look, None, color, motion)))
            .clone();
        let slab = commands
            .spawn((
                StackSlab,
                Mesh3d(quad.clone()),
                MeshMaterial3d(material),
                if pile {
                    pile_slab_transform(i, layers, deck, placement.tapped)
                } else {
                    slab_transform(i, layers, deck)
                },
                // The deck under a card is depth, not cards: the top card is
                // what a click has to reach.
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(card).add_child(slab);
        stack.slabs.push(slab);
    }
    stack.count = placement.count;
    stack.tapped = placement.tapped;
    index.stacks.insert(placement.object, stack);
}
