//! The scene diff: `sync_scene` against the board model.

#[allow(clippy::wildcard_imports)] // the table's shared vocabulary
use super::*;

mod cache;
use cache::{CardInput, DecorationReady, SceneWork};

/// Whether the card under the pointer on the felt has an offer on it (the
/// light the table draws round a card this client can act on): the pointer
/// is a hand over it. Written only when it changes.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct HoveredOffer(pub bool);

/// Brings the scene in line with the board model.
#[allow(clippy::too_many_arguments, clippy::type_complexity)] // a system's injections, grouped under bevy's ceiling
#[allow(clippy::too_many_lines)] // the diff loop is one coherent pass
pub fn sync_scene(
    mut commands: Commands,
    time: Res<Time>,
    duel: Res<Duel>,
    mut index: ResMut<SceneIndex>,
    mut watch: ResMut<ZoneWatch>,
    mut textures: Option<ResMut<CardTextures>>,
    mut card_materials: ResMut<Assets<CardMaterial>>,
    // One parameter for the objects lying on and under a card: a system
    // takes sixteen.
    (
        mut strip_materials,
        mut badge_materials,
        mut plate_materials,
        mut floor_materials,
        mut shell_materials,
    ): CardCompanions<'_>,
    assets: Res<AssetServer>,
    texts: Res<crate::cardtext::CardTexts>,
    // How the player asked to see the table, and where from: the plate and
    // the badge read upright to the camera as it stands this frame.
    (mode, settings, prefs, shown, mut offered): (
        Res<crate::face::FaceMode>,
        Res<crate::settings::ClientSettings>,
        Res<crate::prefs::Prefs>,
        Res<ShownRig>,
        Option<ResMut<crate::table::HoveredOffer>>,
    ),
    sheen: Res<crate::sheen::Sheen>,
    // The fonts, and what has arrived of them: a face is measured in one.
    (fonts, font_assets): (Option<Res<crate::hud::UiFonts>>, Option<Res<Assets<Font>>>),
    mut cards: Query<DrawnCard>,
    mut work: Local<SceneWork>,
) {
    let Some(statics) = duel.statics.as_ref() else {
        return;
    };
    if textures.is_none() {
        return;
    }
    let Some(quad) = index.quad.clone() else {
        return;
    };
    let blank = index.blank.clone();
    let eye = shown.rig().unwrap_or_default().eye();
    // Read after the guards and not before them: a frame that bails because
    // the quad or the print table has not arrived yet must not swallow the
    // one batch of moves this view will ever produce. `sheen` may have read
    // the view already this frame — it needs the same batch to know which
    // door an arriving card came in through — so the reading and the taking
    // are two calls, and this is the only place that takes.
    if let Some(view) = duel.view.as_ref()
        && !watch.has_observed(view)
    {
        watch.observe(view);
    }
    let moves = if watch.undrawn().is_empty() {
        Vec::new()
    } else {
        watch.take()
    };

    // A look carrying a sweep is a key that is asked for on every frame the
    // band is crossing and never again, so it is cached like any other look —
    // otherwise a card arriving would mint a material and a handle sixty
    // times a second — and swept out here once the band is over. Without
    // this the map would gain an entry per permanent per arrival and evict
    // none of them.
    //
    // A look on its way into or out of a passing state ([`Veil`]) is the
    // same kind of key: one per moment a state changed, wanted until it has
    // arrived and the card wears the settled look every card in that state
    // shares.
    let now = time.elapsed_secs_wrapped();
    let arriving = |veil: crate::cardmat::Veil| {
        veil.rate == 0 || (now - f32::from_bits(veil.at)) * f32::from_bits(veil.rate) < 1.0
    };
    let keep_look =
        |look: &CardLook| look.sweep.is_none_or(|s| sheen.live(s)) && arriving(look.veil);
    if index.materials.keys().any(|look| !keep_look(look)) {
        index.materials.retain(|look, _| keep_look(look));
    }
    if index.face_materials.keys().any(|look| !arriving(look.veil)) {
        index.face_materials.retain(|look, _| arriving(look.veil));
    }

    // One original sleeve for the library, its fan and every hidden card.
    // Dress the resident material in place: the library's slabs spawn once
    // and are never visited again. Downloaded printing art cannot replace it.
    if !index.back_dressed
        && let Some(handle) = blank.as_ref()
        && let Some(mut material) = card_materials.get_mut(handle)
        && let Some(textures) = textures.as_deref()
    {
        dress_in_the_back(&mut material, textures.procedural_back());
        index.back_dressed = true;
    }

    // Reduce-motion reaches the cards through their material, so a change to
    // it has to reach every material already made. Compare this setting:
    // unrelated preferences do not change a card's material. The save timer
    // itself already bypasses preference change detection.
    //
    // Rewritten in place rather than thrown away, which is the difference
    // between the switch taking effect and the switch taking effect *later*:
    // a cleared cache is only refilled by whatever draws the card next, and
    // a table nobody is playing at draws nothing. Rewriting keeps every
    // handle valid, so the cards already on the felt change under the
    // player's eyes on the frame the setting moves.
    let still = prefs.all().reduce_motion;
    let motion = motion_of(still);
    if index.still != still {
        index.still = still;
        let handles: Vec<_> = index
            .materials
            .values()
            .chain(index.face_materials.values())
            .cloned()
            .collect();
        for handle in handles {
            if let Some(mut material) = card_materials.get_mut(&handle) {
                material.params.motion = motion;
            }
        }
        for handle in index.marks_materials.values() {
            if let Some(mut material) = strip_materials.get_mut(handle) {
                material.params.motion = motion;
            }
        }
        for handle in index.floor_materials.values() {
            if let Some(mut material) = floor_materials.get_mut(handle) {
                material.params.motion = motion;
            }
        }
        for handle in index.shell_materials.values() {
            if let Some(mut material) = shell_materials.get_mut(handle) {
                material.params.motion = motion;
            }
        }
    }

    let SceneWork {
        wanted,
        live,
        hidden,
        fanned,
        stamps,
        stale,
        visible,
        last_visible,
    } = &mut *work;
    placements_into(&duel, wanted);
    live.clear();
    hidden.clear();
    hidden.extend(wanted.iter().filter(|p| !p.shown).map(|p| p.object));
    // What is standing in a fan this frame, and which pile it came out of.
    // Rebuilt every frame rather than kept, because a fan is open for exactly
    // as long as the pointer is on it and the answer is never carried over.
    fanned.clear();

    // The keyboard/mouse cursor. What is *chosen* rides on the placement,
    // because that is where a group's members are.
    let hovered = duel.hovered.map(|id| {
        duel.board
            .as_ref()
            .map_or(id, |board| board.drawn_object(id))
    });

    // The snapshot the faces below were built from: rules text is projected,
    // so a face is only stale when the game state that produced it moved on.
    let seq = duel.view.as_ref().map_or(0, |v| v.seq);
    // What the faces below are measured with: the font they are set in, once
    // it has arrived.
    let widths = face::Widths::of(
        fonts
            .as_deref()
            .zip(font_assets.as_deref())
            .and_then(|(fonts, assets)| assets.get(&fonts.text)),
    );

    let objects = duel
        .view
        .as_ref()
        .map(baylee_client_core::board::Lookup::new);
    let fonts_changed = fonts.as_ref().is_some_and(DetectChanges::is_changed)
        || font_assets.as_ref().is_some_and(DetectChanges::is_changed);
    let decorations = DecorationReady::of(&index);
    let mut hovered_offer = false;
    // The lane model keeps every object for targeting and keyboard navigation.
    // The scene owns entities only for its visible scroll window.
    for placement in wanted.iter().filter(|p| p.shown) {
        live.insert(placement.object);

        // A card either wears its art or its own text, never both — text on
        // top of artwork is unreadable at any zoom.
        let Some(textures_ref) = textures.as_deref() else {
            return;
        };
        let show_face = face::wants_face(&mode, &settings, textures_ref, placement.art);
        let object = objects
            .as_ref()
            .and_then(|view| view.object(placement.object));

        // What the card is physically: the finish is a property of the
        // printing, so it comes from the print table — which is per seat, and
        // a printing this seat has not earned reads as plain rather than as
        // a leak.
        let finish = crate::cardmat::finish_of(statics, placement.art);
        // The offer is what the player could do with the card — or has just
        // said they will — and is light on the felt round it; a Forest that
        // becomes tappable is lit, and stops being lit the moment priority
        // moves on.
        let glow =
            crate::cardmat::glow_of_with_selection(object, placement.offer, placement.selected);
        if hovered == Some(placement.object) {
            hovered_offer = glow & crate::cardmat::glow::OFFERS != 0;
        }

        // The card's passing states: dimmed while this seat's question leaves
        // it out, phased out. Followed per card, so a change fades in rather
        // than jumps; the clock is the one `globals.time` is written from.
        // A card drawn before in no state has no entry; one drawn for the
        // first time is already in its states, since nothing on screen
        // changed.
        let track = match index.veils.get(&placement.object) {
            Some(track) => track.toward(placement.veil, now, !still),
            None if index.cards.contains_key(&placement.object) => {
                crate::cardmat::VeilTrack::new(0).toward(placement.veil, now, !still)
            }
            None => crate::cardmat::VeilTrack::new(placement.veil),
        };
        let veil = track.veil(now);
        // A card in no state, fades over, needs no entry.
        if veil == crate::cardmat::Veil::NONE {
            if index.veils.contains_key(&placement.object) {
                index.veils.remove(&placement.object);
            }
        } else if index.veils.get(&placement.object) != Some(&track) {
            index.veils.insert(placement.object, track);
        }

        if let Some((_, kind)) = placement.fan
            && let Some(place) = place_of(kind, placement.slot.player)
        {
            fanned.insert(placement.object, place);
        }
        let input = CardInput {
            decorations,
            placement: *placement,
            eye,
            still,
            finish,
            glow,
            veil,
            sweep: sheen.of(placement.object, crate::sheen::Surface::Table),
            show_face,
            texture_epoch: textures.as_deref().map_or(0, CardTextures::epoch),
            text_epoch: texts.generation(),
            seq: if show_face { seq } else { 0 },
        };
        let before = stamps.get(&placement.object);
        let refit = before.is_some_and(|old| old.face_changed(&input, object));
        if !fonts_changed
            && let Some(&entity) = index.cards.get(&placement.object)
            && before.is_some_and(|old| old.matches(entity, &input, object))
            && let Ok((mut target, _, _, _, mut rest, _)) = cards.get_mut(entity)
        {
            // Flying continues through the regular Motion target even while
            // its picture, text and decorations remain unchanged.
            let (transform, resting) = pose_of(placement, hovered, still, now);
            if target.target != transform {
                target.target = transform;
            }
            if rest.0 != resting {
                rest.0 = resting;
            }
            continue;
        }

        // The face is fitted before the material is chosen: its name's lines
        // are the name bar's depth, which the material draws (#259), so the
        // two come out of one fitting. A face fitted before the font arrived
        // is fitted again when it does — the average's widths are a
        // stand-in, and the font's may put the same name on the other number
        // of lines.
        let face_now = if show_face {
            face_now(
                index.faces.get(&placement.object).filter(|_| !refit),
                seq,
                &widths,
                || object.map(|object| face::of_object(object, None, &texts)),
            )
        } else {
            FaceNow::Unbuilt
        };

        let material = if show_face {
            // One material per colour identity and name depth, so a
            // mono-green board is one material however many creatures are on
            // it.
            let look = face_look(object, face_now.lines(), finish).with_veil(veil);
            if let Some(handle) = index.face_materials.get(&look) {
                handle.clone()
            } else {
                let tint = face::table_color(object.map_or(ColorSet::EMPTY, |o| o.colors));
                let handle = card_materials.add(material(look, None, tint, motion));
                index.face_materials.insert(look, handle.clone());
                handle
            }
        } else {
            // One material per look, created on first use.
            match placement.art {
                Some(key) => {
                    let look = CardLook::art(key, finish)
                        .with_sweep(sheen.of(placement.object, crate::sheen::Surface::Table))
                        .with_veil(veil);
                    if let Some(handle) = index.materials.get(&look) {
                        handle.clone()
                    } else {
                        let Some(textures) = textures.as_deref_mut() else {
                            return;
                        };
                        let image = textures.get(key, statics, &assets);
                        let handle =
                            card_materials.add(material(look, Some(image), BACK_COLOR, motion));
                        index.materials.insert(look, handle.clone());
                        handle
                    }
                }
                None => blank.clone().unwrap_or_default(),
            }
        };

        let (transform, resting) = pose_of(placement, hovered, still, now);

        let entity = if let Some(&entity) = index.cards.get(&placement.object) {
            // Existing card: update in place. Touching only what changed is
            // what keeps a large board cheap.
            if let Ok((
                mut motion,
                mut visual,
                mut current_material,
                airborne,
                mut rest,
                mut seen,
            )) = cards.get_mut(entity)
            {
                // A card a scrolled row does not show is not drawn, and
                // everything lying on it or under it goes with it.
                seen.set_if_neq(shown_as(placement.shown));
                if motion.target != transform {
                    motion.target = transform;
                }
                if rest.0 != resting {
                    rest.0 = resting;
                }
                if visual.count != placement.count {
                    visual.count = placement.count;
                }
                if current_material.0 != material {
                    current_material.0 = material;
                }
                // A creature can gain flying and lose it again — an anthem
                // resolving, an aura leaving — so this is a diff like the
                // others and not a property of the entity. Compared first:
                // an unconditional insert every frame would be an archetype
                // move every frame for every flier on the table.
                if airborne != placement.flying {
                    if placement.flying {
                        commands.entity(entity).insert(Floating);
                    } else {
                        commands.entity(entity).remove::<Floating>();
                    }
                }
            }
            entity
        } else {
            let entity = commands
                .spawn((
                    DuelStage,
                    CardVisual {
                        object: placement.object,
                        count: placement.count,
                    },
                    Mesh3d(quad.clone()),
                    MeshMaterial3d(material),
                    // Appears wherever it is coming from and settles onto its
                    // mark; `glide` does the rest, and a player who has turned
                    // motion off gets the target on the very first frame.
                    if placement.fan.is_some() || moves.iter().any(|m| m.object == placement.object)
                    {
                        entrance_from(
                            pile_stand(
                                &duel,
                                placement
                                    .fan
                                    .and_then(|(_, kind)| place_of(kind, placement.slot.player))
                                    .or_else(|| {
                                        moves
                                            .iter()
                                            .find(|m| m.object == placement.object)
                                            .and_then(|m| m.from)
                                    }),
                            ),
                            &transform,
                        )
                    } else {
                        // Scrolling reveals a card already on the table; it
                        // must not replay a battlefield entrance.
                        transform
                    },
                    Motion { target: transform },
                    CardRest(resting),
                    shown_as(placement.shown),
                ))
                .id();
            index.cards.insert(placement.object, entity);
            if placement.flying {
                commands.entity(entity).insert(Floating);
            }

            entity
        };

        // What the strip and the plate say. The plate shows only where the
        // print cannot say it (`Corner::shows_plate`): a card showing its
        // text face has no printed box, and one showing its art has, unless
        // the next card of the row lies over it. The chip goes with it.
        let print = !show_face && placement.art.is_some();
        let corner = placement.corner;
        let plated = corner.shows_plate(print, placement.covered);
        let strip =
            cardrail::Strip::new(placement.marks, plated.then_some(corner), placement.crests);
        sync_strip(
            &mut commands,
            &mut index,
            &mut strip_materials,
            entity,
            (placement.object, strip, placement.rung),
            motion,
        );
        sync_badge(
            &mut commands,
            &mut index,
            &mut badge_materials,
            entity,
            placement,
            &eye,
        );
        sync_plate(
            &mut commands,
            &mut index,
            &mut plate_materials,
            entity,
            placement,
            plated.then(|| PlateWords::of(corner, placement.sick)),
            &eye,
        );
        sync_floor(
            &mut commands,
            &mut index,
            &mut floor_materials,
            entity,
            placement,
            glow,
            motion,
        );
        sync_stack(
            &mut commands,
            &mut index,
            &mut card_materials,
            entity,
            placement,
            motion,
        );
        sync_shell(
            &mut commands,
            &mut index,
            &mut shell_materials,
            entity,
            placement,
            motion,
        );

        stamps
            .entry(placement.object)
            .and_modify(|stamp| stamp.update(entity, &input, object))
            .or_insert_with(|| cache::CardStamp::new(entity, &input, object));

        // The text children follow the same decision as the material, and are
        // rebuilt when the snapshot they were made from is no longer current:
        // an anthem, a counter or a clone all change what the face should say.
        if let FaceNow::Kept(_) = face_now {
            continue;
        }
        if let Some(previous) = index.faces.remove(&placement.object) {
            for text in previous.texts {
                commands.entity(text).despawn();
            }
        }
        let (FaceNow::Fitted(fitted), Some(fonts)) = (face_now, fonts.as_deref()) else {
            continue;
        };
        let (built, fit) = *fitted;
        let spawned = face::spawn_world(
            &mut commands,
            entity,
            &built,
            &fit,
            face_word(object, fit.lines()),
            placement.corner.plate,
            fonts,
        );
        index.faces.insert(
            placement.object,
            ShownFace {
                seq,
                measured: fit.measured,
                lines: fit.lines(),
                texts: spawned,
            },
        );
    }
    // The pointer is a hand over a card this client offers something for
    // (the pointer's shape); written only when that changes.
    if let Some(offered) = offered.as_mut() {
        offered.set_if_neq(crate::table::HoveredOffer(hovered_offer));
    }

    stamps.retain(|id, _| live.contains(id));

    // Anything no longer on the board leaves the scene.

    stale.clear();
    stale.extend(index.cards.keys().copied().filter(|id| !live.contains(id)));
    for &id in &*stale {
        if let Some(entity) = index.cards.remove(&id) {
            // Despawning a card takes its text children with it, so the map
            // only has to forget them — and it keeps them for as long as the
            // card is still leaving, which is what makes a named card sink
            // into the graveyard rather than a blank one. The strip, the
            // badge and the light on the felt are children in the same way.
            index.faces.remove(&id);
            index.veils.remove(&id);
            index.marks.remove(&id);
            index.badges.remove(&id);
            index.plates.remove(&id);
            index.floors.remove(&id);
            index.stacks.remove(&id);
            // Not the shell: a keyword's border means nothing off the
            // battlefield, and one flying off with its card would no
            // longer be fitted to anything on the way.
            if let Some(shell) = index.shells.remove(&id) {
                shell.despawn(&mut commands);
            }
            if hidden.contains(&id) {
                // Scrolling is not a zone change. Remove the hidden entity and
                // its children immediately; the full board still owns the card.
                commands.entity(entity).despawn();
                continue;
            }
            // A stale id with no move behind it did not leave anywhere: it is
            // a graveyard's old top card, covered by the one that landed on
            // it this frame, or a group that re-keyed when its lowest-id
            // member went. Nothing about the table changed where it stands,
            // so it goes at once, as it always did — an exit played for one
            // of those would be a card visibly sliding out from under a pile
            // it never left.
            let Some(step) = moves.iter().find(|m| m.object == id).copied() else {
                // Unless it is a card the fan had in the air a frame ago. It
                // has not left anywhere — the pointer left *it* — so it takes
                // the pile-bound exit with no door on it, which is the glide
                // that puts it back under the pile's top card. A card that
                // moved zones on the same frame never reaches here: the move
                // below is the truer answer and takes precedence.
                if let Some(&home) = index.fanned.get(&id) {
                    if let Ok((mut motion, ..)) = cards.get_mut(entity) {
                        motion.target =
                            exit(Some(home), pile_stand(&duel, Some(home)), &motion.target);
                    }
                    commands
                        .entity(entity)
                        .remove::<CardVisual>()
                        .insert((Pickable::IGNORE, Departing { left: EXIT_LIFE }));
                    continue;
                }
                commands.entity(entity).despawn();
                continue;
            };
            let to = step.to;
            if let Ok((mut motion, _, mut worn, ..)) = cards.get_mut(entity) {
                motion.target = exit(to, pile_stand(&duel, to), &motion.target);
                if let Some(dressed) = dress_the_exit(
                    &mut card_materials,
                    &worn.0,
                    step,
                    sheen.now(),
                    motion_of(still),
                ) {
                    worn.0 = dressed;
                }
            }
            // It stops being a card here. `CardVisual` is what every reader
            // finds a permanent by, so taking it away is what stops a hover,
            // a preview or a combat line from following something that is on
            // its way out of the game.
            commands
                .entity(entity)
                .remove::<CardVisual>()
                .insert((Pickable::IGNORE, Departing { left: EXIT_LIFE }));
        }
    }

    // After the stale pass and not before it: what this frame fanned is next
    // frame's answer to "was that card in the air".
    if !index.fanned.is_empty() || !fanned.is_empty() {
        std::mem::swap(&mut index.fanned, fanned);
    }

    // Tell the cache what is on screen so the next fetch evicts something
    // else. The placements are the table's own cards; `required_images` is
    // the board model's whole answer — the hand, the stack and what it points
    // at, a pile's top card and the fan a hover spreads out of it, the
    // cardboard under a copy — and none of those is on the table, so none of
    // them was being touched at all. The zone dialog's rows are the one thing
    // neither list holds, and `hud::tray` touches those itself.
    if let Some(board) = duel.board.as_ref() {
        board.required_images_into(visible);
    } else {
        visible.clear();
    }
    visible.extend(wanted.iter().filter(|p| p.shown).filter_map(|p| p.art));
    visible.sort();
    visible.dedup();
    if *visible != *last_visible {
        last_visible.clone_from(visible);
        if let Some(textures) = textures.as_deref_mut() {
            textures.touch_visible(visible);
        }
    }
}

/// Keeps animated poses independent of changes to the card's drawn content.
fn pose_of(
    placement: &Placement,
    hovered: Option<ObjectId>,
    still: bool,
    now: f32,
) -> (Transform, Transform) {
    // A pile stands on the cards under it. The top card is drawn at the
    // deck's own height and the rest of the deck hangs below it as
    // children, so what a player sees is one block of cardboard with a
    // face on top rather than a card with a fan of cards behind it.
    let deck = stack_rise(placement.count.saturating_sub(1));
    // A creature with flying stands off the felt. It goes in *here*, with
    // the row's rise and the deck under it, so it reaches the card
    // through `Motion` and `glide` like every other reason a card is
    // where it is — an offset added to the transform after the glide
    // would be fought by the glide on the next frame and compound.
    //
    let float = float_of(
        placement,
        placement.selected || placement.offer.armed || hovered == Some(placement.object) || still,
        now,
    );
    let mut transform = card_transform(
        &placement.slot,
        placement.position,
        placement.tapped,
        placement.lift + deck + float,
    );
    if placement.flying
        && !still
        && !placement.selected
        && !placement.offer.armed
        && hovered != Some(placement.object)
    {
        let phase = airborne::phase(placement.object) * std::f32::consts::TAU;
        let t = now;
        // Subtle bank and pitch, through the regular glide.
        transform.rotation *= Quat::from_rotation_x((t * 1.15 + phase).sin() * 0.012)
            * Quat::from_rotation_y((t * 0.83 + phase).cos() * 0.016);
    }
    // A card in a fan is tipped up and turned; everything else about it —
    // where it stands, the deck under it, the hover lift below — is the
    // same arithmetic every other card gets.
    if let Some((pose, _)) = placement.fan {
        transform.rotation = fan_rotation(&placement.slot, pose);
    }
    // Hover (cursor) lifts the card a touch; a chosen card stays raised
    // until the choice is answered, and so does an armed one — a deed
    // waiting on a second tap is a commitment the player has already
    // made, which is the same claim being selected makes and belongs at
    // the same height. Selected wins over both; the pointer moving away
    // must not put an armed card back down.
    // Where the card stands with nothing touching it, kept before the two
    // branches below add what is. See [`CardRest`].
    let resting = transform;
    if placement.selected || placement.offer.armed {
        transform.translation.y += SELECTED_LIFT;
        transform.scale *= SELECTED_SCALE;
    } else if hovered == Some(placement.object) {
        transform.translation.y += HOVER_LIFT;
        transform.scale *= HOVER_SCALE;
    }

    (transform, resting)
}

#[cfg(test)]
mod tests;
