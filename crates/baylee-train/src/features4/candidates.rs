/// Encodes all legal candidates for `pending` given `state` and `view`.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "One explicit encoding arm per Pending variant"
)]
pub fn encode_candidates4(
    state: &EncodedState4,
    view: &PlayerView,
    pending: &Pending,
) -> EncodedCandidates4 {
    let viewer = view.seat;
    let seat_count = view.seats.len().clamp(1, MAX_SEATS);
    let pending_kind = PendingKind4::of(pending);
    let mut out = EncodedCandidates4 {
        head: HeadKind4::SingleChoice,
        pending_kind,
        min_choices: 1,
        max_choices: 1,
        candidates: Vec::new(),
        pairs: Vec::new(),
        pair_objects: Vec::new(),
    };

    match pending {
        Pending::Priority { legal, .. } => {
            if legal.can_pass {
                let mut pass = Candidate4::default();
                pass.feat[0] = 1.0; // PassPriority
                pass.action = Some(PlayerAction::PassPriority);
                out.candidates.push(pass);
            }

            if let Some(cast_id) = view.casting
                && out.candidates.len() < MAX_CANDIDATES
            {
                let mut cancel = Candidate4::default();
                cancel.feat[1] = 1.0; // CancelCast
                if let Some(eidx) = state.find_entity(cast_id) {
                    cancel.entity_idx = eidx as i16;
                    cancel.card_id = state.entity_card_ids[eidx];
                }
                cancel.action = Some(PlayerAction::CancelCast);
                out.candidates.push(cancel);
            }

            for &land_id in &legal.lands {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[2] = 1.0; // PlayLand
                if let Some(eidx) = state.find_entity(land_id) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::PlayLand { card: land_id });
                out.candidates.push(c);
            }

            for &spell_id in &legal.castable {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[3] = 1.0; // CastSpell
                if let Some(eidx) = state.find_entity(spell_id) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                    c.feat[16] = state.entity_feat[eidx][45];
                }
                c.action = Some(PlayerAction::CastSpell { card: spell_id });
                out.candidates.push(c);
            }

            for &susp_id in &legal.suspendable {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[3] = 1.0;
                c.feat[18] = 1.0; // Suspend
                if let Some(eidx) = state.find_entity(susp_id) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::Suspend { card: susp_id });
                out.candidates.push(c);
            }

            for &mana_src in &legal.mana_abilities {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[5] = 1.0; // ActivateManaAbility
                if let Some(eidx) = state.find_entity(mana_src) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ActivateManaAbility { source: mana_src });
                out.candidates.push(c);
            }

            for &(src_id, ab_idx) in &legal.abilities {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[4] = 1.0; // ActivateAbility
                c.feat[17] = unit(ab_idx as f32, 8.0);
                if let Some(eidx) = state.find_entity(src_id) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                    c.ability_idx = i64::from(ab_idx);
                    populate_cand_ability_dsl(&mut c, state, eidx, ab_idx as usize);
                }
                c.action = Some(PlayerAction::ActivateAbility {
                    source: src_id,
                    ability_index: ab_idx,
                });
                out.candidates.push(c);
            }

            for grant in &legal.granted_actions {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[8] = 1.0; // TakeGrantedAction
                if let Some(eidx) = state.find_entity(grant.source.object) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::TakeGrantedAction { id: grant.id });
                out.candidates.push(c);
            }
        }

        Pending::ChooseManaAbility {
            choice, options, ..
        } => {
            for (idx, opt) in options.iter().enumerate() {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[5] = 1.0; // ChooseManaAbility
                c.feat[16] = unit(idx as f32, 16.0);
                c.feat[17] = unit(opt.ability_index.unwrap_or(0) as f32, 8.0);
                c.ability_idx = opt.ability_index.map_or(-1, i64::from);
                if let Some(eidx) = state.find_entity(opt.source.object) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ChooseManaAbility {
                    choice: *choice,
                    source: opt.source,
                    ability_index: opt.ability_index,
                });
                out.candidates.push(c);
            }
        }

        Pending::ChooseDamageSource {
            choice, options, ..
        } => {
            for &src in options {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[6] = 1.0; // ChooseDamageSource
                if let Some(eidx) = state.find_entity(src.object) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                } else if let Some(dsv) = view.damage_sources.iter().find(|d| d.source == src)
                    && let Some(ci) = dsv.card
                {
                    c.card_id = ci.index.get();
                }
                c.action = Some(PlayerAction::ChooseDamageSource {
                    choice: *choice,
                    source: src,
                });
                out.candidates.push(c);
            }
        }

        Pending::ChooseDamageEffect {
            choice, options, ..
        } => {
            for eff in options {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[7] = 1.0; // ChooseDamageEffect
                match eff.kind {
                    DamageEffectKind::PreventNext { .. }
                    | DamageEffectKind::PreventThisEvent { .. }
                    | DamageEffectKind::PreventCombat
                    | DamageEffectKind::Protection
                    | DamageEffectKind::PreventAll
                    | DamageEffectKind::PreventFromSource { .. }
                    | DamageEffectKind::RemoveShieldCounter
                    | DamageEffectKind::RemoveCounter { .. } => c.feat[16] = 1.0,
                    DamageEffectKind::Redirect { .. } | DamageEffectKind::RedirectNext { .. } => {
                        c.feat[17] = 1.0;
                    }
                    DamageEffectKind::ReplaceAmount { .. }
                    | DamageEffectKind::Reflect { .. }
                    | DamageEffectKind::TurnFaceUp { .. } => c.feat[18] = 1.0,
                }
                if let Some(src_obj) = eff.source
                    && let Some(eidx) = state.find_entity(src_obj)
                {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ChooseDamageEffect {
                    choice: *choice,
                    effect: eff.id,
                });
                out.candidates.push(c);
            }
        }

        Pending::AllocatePrevention {
            choice,
            damage,
            total,
            ..
        } => {
            let mut rem = *total;
            let mut alloc = Vec::new();
            for d in damage {
                let take = rem.min(d.amount);
                alloc.push((d.id, take));
                rem -= take;
            }
            let mut c = Candidate4::default();
            c.feat[7] = 1.0;
            c.feat[19] = unit(*total as f32, 10.0);
            c.action = Some(PlayerAction::AllocatePrevention {
                choice: *choice,
                allocation: alloc,
            });
            out.candidates.push(c);
        }

        Pending::ChooseCastMode {
            object, options, ..
        } => {
            for (idx, opt) in options.iter().enumerate() {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                encode_cast_mode_kind_into(&opt.kind, &opt.cost, idx as u8, &mut c.feat);
                if let Some(eidx) = state.find_entity(*object) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                    c.extra_rules = crate::cardstructure::refs::mode_refs(
                        c.card_id,
                        state.entity_face_ids[eidx],
                        &opt.kind,
                    );
                }
                c.action = Some(PlayerAction::ChooseMode(idx));
                out.candidates.push(c);
            }
        }

        Pending::Mulligan { can_take, .. } => {
            let mut keep = Candidate4::default();
            keep.feat[9] = 1.0;
            keep.action = Some(PlayerAction::MulliganKeep);
            out.candidates.push(keep);

            if *can_take {
                let mut take = Candidate4::default();
                take.feat[10] = 1.0;
                take.action = Some(PlayerAction::MulliganTake);
                out.candidates.push(take);
            }
        }

        Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
            out.head = HeadKind4::SubsetPick;
            out.min_choices = u32::from(*count);
            out.max_choices = u32::from(*count);
            for h_obj in view.hand.iter().take(MAX_CANDIDATES) {
                let mut c = Candidate4::default();
                c.feat[6] = 1.0;
                if let Some(eidx) = state.find_entity(h_obj.id) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.object_id = Some(h_obj.id);
                out.candidates.push(c);
            }
        }

        Pending::LegendChoice { options, .. } => {
            for &oid in options.iter().take(MAX_CANDIDATES) {
                let mut c = Candidate4::default();
                c.feat[6] = 1.0;
                if let Some(eidx) = state.find_entity(oid) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ChooseObjects { objects: vec![oid] });
                out.candidates.push(c);
            }
        }

        Pending::YesNo { source, .. } => {
            let cid = source.map_or(UNKNOWN_CARD, |a| a.card.get());
            for yn in [true, false] {
                let mut c = Candidate4::default();
                c.feat[11] = 1.0;
                c.feat[16] = f32::from(u8::from(yn));
                c.entity_idx = -1;
                c.card_id = cid;
                c.action = Some(PlayerAction::YesNo(yn));
                out.candidates.push(c);
            }
        }

        Pending::ChooseColor { options, .. } => {
            for &mc in options {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[13] = 1.0;
                c.feat[16 + mc.index().min(5)] = 1.0;
                c.action = Some(PlayerAction::ChooseColor(mc));
                out.candidates.push(c);
            }
        }

        Pending::ChoosePlayer { options, .. } => {
            for &pid in options {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[14] = 1.0;
                let rel = rel_seat(viewer, Some(pid), seat_count);
                c.rel_seat_idx = rel as i8;
                c.feat[16 + rel.min(7)] = 1.0;
                c.action = Some(PlayerAction::ChoosePlayer(pid));
                out.candidates.push(c);
            }
        }

        Pending::ChoosePile { piles, .. } => {
            for (p_idx, pile) in piles.iter().enumerate() {
                let mut c = Candidate4::default();
                c.feat[15] = 1.0;
                c.feat[18] = p_idx as f32;
                c.feat[19] = unit(pile.len() as f32, 10.0);
                if let Some(&first_obj) = pile.first()
                    && let Some(eidx) = state.find_entity(first_obj)
                {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(PlayerAction::ChooseMode(p_idx));
                out.candidates.push(c);
            }
        }

        Pending::ChooseSubtype { options, .. } => {
            let mut prioritized: Vec<baylee_core::ids::SubtypeId> = Vec::new();
            for h_obj in &view.hand {
                if let Some(def) = baylee_cards::by_index(h_obj.card.index) {
                    for face in def.faces {
                        for &st in face.subtypes {
                            if options.contains(&st) && !prioritized.contains(&st) {
                                prioritized.push(st);
                            }
                        }
                    }
                }
            }
            for obj in view
                .battlefield_of(viewer)
                .chain(view.battlefield.iter())
                .chain(&view.stack)
                .chain(view.graveyards.iter().flatten())
                .chain(view.command.iter().flatten())
                .chain(view.exile.iter().flatten())
            {
                for st in obj.subtypes.iter() {
                    if options.contains(&st) && !prioritized.contains(&st) {
                        prioritized.push(st);
                    }
                }
                if let Some(ci) = obj.card
                    && let Some(def) = baylee_cards::by_index(ci.index)
                {
                    for face in def.faces {
                        for &st in face.subtypes {
                            if options.contains(&st) && !prioritized.contains(&st) {
                                prioritized.push(st);
                            }
                        }
                    }
                }
            }
            for &st in options {
                if prioritized.len() >= MAX_CANDIDATES {
                    break;
                }
                if !prioritized.contains(&st) {
                    prioritized.push(st);
                }
            }
            for st in prioritized.into_iter().take(MAX_CANDIDATES) {
                let mut c = Candidate4::default();
                c.feat[15] = 1.0;
                c.feat[20] = f32::from(st.get()) / 1024.0;
                c.action = Some(PlayerAction::ChooseSubtype(st));
                out.candidates.push(c);
            }
        }

        // "Choose a card type" (Serra's Emissary): one candidate per offered
        // type, in the engine's order, marked like the other fixed pick lists.
        Pending::ChooseCardType { options, .. } => {
            for (i, &card_type) in options.iter().enumerate() {
                if out.candidates.len() >= MAX_CANDIDATES {
                    break;
                }
                let mut c = Candidate4::default();
                c.feat[15] = 1.0;
                c.feat[18] = unit(i as f32, 16.0);
                c.action = Some(PlayerAction::ChooseCardType(card_type));
                out.candidates.push(c);
            }
        }

        Pending::ChooseCardName {
            types,
            excluded_types,
            ..
        } => {
            // Naming is a choice over the public registry, not just the cards
            // currently visible plus one arbitrary fallback. Structural card
            // features distinguish candidates with no board entity pointer.
            for def in baylee_cards::all() {
                for (face_idx, face) in def.faces.iter().enumerate() {
                    if (!types.is_empty() && !face.types.intersects(*types))
                        || face.types.intersects(*excluded_types)
                    {
                        continue;
                    }
                    let mut candidate = Candidate4::default();
                    candidate.feat[15] = 1.0;
                    candidate.card_id = def.index.get();
                    candidate.action = Some(PlayerAction::ChooseCardName {
                        card: def.index,
                        face: face_idx as u8,
                    });
                    out.candidates.push(candidate);
                }
            }
        }

        Pending::Arrange { cards, piles, .. } => {
            let arrangements = enumerate_arrangements(cards, piles);
            for (arr_idx, arr) in arrangements.into_iter().enumerate().take(MAX_CANDIDATES) {
                let mut c = Candidate4::default();
                c.feat[15] = 1.0;
                c.feat[21] = unit(arr_idx as f32, 16.0);
                let mut rep_obj = cards.first().copied();
                if let PlayerAction::Arrange {
                    piles: ref arr_piles,
                } = arr
                {
                    let p0_len = arr_piles.first().map_or(0, Vec::len);
                    let p1_len = arr_piles.get(1).map_or(0, Vec::len);
                    c.feat[22] = unit(p0_len as f32, 5.0);
                    c.feat[23] = unit(p1_len as f32, 5.0);
                    if let Some(p0) = arr_piles.first() {
                        if let Some(&top_id) = p0.first() {
                            rep_obj = Some(top_id);
                        }
                        for (k, &cid) in cards.iter().take(5).enumerate() {
                            if p0.contains(&cid) {
                                c.feat[24 + k] = 1.0;
                            }
                        }
                    }
                }
                if let Some(oid) = rep_obj
                    && let Some(eidx) = state.find_entity(oid)
                {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                c.action = Some(arr);
                out.candidates.push(c);
            }
        }

        Pending::ChooseAttackers {
            attackers,
            defenders,
            required,
            minimum_required,
            limits,
            ..
        } => {
            out.head = HeadKind4::SubsetPick;
            out.min_choices = minimum_required.unwrap_or(required.len() as u32);
            out.max_choices = attackers.len() as u32;
            let best_def = choose_best_attack_target(view, defenders);
            let per_attacker_defs: Vec<(ObjectId, Vec<Defender>)> = attackers
                .iter()
                .map(|&cid| {
                    let allowed_defs: &[Defender] = limits
                        .iter()
                        .find(|l| l.creature == cid)
                        .map_or(defenders.as_slice(), |l| l.defenders.as_slice());
                    let mut ordered_defs: Vec<Defender> = Vec::with_capacity(allowed_defs.len());
                    if let Some(bd) = best_def
                        && allowed_defs.contains(&bd)
                    {
                        ordered_defs.push(bd);
                    }
                    for &d in allowed_defs {
                        if !ordered_defs.contains(&d) {
                            ordered_defs.push(d);
                        }
                    }
                    (cid, ordered_defs)
                })
                .collect();
            let max_defs = per_attacker_defs
                .iter()
                .map(|(_, ds)| ds.len())
                .max()
                .unwrap_or(0);
            // Round-robin by defender rank so every attacker gets its primary defender
            // before secondary defenders fill remaining MAX_CANDIDATES slots.
            for def_rank in 0..max_defs {
                for (cid, ordered_defs) in &per_attacker_defs {
                    if out.candidates.len() >= MAX_CANDIDATES {
                        break;
                    }
                    let Some(&def) = ordered_defs.get(def_rank) else {
                        continue;
                    };
                    let mut c = Candidate4::default();
                    c.feat[5] = 1.0;
                    c.feat[16] = f32::from(u8::from(required.contains(cid)));
                    if let Some(eidx) = state.find_entity(*cid) {
                        c.entity_idx = eidx as i16;
                        c.card_id = state.entity_card_ids[eidx];
                    }
                    match def {
                        Defender::Player(pid) => {
                            let rel = rel_seat(viewer, Some(pid), seat_count);
                            c.rel_seat_idx = rel as i8;
                            c.feat[17] = 1.0;
                            c.feat[20 + rel.min(7)] = 1.0;
                        }
                        Defender::Planeswalker(pw_id) => {
                            c.feat[18] = 1.0;
                            if let Some(pw_eidx) = state.find_entity(pw_id) {
                                c.feat[28] = unit(pw_eidx as f32, 64.0);
                                let rel = state.entity_rel_controller[pw_eidx] as usize;
                                if rel < MAX_SEATS {
                                    c.rel_seat_idx = rel as i8;
                                    c.feat[20 + rel] = 1.0;
                                }
                            }
                        }
                        Defender::Battle(b_id) => {
                            c.feat[19] = 1.0;
                            if let Some(b_eidx) = state.find_entity(b_id) {
                                c.feat[28] = unit(b_eidx as f32, 64.0);
                                let rel = state.entity_rel_controller[b_eidx] as usize;
                                if rel < MAX_SEATS {
                                    c.rel_seat_idx = rel as i8;
                                    c.feat[20 + rel] = 1.0;
                                }
                            }
                        }
                    }
                    c.object_id = Some(*cid);
                    c.defender = Some(def);
                    out.candidates.push(c);
                }
            }
        }

        Pending::ChooseCards {
            options: objs,
            min,
            max,
            ..
        } => {
            let single = *min == 1 && *max == 1;
            out.head = if single {
                HeadKind4::SingleChoice
            } else {
                HeadKind4::SubsetPick
            };
            out.min_choices = u32::from(*min);
            out.max_choices = u32::from(*max);
            let cap = MAX_CANDIDATES.saturating_sub(out.candidates.len());
            for oid in prioritize_object_options(view, objs, cap) {
                let mut c = Candidate4::default();
                c.feat[6] = 1.0;
                if let Some(eidx) = state.find_entity(oid) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                if single {
                    c.action = Some(PlayerAction::ChooseObjects { objects: vec![oid] });
                } else {
                    c.object_id = Some(oid);
                }
                out.candidates.push(c);
            }
        }

        Pending::ChooseTargets {
            options,
            player_options,
            min,
            max,
            ..
        } => {
            let single = *min <= 1 && *max == 1;
            out.head = if single {
                HeadKind4::SingleChoice
            } else {
                HeadKind4::SubsetPick
            };
            out.min_choices = *min;
            out.max_choices = *max;
            if *min == 0 && single {
                let mut none_cand = Candidate4::default();
                none_cand.feat[0] = 1.0;
                none_cand.action = Some(PlayerAction::ChooseTargets {
                    objects: Vec::new(),
                    players: Vec::new(),
                });
                out.candidates.push(none_cand);
            }
            let reserved_players = player_options.len().min(MAX_SEATS);
            let obj_cap = MAX_CANDIDATES
                .saturating_sub(out.candidates.len())
                .saturating_sub(reserved_players);
            for oid in prioritize_object_options(view, options, obj_cap) {
                let mut c = Candidate4::default();
                c.feat[6] = 1.0;
                if let Some(eidx) = state.find_entity(oid) {
                    c.entity_idx = eidx as i16;
                    c.card_id = state.entity_card_ids[eidx];
                }
                if single {
                    c.action = Some(PlayerAction::ChooseTargets {
                        objects: vec![oid],
                        players: Vec::new(),
                    });
                } else {
                    c.object_id = Some(oid);
                }
                out.candidates.push(c);
            }
            for &pid in player_options
                .iter()
                .take(MAX_CANDIDATES.saturating_sub(out.candidates.len()))
            {
                let mut c = Candidate4::default();
                c.feat[14] = 1.0;
                let rel = rel_seat(viewer, Some(pid), seat_count);
                c.rel_seat_idx = rel as i8;
                c.feat[16 + rel.min(7)] = 1.0;
                if single {
                    c.action = Some(PlayerAction::ChooseTargets {
                        objects: Vec::new(),
                        players: vec![pid],
                    });
                } else {
                    c.player_id = Some(pid);
                }
                out.candidates.push(c);
            }
        }

        Pending::ChooseBlockers { blockers, .. } => {
            out.head = HeadKind4::PairPick;
            let max_atk = blockers
                .iter()
                .map(|b| b.attackers.len())
                .max()
                .unwrap_or(0);
            for atk_idx in 0..max_atk {
                for b in blockers {
                    if out.pairs.len() >= MAX_PAIRS {
                        break;
                    }
                    let Some(&atk_id) = b.attackers.get(atk_idx) else {
                        continue;
                    };
                    let b_eidx = state.find_entity(b.blocker).map_or(-1, |i| i as i16);
                    let a_eidx = state.find_entity(atk_id).map_or(-1, |i| i as i16);
                    out.pairs.push([b_eidx, a_eidx]);
                    out.pair_objects.push((b.blocker, atk_id));
                }
            }
        }

        Pending::ChooseNumber {
            min, max, reason, ..
        } => {
            out.head = HeadKind4::ScalarValue;
            out.min_choices = *min;
            out.max_choices = bounded_scalar_max(view, *min, *max, *reason);
        }

        Pending::GameOver(_) => {}
    }

    out
}

/// Bounds open-ended `ChooseNumber` prompts (`max > 64`, e.g. `2_147_483_647` on
/// `NumberPrompt::X` or `NumberPrompt::ManaPayment`) to the view-only reachable
/// mana or life limit so `normalize_scalar_choice` and `denormalize_scalar_choice`
/// operate on the true payable range rather than `[0, 2^31 - 1]`.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "Bounds X across active casting, top of stack, hand/zone spells, and battlefield abilities"
)]
pub fn bounded_scalar_max(
    view: &PlayerView,
    min: u32,
    max: u32,
    prompt: baylee_engine::choice::NumberPrompt,
) -> u32 {
    if max <= 64 && !matches!(prompt, baylee_engine::choice::NumberPrompt::X) {
        return max.max(min);
    }
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(u32::MAX));
    match prompt {
        baylee_engine::choice::NumberPrompt::ManaPayment { preventable_damage } => {
            let cap = if preventable_damage > 0 {
                pool_total.min(preventable_damage)
            } else {
                pool_total
            };
            cap.clamp(min, max)
        }
        baylee_engine::choice::NumberPrompt::X => {
            let untapped_mana = untapped_mana_production(view, actor);
            let total_mana = pool_total.saturating_add(untapped_mana);
            if let Some(id) = view.casting {
                if let Some(h) = view.hand.iter().find(|h| h.id == id)
                    && let Some(def) = baylee_cards::by_index(h.card.index)
                    && let Some(mx) = bound_x_for_card_def(
                        view,
                        actor,
                        def,
                        usize::from(h.card.face),
                        total_mana,
                        0,
                        true,
                    )
                {
                    return mx.clamp(min, max);
                }
                if let Some(obj) = view.object(id)
                    && let Some(mx) = bound_x_for_public_object(view, actor, obj, total_mana, true)
                {
                    return mx.clamp(min, max);
                }
            }
            if let Some(top) = view.top_of_stack()
                && let Some(mx) = bound_x_for_public_object(view, actor, top, total_mana, true)
            {
                return mx.clamp(min, max);
            }

            // In offline evaluation and self-play, `view.casting` is `None` and the spell
            // or activated ability asking `NumberPrompt::X` has not yet moved to `view.stack`.
            // Inspect visible hand/zone spells and battlefield activated abilities with X costs.
            let mut spell_candidates = Vec::new();
            if actor == view.seat {
                for h in &view.hand {
                    if let Some(def) = baylee_cards::by_index(h.card.index)
                        && let Some(mx) = bound_x_for_card_def(
                            view,
                            actor,
                            def,
                            usize::from(h.card.face),
                            total_mana,
                            0,
                            true,
                        )
                    {
                        spell_candidates.push(mx);
                    }
                }
            }
            for obj in view
                .command
                .iter()
                .flatten()
                .chain(view.graveyards.iter().flatten())
                .chain(view.exile.iter().flatten())
            {
                if obj.owner == actor
                    && let Some(mx) = bound_x_for_public_object(view, actor, obj, total_mana, true)
                {
                    spell_candidates.push(mx);
                }
            }
            if let Some(best_spell_x) = spell_candidates
                .iter()
                .copied()
                .filter(|&mx| mx > 0 && mx >= min)
                .min()
            {
                return best_spell_x.clamp(min, max);
            }

            let mut abil_candidates = Vec::new();
            for obj in view.battlefield_of(actor) {
                if obj.status.contains(ObjectStatus::PHASED_OUT) {
                    continue;
                }
                if let Some(mx) = bound_x_for_public_object(view, actor, obj, total_mana, false) {
                    abil_candidates.push(mx);
                }
            }
            if let Some(best_abil_x) = abil_candidates
                .iter()
                .copied()
                .filter(|&mx| mx > 0 && mx >= min)
                .min()
                .or_else(|| abil_candidates.into_iter().filter(|&mx| mx >= min).min())
            {
                return best_abil_x.clamp(min, max);
            }
            if let Some(zero_spell_x) = spell_candidates.into_iter().filter(|&mx| mx >= min).min() {
                return zero_spell_x.clamp(min, max);
            }

            total_mana.clamp(min, max)
        }
        _ => max.max(min),
    }
}

fn bound_x_for_public_object(
    view: &PlayerView,
    actor: PlayerId,
    obj: &PublicObject,
    total_mana: u32,
    include_spell: bool,
) -> Option<u32> {
    let rf = obj.rules.or_else(|| {
        obj.card.map(|c| baylee_view::RulesFace {
            card: c.index,
            face: c.face,
        })
    })?;
    let def = baylee_cards::by_index(rf.card)?;
    let self_mana = single_object_mana_production(obj);
    bound_x_for_card_def(
        view,
        actor,
        def,
        usize::from(rf.face),
        total_mana,
        self_mana,
        include_spell,
    )
}

fn bound_x_for_card_def(
    view: &PlayerView,
    actor: PlayerId,
    def: &baylee_cards_dsl::CardDef,
    face_idx: usize,
    total_mana: u32,
    self_mana: u32,
    include_spell: bool,
) -> Option<u32> {
    let face = def.faces.get(face_idx)?;
    if include_spell {
        let cost = face.mana_cost;
        if cost.has_variable() {
            let base_cmc = cost.with_x(0).cmc();
            if base_cmc > total_mana {
                return Some(0);
            }
            let x_pips = cost.with_x(1).cmc().saturating_sub(base_cmc).max(1);
            let mut max_x = total_mana.saturating_sub(base_cmc) / x_pips;
            for ab in def.abilities_for_face(face_idx) {
                if let baylee_cards_dsl::AbilityDef::Spell {
                    targets: Some(req), ..
                } = ab
                    && req.count_is_x
                    && let baylee_cards_dsl::TargetSpec::Object(filter) = req.spec
                {
                    let count = u32::try_from(
                        view.battlefield
                            .iter()
                            .filter(|o| public_object_matches_basic_filter(actor, o, filter))
                            .count(),
                    )
                    .unwrap_or(u32::MAX);
                    max_x = max_x.min(count);
                }
            }
            return Some(max_x);
        }
        let pays_life_x = face
            .mandatory_additional_costs
            .contains(&baylee_cards_dsl::CostPart::PayLifeX);
        if pays_life_x {
            let life_cap = view
                .seat(actor)
                .map_or(0, |s| u32::try_from(s.life.saturating_sub(1)).unwrap_or(0));
            return Some(life_cap);
        }
    }

    let mut best_abil: Option<u32> = None;
    for ab in def.abilities_for_face(face_idx) {
        let (cost, target_spec) = match ab {
            baylee_cards_dsl::AbilityDef::Activated { cost, targets, .. }
            | baylee_cards_dsl::AbilityDef::ActivatedConditional { cost, targets, .. } => {
                (cost, targets.as_ref().map(|r| &r.spec))
            }
            baylee_cards_dsl::AbilityDef::ModalActivated { cost, .. } => (cost, None),
            _ => continue,
        };
        if cost.mana.has_variable() {
            let avail = if cost.parts.contains(&baylee_cards_dsl::CostPart::TapSelf) {
                total_mana.saturating_sub(self_mana)
            } else {
                total_mana
            };
            let base_cmc = cost.mana.with_x(0).cmc();
            if base_cmc > avail {
                continue;
            }
            let x_pips = cost.mana.with_x(1).cmc().saturating_sub(base_cmc).max(1);
            let mut max_x = avail.saturating_sub(base_cmc) / x_pips;
            if let Some(baylee_cards_dsl::TargetSpec::CardInGraveyard(
                filter,
                baylee_cards_dsl::PlayerRel::You,
            )) = target_spec
                && filter_requires_cmc_exactly_x(filter)
                && let Some(gy) = view.graveyards.get(usize::from(actor.get()))
            {
                let best_gy = gy
                    .iter()
                    .filter(|o| public_object_matches_basic_filter(actor, o, filter))
                    .filter_map(|o| {
                        let rf = o.rules.or_else(|| {
                            o.card.map(|c| baylee_view::RulesFace {
                                card: c.index,
                                face: c.face,
                            })
                        })?;
                        let d = baylee_cards::by_index(rf.card)?;
                        let f = d.faces.get(usize::from(rf.face))?;
                        Some(f.mana_cost.cmc())
                    })
                    .filter(|&cmc| cmc <= max_x)
                    .max()
                    .unwrap_or(0);
                max_x = best_gy;
            }
            best_abil = Some(best_abil.map_or(max_x, |prev| prev.min(max_x)));
        }
    }
    best_abil
}

fn filter_requires_cmc_exactly_x(filter: &baylee_cards_dsl::Filter) -> bool {
    match filter {
        baylee_cards_dsl::Filter::CmcExactlyX => true,
        baylee_cards_dsl::Filter::And(parts) | baylee_cards_dsl::Filter::Or(parts) => {
            parts.iter().any(filter_requires_cmc_exactly_x)
        }
        _ => false,
    }
}

/// Checks whether a [`PublicObject`] satisfies a basic type/controller/characteristic [`baylee_cards_dsl::Filter`].
#[must_use]
pub fn public_object_matches_basic_filter(
    actor: PlayerId,
    obj: &PublicObject,
    filter: &baylee_cards_dsl::Filter,
) -> bool {
    match filter {
        baylee_cards_dsl::Filter::And(parts) => parts
            .iter()
            .all(|f| public_object_matches_basic_filter(actor, obj, f)),
        baylee_cards_dsl::Filter::Or(parts) => parts
            .iter()
            .any(|f| public_object_matches_basic_filter(actor, obj, f)),
        baylee_cards_dsl::Filter::Not(inner) => {
            !public_object_matches_basic_filter(actor, obj, inner)
        }
        baylee_cards_dsl::Filter::HasType(ts) => obj.types.intersects(*ts),
        baylee_cards_dsl::Filter::LacksType(ts) => !obj.types.intersects(*ts),
        baylee_cards_dsl::Filter::HasSupertype(st) => obj.supertypes.contains(*st),
        baylee_cards_dsl::Filter::HasSubtype(sub) => obj.subtypes.contains(*sub),
        baylee_cards_dsl::Filter::HasColor(cs) => obj.colors.intersects(*cs),
        baylee_cards_dsl::Filter::IsColorless => obj.colors.is_empty(),
        baylee_cards_dsl::Filter::Monocolored => obj.colors.len() == 1,
        baylee_cards_dsl::Filter::HasKeyword(kw) => (obj.keywords & kw.bits()) == kw.bits(),
        baylee_cards_dsl::Filter::IsToken => obj.token.is_some() || obj.card.is_none(),
        baylee_cards_dsl::Filter::IsCommander => obj.commander,
        baylee_cards_dsl::Filter::CmcAtMost(n) => obj.mana_value <= *n,
        baylee_cards_dsl::Filter::CmcAtLeast(n) => obj.mana_value >= *n,
        baylee_cards_dsl::Filter::PowerAtMost(n) => obj.power.is_some_and(|p| p <= *n),
        baylee_cards_dsl::Filter::PowerAtLeast(n) => obj.power.is_some_and(|p| p >= *n),
        baylee_cards_dsl::Filter::ToughnessAtMost(n) => obj.toughness.is_some_and(|t| t <= *n),
        baylee_cards_dsl::Filter::ToughnessAtLeast(n) => obj.toughness.is_some_and(|t| t >= *n),
        baylee_cards_dsl::Filter::ControlledByYou => obj.controller == actor,
        baylee_cards_dsl::Filter::ControlledByOpponent => obj.controller != actor,
        baylee_cards_dsl::Filter::OwnedByYou => obj.owner == actor,
        baylee_cards_dsl::Filter::Tapped => obj.status.contains(ObjectStatus::TAPPED),
        baylee_cards_dsl::Filter::Untapped => !obj.status.contains(ObjectStatus::TAPPED),
        baylee_cards_dsl::Filter::HasAnyCounter => !obj.counters.is_empty(),
        _ => true,
    }
}

fn single_object_mana_production(obj: &PublicObject) -> u32 {
    if obj.status.contains(ObjectStatus::TAPPED)
        || obj.status.contains(ObjectStatus::PHASED_OUT)
        || obj.summoning_sick
    {
        return 0;
    }
    let mut prod = 0_u32;
    for gm in obj.granted_mana_options() {
        prod = prod.max(u32::from(gm.amount));
    }
    if obj.board_mana.is_some() {
        prod = prod.max(1);
    }
    if obj.types.contains(TypeSet::LAND) {
        prod = prod.max(1);
    }
    let rf = obj.rules.or_else(|| {
        obj.card.map(|c| baylee_view::RulesFace {
            card: c.index,
            face: c.face,
        })
    });
    if let Some(rf) = rf
        && let Some(def) = baylee_cards::by_index(rf.card)
    {
        for ab in def.abilities_for_face(usize::from(rf.face)) {
            if let baylee_cards_dsl::AbilityDef::Activated {
                cost,
                effects,
                targets,
                ..
            } = ab
                && baylee_cards_dsl::mana_rule::activated_mana_ability(
                    cost,
                    effects,
                    targets.is_some(),
                )
            {
                let counter_unmet = cost.parts.iter().any(|p| match p {
                    baylee_cards_dsl::CostPart::RemoveCounterSelf { n, .. } => {
                        obj.counters.iter().map(|c| c.count).sum::<u16>() < *n
                    }
                    baylee_cards_dsl::CostPart::RemoveCounterSelfX { .. } => {
                        obj.counters.iter().map(|c| c.count).sum::<u16>() < 1
                    }
                    baylee_cards_dsl::CostPart::TapOther(_)
                    | baylee_cards_dsl::CostPart::TapX(_)
                    | baylee_cards_dsl::CostPart::Sacrifice(_)
                    | baylee_cards_dsl::CostPart::SacrificeX(_) => true,
                    _ => false,
                });
                if counter_unmet {
                    continue;
                }
                let amt = mana_effects_amount(effects).saturating_sub(cost.mana.cmc());
                prod = prod.max(amt);
            }
        }
    } else if let Some(tok_id) = obj.token
        && let Some(tdef) = baylee_cards::tokens::by_token_id(tok_id)
    {
        for ab in tdef.abilities {
            if let baylee_cards_dsl::AbilityDef::Activated {
                cost,
                effects,
                targets,
                ..
            } = ab
                && baylee_cards_dsl::mana_rule::activated_mana_ability(
                    cost,
                    effects,
                    targets.is_some(),
                )
            {
                let amt = mana_effects_amount(effects).saturating_sub(cost.mana.cmc());
                prod = prod.max(amt);
            }
        }
    }
    prod
}

/// Estimates the total mana producible right now by untapped permanents
/// controlled by `actor` on `view.battlefield`.
#[must_use]
pub fn untapped_mana_production(view: &PlayerView, actor: PlayerId) -> u32 {
    view.battlefield_of(actor)
        .map(single_object_mana_production)
        .fold(0_u32, u32::saturating_add)
}

fn mana_effects_amount(effects: &[baylee_cards_dsl::Effect]) -> u32 {
    let mut sum = 0_u32;
    for eff in effects {
        if let baylee_cards_dsl::Effect::AddMana {
            amount,
            restriction,
            ..
        } = eff
        {
            if restriction.is_some_and(|r| r.restricts) {
                continue;
            }
            let n = match amount {
                baylee_cards_dsl::Amount::Fixed(v) => (*v).max(1),
                _ => 1,
            };
            sum = sum.saturating_add(n.max(1));
        }
    }
    sum
}

fn prioritize_object_options(view: &PlayerView, objs: &[ObjectId], cap: usize) -> Vec<ObjectId> {
    if objs.len() <= cap {
        return objs.to_vec();
    }
    // Partition into:
    // 0: unique non-land objects by (controller, card_id, token, status, power, toughness)
    // 1: unique land objects by (controller, card_id, status)
    // 2: remaining non-land objects
    // 3: remaining land objects
    let mut tier0 = Vec::new();
    let mut tier1 = Vec::new();
    let mut tier2 = Vec::new();
    let mut tier3 = Vec::new();
    let mut seen_nonland = Vec::new();
    let mut seen_land = Vec::new();

    for &oid in objs {
        if let Some(o) = view.object(oid) {
            let cid = o.card.map_or(UNKNOWN_CARD, |c| c.index.get());
            let tok = o.token.unwrap_or(0);
            if o.types.contains(TypeSet::LAND) {
                let sig = (o.controller.get(), cid, o.status.bits());
                if seen_land.contains(&sig) {
                    tier3.push(oid);
                } else {
                    seen_land.push(sig);
                    tier1.push(oid);
                }
            } else {
                let sig = (
                    o.controller.get(),
                    cid,
                    tok,
                    o.status.bits(),
                    o.power.unwrap_or(0),
                    o.toughness.unwrap_or(0),
                );
                if seen_nonland.contains(&sig) {
                    tier2.push(oid);
                } else {
                    seen_nonland.push(sig);
                    tier0.push(oid);
                }
            }
        } else {
            tier0.push(oid);
        }
    }

    tier0
        .into_iter()
        .chain(tier1)
        .chain(tier2)
        .chain(tier3)
        .take(cap)
        .collect()
}

fn populate_cand_ability_dsl(
    c: &mut Candidate4,
    state: &EncodedState4,
    entity_idx: usize,
    ab_idx: usize,
) {
    let cid = state.entity_card_ids[entity_idx];
    let fid = state.entity_face_ids[entity_idx] as usize;
    if cid != UNKNOWN_CARD
        && let Some(def) = baylee_cards::by_index(CardIndex::new(cid))
        && let Some(ab) = def.abilities_for_face(fid).get(ab_idx)
    {
        let row = encode_ability_def(ab);
        c.feat[24..48].copy_from_slice(&row[40..64]);
        return;
    }
    let tok_slot = state.entity_token_ids[entity_idx];
    if tok_slot != 0
        && let Some(tok) = baylee_cards::tokens::by_token_id(tok_slot - 1)
        && let Some(ab) = tok.abilities.get(ab_idx)
    {
        let row = encode_ability_def(ab);
        c.feat[24..48].copy_from_slice(&row[40..64]);
    }
}

/// Encodes all 21 [`CastModeKind`] variants + option [`ManaCost`] into `feat[0..48]`
/// (fixes P0-4: `ChooseMode` options are never collapsed to `[0, 0, index]`).
fn encode_cast_mode_kind_into(kind: &CastModeKind, cost: &ManaCost, index: u8, feat: &mut [f32]) {
    feat[14] = 1.0; // ChooseMode family marker
    feat[15] = unit(f32::from(index), 8.0);
    let mc = encode_mana_cost(cost);
    feat[16..26].copy_from_slice(&mc);

    let kind_slot = match kind {
        CastModeKind::Normal => 26,
        CastModeKind::Kicked => 27,
        // Foretell is an alternative cost (CR 702.143a): its slot, so the
        // candidate width the trained nets read stays the same.
        CastModeKind::Alternative(_) | CastModeKind::Foretell => 28,
        CastModeKind::Mode(_) => 29,
        CastModeKind::Modes(_) => 30,
        CastModeKind::Face(_) => 31,
        CastModeKind::PlayLandFace(_) => 32,
        CastModeKind::Prototype => 33,
        CastModeKind::Disguise => 34,
        CastModeKind::Miracle => 35,
        CastModeKind::Flashback => 36,
        CastModeKind::BeamMeUp => 37,
        CastModeKind::Dash => 38,
        CastModeKind::Mutate => 39,
        CastModeKind::MutatePosition(_) => 40,
        CastModeKind::Dungeon(_) | CastModeKind::DungeonRoom { .. } => 41,
        CastModeKind::Escape | CastModeKind::EscapeGranted { .. } => 42,
        CastModeKind::Permission(_) => 43,
        CastModeKind::Counter(_) => 44,
        CastModeKind::Colors(_) => 45,
    };
    feat[kind_slot] = 1.0;
    match kind {
        CastModeKind::Mode(m) | CastModeKind::Face(m) | CastModeKind::PlayLandFace(m) => {
            feat[46] = unit(*m as f32, 6.0);
        }
        CastModeKind::MutatePosition(on_top) => {
            feat[47] = f32::from(u8::from(*on_top));
        }
        _ => {}
    }
}

fn enumerate_arrangements(cards: &[ObjectId], piles: &[ArrangePile]) -> Vec<PlayerAction> {
    let mut out = Vec::new();
    let mut push_unique = |p: Vec<Vec<ObjectId>>| {
        if out.len() >= MAX_CANDIDATES {
            return;
        }
        let act = PlayerAction::Arrange { piles: p };
        if !out.contains(&act) {
            out.push(act);
        }
    };
    if let Some(def) = default_arrangement(cards, piles) {
        push_unique(def);
    }
    if cards.len() > 1 {
        let mut rev_cards = cards.to_vec();
        rev_cards.reverse();
        if let Some(rev) = default_arrangement(&rev_cards, piles) {
            push_unique(rev);
        }
    }
    // Enumerate all legal 2-pile splits (Scry, Surveil, DiscardToLibrary, Fact or Fiction) up to 5 cards
    if piles.len() == 2 && cards.len() <= 5 {
        let n = cards.len();
        for mask in 0..(1_usize << n) {
            let mut p0 = Vec::with_capacity(n);
            let mut p1 = Vec::with_capacity(n);
            for (idx, &cid) in cards.iter().enumerate() {
                if (mask & (1 << idx)) == 0 {
                    p0.push(cid);
                } else {
                    p1.push(cid);
                }
            }
            if p0.len() >= piles[0].min as usize
                && p0.len() <= piles[0].max as usize
                && p1.len() >= piles[1].min as usize
                && p1.len() <= piles[1].max as usize
            {
                push_unique(vec![p0.clone(), p1.clone()]);
                if p0.len() >= 2 && n <= 4 {
                    let mut p0_rev = p0.clone();
                    p0_rev.reverse();
                    push_unique(vec![p0_rev, p1.clone()]);
                }
                if p1.len() >= 2 && n <= 4 {
                    let mut p1_rev = p1;
                    p1_rev.reverse();
                    push_unique(vec![p0, p1_rev]);
                }
            }
        }
    } else if piles.len() == 2 && piles[0].min == 0 && (piles[1].max as usize) >= cards.len() {
        push_unique(vec![Vec::new(), cards.to_vec()]);
    }
    out
}
