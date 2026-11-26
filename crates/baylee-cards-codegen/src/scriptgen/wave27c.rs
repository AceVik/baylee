//! Wave 27c: resolution-scoped `Effect::CaptureAffected` / `Remembered`
//! follow-ups and 2–3 slot independent target chains (`WithThirdTarget`,
//! `WithAllTargets`, `FirstTargetPlayerOrController`,
//! `ThirdTargetPlayerOrController`), mapped onto Engine Wave 15c DSL primitives.
//!
//! Pure Paper-MTG only: Alchemy/digital (`MakeCard`, `Seek`, `Perpetual`,
//! `Conjure`, `Spellbook`, `Specialize`, `Heist`, `Intensity`, `Boon`,
//! `Draft`, `Empower`) and Un-set/Acorn/Sticker/Planechase constructs remain
//! refused, and any `Remember*` / `Forget*` / `ClearRemembered` semantics
//! outside `CaptureAffected`'s resolution-scoped guarantee are refused
//! honestly.

use super::chains::LineFlags;
use super::{Chain, Params, Tx};
use std::collections::BTreeSet;

/// Resolution-scoped capture context active while lowering the `then` body of
/// `Effect::CaptureAffected { first, then }`.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct CaptureScope {
    /// True when `first` can affect at most 1 object.
    pub(super) single_object: bool,
    /// Forge valid-string atom that every object captured by `first` is
    /// guaranteed to satisfy (e.g. `"Creature"` for `SacValid$ Creature`).
    pub(super) guaranteed_valid: Option<String>,
    /// True when `first` only affects permanents on the battlefield.
    pub(super) from_battlefield: bool,
    /// True when every object captured by `first` is guaranteed owned by `You`.
    pub(super) you_own_only: bool,
    /// True when `first` acts on multiple players simultaneously (`Defined$ Player`
    /// or multi-player `ChangeZoneAll`/`SacrificeAll`), so a per-player follow-up
    /// reading `Remembered$Amount` would conflate all players' counts.
    pub(super) multi_player: bool,
}

/// Which target slot a `chain_step` line just declared or operates on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum MultiTargetAction {
    /// Line declared Slot 3 (`third_targets`); wrap in `Effect::WithThirdTarget`.
    ThirdTarget,
    /// Trailing `Defined$ Targeted` line after a 2- or 3-slot target chain;
    /// wrap in `Effect::WithAllTargets`.
    AllTargets,
}

impl Tx<'_> {
    // =========================================================================
    // Part 1: Resolution-scoped `CaptureAffected` & `Remembered` follow-ups
    // =========================================================================

    /// Filter atoms available inside `Effect::CaptureAffected::then`.
    pub(super) fn wave27c_remembered_filter_atom(&self, atom: &str) -> Option<String> {
        self.capture_scope.as_ref()?;
        Some(
            match atom {
                "IsRemembered" => "Filter::IsRemembered",
                "!IsRemembered" => "Filter::NotRemembered",
                "sharesCardTypeWith Remembered"
                | "sharesCardTypeWith RememberedLKI"
                | "sharesCardTypeWith RememberedCard" => "Filter::SharesCardTypeWithRemembered",
                "!sharesCardTypeWith Remembered"
                | "!sharesCardTypeWith RememberedLKI"
                | "!sharesCardTypeWith RememberedCard" => {
                    "Filter::Not(&Filter::SharesCardTypeWithRemembered)"
                }
                "sharesCardTypeWithOther Remembered" | "sharesCardTypeWithOther RememberedLKI" => {
                    "Filter::And(&[Filter::SharesCardTypeWithRemembered, Filter::NotRemembered])"
                }
                "sharesNameWith Remembered"
                | "sharesNameWith RememberedLKI"
                | "sharesNameWith RememberedCard" => "Filter::SharesNameWithRemembered",
                "!sharesNameWith Remembered"
                | "!sharesNameWith RememberedLKI"
                | "!sharesNameWith RememberedCard"
                | "doesNotShareNameWith Remembered"
                | "doesNotShareNameWith RememberedLKI" => {
                    "Filter::Not(&Filter::SharesNameWithRemembered)"
                }
                "ControlledBy RememberedController" => {
                    "Filter::ControlledBy(PlayerRel::RememberedController)"
                }
                _ => return None,
            }
            .to_string(),
        )
    }

    /// `PlayerRel` forms available inside `Effect::CaptureAffected::then`.
    pub(super) fn wave27c_remembered_player_rel(&self, defined: &str) -> Option<&'static str> {
        self.capture_scope.as_ref()?;
        match defined {
            "RememberedController" | "Player.RememberedController" => {
                Some("PlayerRel::RememberedController")
            }
            _ => None,
        }
    }

    /// `Amount` expressions available inside `Effect::CaptureAffected::then`.
    pub(super) fn wave27c_remembered_amount(&self, def: &str) -> Option<String> {
        let scope = self.capture_scope.as_ref()?;
        let trimmed = def.trim();
        match trimmed {
            "Remembered$Amount" | "Count$RememberedSize" => {
                Some("Amount::RememberedCount".to_string())
            }
            "Remembered$CardManaCost"
            | "RememberedLKI$CardManaCost"
            | "Remembered$CardManaCostLKI" => Some("Amount::RememberedManaValue".to_string()),
            "Remembered$CardPower" | "RememberedLKI$CardPower" => {
                Some("Amount::RememberedPower".to_string())
            }
            "Remembered$CardToughness" | "RememberedLKI$CardToughness" => {
                Some("Amount::RememberedToughness".to_string())
            }
            _ => {
                let valid = trimmed
                    .strip_prefix("Remembered$Valid ")
                    .or_else(|| trimmed.strip_prefix("RememberedLKI$Valid "))?
                    .trim();
                Self::scope_guarantees_present(scope, valid)
                    .then(|| "Amount::RememberedCount".to_string())
            }
        }
    }

    /// `GainControl | Defined$ Self | NewController$ RememberedController`
    /// (*Act of Authority*): uses `Effect::GainControlEx { target: TargetSpec::ThisObject, .. }`
    /// so `TargetSpec::ThisObject` transfers the source permanent even when the
    /// enclosing ability also targeted an artifact or enchantment (`CR 608.2h`).
    pub(super) fn wave27c_gain_control_self_remembered(
        &self,
        p: &mut Params,
    ) -> Option<Vec<String>> {
        if self.on_a_spell
            || !self.capture_scope.as_ref().is_some_and(|s| s.single_object)
            || p.peek("Defined") != Some("Self")
            || p.peek("NewController") != Some("RememberedController")
            || p.has("AllValid")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Defined");
        probe.take("NewController");
        let (duration, untap, haste_until_eot) = self.gain_control_ex_riders(&mut probe)?;
        *p = probe;
        Some(vec![format!(
            "Effect::GainControlEx {{ target: TargetSpec::ThisObject, \
             to: PlayerRel::RememberedController, duration: {duration}, \
             untap: {untap}, haste_until_eot: {haste_until_eot} }}"
        )])
    }

    /// `GainControl` `NewController$` resolution for Wave 27c:
    /// - `RememberedController` inside a single-object capture (`Act of Authority`),
    /// - `ParentTarget` on a 2nd-target `GainControl` (`Bazaar Trader`) -> `FirstTargetPlayerOrController`.
    pub(super) fn wave27c_gain_control_new_controller(
        &self,
        raw: &str,
        allow_parent_target_first_player: bool,
    ) -> Option<&'static str> {
        if raw == "RememberedController"
            && self.capture_scope.as_ref().is_some_and(|s| s.single_object)
        {
            return Some("PlayerRel::RememberedController");
        }
        if allow_parent_target_first_player && raw == "ParentTarget" {
            return Some("PlayerRel::FirstTargetPlayerOrController");
        }
        None
    }

    /// Extra `ChangeZoneAll` cases needed by Wave 27c:
    /// 1. `Origin$ Hand | Destination$ Library | LibraryPosition$ -1` (*Arjun, the Shifting Flame*):
    ///    puts the player's hand onto the bottom of their library in chosen (or random) order.
    /// 2. Inside `CaptureAffected::then`: `Origin$ Graveyard,Exile` (or `Exile` / `Graveyard`)
    ///    with `ChangeType$ <F>.IsRemembered` moving all matching remembered cards to `Hand`,
    ///    `Battlefield`, `Exile`, or `Graveyard` via `Effect::ForEachRemembered`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave27c_change_zone_all_extra(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        // 1. Hand -> Library bottom (Arjun, the Shifting Flame, etc.)
        if p.peek("Origin") == Some("Hand")
            && p.peek("Destination") == Some("Library")
            && p.peek("LibraryPosition") == Some("-1")
            && !p.has("Shuffle")
        {
            let mut probe = p.clone();
            probe.take("Origin");
            probe.take("Destination");
            probe.take("LibraryPosition");
            probe.take("IsCurse");
            let random = match probe.take("RandomOrder").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let defined = probe.take("Defined");
            let change_type = probe.take("ChangeType");
            let no_open_target = target.is_none() || prior_target_used;
            let who = match change_type.as_deref() {
                Some("Card.YouOwn" | "Card.YouCtrl") if defined.is_none() && no_open_target => {
                    "PlayerRel::You"
                }
                Some("Card") | None if defined.is_some() || targets_a_player => {
                    let rel =
                        self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)?;
                    if !Self::is_single_player_rel(rel) {
                        return None;
                    }
                    rel
                }
                _ => return None,
            };
            if !probe.exhausted() {
                return None;
            }
            *p = probe;
            let bottom_move = if random {
                "ChooseMove::BottomOfLibraryRandom"
            } else {
                "ChooseMove::BottomOfLibraryChosenOrder"
            };
            return Some(vec![format!(
                "Effect::ChooseCardEx {{ chooser: {who}, zone: ChooseZone::Hand({who}), \
                 filter: &Filter::Any, min: 0, max: Amount::Fixed(0), reveal_chosen: false, \
                 movement: ChooseMove::None, not_chosen_movement: {bottom_move}, \
                 then: &[], for_unchosen: &[] }}"
            )]);
        }

        // 2. Inside CaptureAffected::then: `ChangeZoneAll` moving `<F>.IsRemembered`
        //    from `Exile` -> `Graveyard` / `Hand` / `Library (-1, RandomOrder)` (with
        //    optional `ForgetChanged$ True`), e.g. *Epic Experiment*, *Collected
        //    Conjuring*, *Muse Vortex*.
        if let Some(effs) = self.wave28d_capture_change_zone_all_exile(p) {
            return Some(effs);
        }

        // 3. Inside CaptureAffected::then: `ChangeZoneAll` moving `<F>.IsRemembered`
        //    from `Graveyard,Exile` / `Exile` / `Graveyard` -> `Hand` / `Battlefield` / `Exile`.
        let scope = self.capture_scope.as_ref()?;
        if targets_a_player {
            return None;
        }
        let _ = (target, prior_target_used);
        let origin = p.peek("Origin")?;
        let dest = p.peek("Destination")?;
        let change_type = p.peek("ChangeType")?;
        if !matches!(
            origin,
            "Graveyard,Exile" | "Exile,Graveyard" | "Exile" | "Graveyard"
        ) || !matches!(dest, "Hand" | "Battlefield" | "Exile")
        {
            return None;
        }
        if !change_type.contains("IsRemembered") || change_type.contains("!IsRemembered") {
            return None;
        }
        let stripped = Self::strip_is_remembered_all_alts(change_type)?;
        let mut probe = p.clone();
        probe.take("Origin");
        probe.take("Destination");
        probe.take("ChangeType");
        probe.take("IsCurse");
        if probe.peek("Defined").is_some() {
            return None;
        }
        let inner_effect = match dest {
            "Hand" => "Effect::ReturnToHand { target: TargetSpec::EventObject }".to_string(),
            "Exile" => "Effect::exile(TargetSpec::EventObject)".to_string(),
            "Battlefield" => {
                let tapped = match probe.take("Tapped").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let gain = match probe.take("GainControl").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let owner_control = !gain && !scope.you_own_only;
                let origin_ref = match origin {
                    "Graveyard" => "Some(ZoneRef::Graveyard)",
                    "Exile" => "Some(ZoneRef::Exile)",
                    _ => "None",
                };
                format!(
                    "Effect::MoveAndModify {{ subject: MoveSubject::Target(TargetSpec::EventObject), \
                     expected_origin: {origin_ref}, blink: false, owner_control: {owner_control}, \
                     tapped: {tapped}, face: None, counters: None, modifiers: &[], \
                     duration: Duration::Indefinitely, attach_source: false, \
                     when_source_leaves: None }}"
                )
            }
            _ => return None,
        };
        if !Self::only_condition_keys_left(&probe) {
            return None;
        }
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let filter_name = self.body.filter_static("REMEMBERED", &expr);
        *p = probe;
        Some(vec![format!(
            "Effect::ForEachRemembered {{ filter: &{filter_name}, effects: &[{inner_effect}] }}"
        )])
    }

    /// Which Forge `Remember*` parameter on `api` cleanly maps to
    /// `Effect::CaptureAffected { first, then }`.
    pub(super) fn wave27c_capture_flag(api: &str, p: &Params) -> Option<&'static str> {
        let primary = match api {
            "ChangeZone" | "ChangeZoneAll" | "Dig" => "RememberChanged",
            "Mill" => "RememberMilled",
            "Sacrifice" | "SacrificeAll" => "RememberSacrificed",
            "Destroy" | "DestroyAll" => "RememberDestroyed",
            "Token" | "CopyPermanent" => "RememberTokens",
            "Discard" => "RememberDiscarded",
            "Draw" => "RememberDrawn",
            "ChooseCard" => "RememberChosen",
            "Reveal" => "RememberRevealed",
            _ => return None,
        };
        if p.peek(primary) == Some("True") {
            return Some(primary);
        }
        if api == "ChangeZone"
            && p.peek("RememberTargets") == Some("True")
            && !p.has("RememberChanged")
            && !p.has("Optional")
        {
            return Some("RememberTargets");
        }
        if matches!(
            api,
            "ChangeZone"
                | "ChangeZoneAll"
                | "Destroy"
                | "DestroyAll"
                | "Sacrifice"
                | "SacrificeAll"
        ) && p.peek("RememberLKI") == Some("True")
        {
            return Some("RememberLKI");
        }
        None
    }

    /// True if `p` carries any persistent/unsupported memory parameter that
    /// `CaptureAffected` cannot faithfully represent.
    fn has_unsupported_memory_param(p: &Params, allowed_key: &str) -> bool {
        p.entries.iter().any(|(k, _)| {
            k.as_str() != allowed_key
                && k.as_str() != "ForgetOtherRemembered"
                && (k.starts_with("Remember") || k.starts_with("Forget"))
        })
    }

    /// Derive the [`CaptureScope`] guaranteed by a `first` step.
    #[allow(clippy::too_many_lines)]
    fn compute_capture_scope(
        api: &str,
        p: &Params,
        target: Option<&str>,
        flags: LineFlags,
        chain: &Chain,
    ) -> CaptureScope {
        let from_battlefield = matches!(
            api,
            "Sacrifice" | "SacrificeAll" | "Destroy" | "DestroyAll" | "Token" | "CopyPermanent"
        ) || (matches!(api, "ChangeZone" | "ChangeZoneAll")
            && p.peek("Origin") == Some("Battlefield"));

        let you_own_only = match api {
            "Mill" | "Draw" | "Discard" => {
                !flags.targets_a_player && matches!(p.peek("Defined"), None | Some("You"))
            }
            "Dig" => !flags.targets_a_player && matches!(p.peek("Defined"), None | Some("You")),
            "Token" | "CopyPermanent" => {
                matches!(
                    p.peek("TokenOwner").or_else(|| p.peek("Controller")),
                    None | Some("You")
                )
            }
            "ChangeZone" => {
                p.peek("Origin") == Some("Library")
                    && !flags.targets_a_player
                    && matches!(
                        p.peek("Defined").or_else(|| p.peek("DefinedPlayer")),
                        None | Some("You")
                    )
            }
            "ChangeZoneAll" => {
                !flags.targets_a_player
                    && p.peek("ChangeType")
                        .is_some_and(|ct| ct.contains("YouOwn") || ct.contains("YouCtrl"))
            }
            _ => false,
        };

        let single_object = match api {
            "Destroy" => {
                (flags.targets_here && flags.single_target && chain.third_target.is_none())
                    || p.peek("Defined") == Some("Self")
            }
            "ChangeZone" => {
                if flags.targets_here {
                    flags.single_target && chain.third_target.is_none()
                } else if p.peek("Defined").is_some_and(|d| d == "Self") {
                    true
                } else {
                    matches!(p.peek("ChangeNum"), None | Some("1"))
                }
            }
            "Dig" => {
                matches!(p.peek("ChangeNum"), None | Some("1"))
                    || (p.peek("ChangeNum") == Some("All") && p.peek("DigNum") == Some("1"))
            }
            "Mill" => {
                !flags.targets_a_player
                    && matches!(p.peek("Defined"), None | Some("You"))
                    && p.peek("NumCards") == Some("1")
            }
            "Sacrifice" => {
                matches!(p.peek("Amount"), None | Some("1"))
                    && (flags.targets_a_player
                        || matches!(
                            p.peek("Defined"),
                            None | Some(
                                "You" | "TriggeredPlayer" | "EnchantedPlayer" | "ActivePlayer"
                            )
                        ))
            }
            "Token" => matches!(p.peek("TokenAmount"), None | Some("1")),
            "Discard" => {
                matches!(p.peek("NumCards"), None | Some("1"))
                    && (flags.targets_a_player || matches!(p.peek("Defined"), None | Some("You")))
            }
            "Draw" => {
                matches!(p.peek("NumCards"), None | Some("1"))
                    && (flags.targets_a_player || matches!(p.peek("Defined"), None | Some("You")))
            }
            "ChooseCard" => matches!(p.peek("Amount"), None | Some("1")),
            _ => false,
        };

        let guaranteed_valid = match api {
            "Sacrifice" | "SacrificeAll" => p.peek("SacValid").or_else(|| p.peek("ValidCards")),
            "Dig" => p.peek("ChangeValid"),
            "ChangeZone" | "ChangeZoneAll" => p.peek("ChangeType"),
            "ChooseCard" => p.peek("Choices"),
            "Destroy" | "DestroyAll" => p.peek("ValidCards"),
            _ => None,
        }
        .map(str::trim)
        .filter(|v| !v.is_empty() && !v.contains(',') && !v.contains(' '))
        .map(str::to_string)
        .or_else(|| {
            if target == Some("TargetSpec::Object(&Filter::CREATURE)") {
                Some("Creature".to_string())
            } else if target == Some("TargetSpec::Object(&Filter::ARTIFACT)") {
                Some("Artifact".to_string())
            } else if target == Some("TargetSpec::Object(&Filter::ENCHANTMENT)") {
                Some("Enchantment".to_string())
            } else if target == Some("TargetSpec::Object(&Filter::LAND)") {
                Some("Land".to_string())
            } else if target == Some("TargetSpec::Object(&Filter::PLANESWALKER)") {
                Some("Planeswalker".to_string())
            } else {
                None
            }
        });

        let multi_player = matches!(
            p.peek("Defined"),
            Some("Player" | "Opponent" | "Player.Opponent" | "Player.Other")
        ) || (api == "SacrificeAll"
            && p.peek("ValidCards")
                .is_some_and(|v| !v.contains("YouCtrl") && !v.contains("YouOwn")))
            || (api == "ChangeZoneAll"
                && !flags.targets_a_player
                && p.peek("Defined").is_none()
                && p.peek("ChangeType")
                    .is_some_and(|ct| !ct.contains("YouOwn") && !ct.contains("YouCtrl")));

        CaptureScope {
            single_object,
            guaranteed_valid,
            from_battlefield,
            you_own_only,
            multi_player,
        }
    }

    /// True if every object remembered in `scope` is guaranteed to satisfy
    /// Forge's `ConditionPresent$` / `Remembered$Valid` string `present`.
    fn scope_guarantees_present(scope: &CaptureScope, present: &str) -> bool {
        let present = present.trim();
        if matches!(present, "Card" | "Card.All") {
            return true;
        }
        if present == "Card.YouOwn" && scope.you_own_only {
            return true;
        }
        if matches!(present, "Permanent" | "Card.Permanent") && scope.from_battlefield {
            return true;
        }
        if let Some(guaranteed) = scope.guaranteed_valid.as_deref() {
            if present == guaranteed {
                return true;
            }
            if let Some(rest) = present.strip_prefix("Card.")
                && rest == guaranteed
            {
                return true;
            }
            if let Some(rest) = guaranteed.strip_prefix("Card.")
                && rest == present
            {
                return true;
            }
            // If guaranteed is `Creature.StrictlyOther` or `Creature.!token`,
            // its base type `Creature` is guaranteed.
            if let Some((base, _)) = guaranteed.split_once('.')
                && (present == base || present.strip_prefix("Card.") == Some(base))
            {
                return true;
            }
        }
        false
    }

    /// Speculative wrapper around [`Self::wave27c_capture_affected_chain`] that
    /// yields to existing trigger-level `wave24e` / `wave26f` handlers and rolls
    /// back all mutable state cleanly on failure.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn wave27c_try_capture_affected_chain(
        &mut self,
        spec: &str,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        flags: LineFlags,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        // A nested hand-to-battlefield choice followed by Animate/Pump already has
        // a dedicated reader for the whole chain, including its delayed zone move.
        // Let that reader consume the SubAbility chain instead of lowering only the
        // choice here and then processing its follow-up a second time.
        if self.capture_scope.is_some()
            && api == "ChangeZone"
            && p.peek("Origin") == Some("Hand")
            && p.peek("Destination") == Some("Battlefield")
            && (self.sub_is_api(sub.as_deref(), "Animate")
                || self.sub_is_api(sub.as_deref(), "Pump"))
        {
            return None;
        }
        if api == "Dig"
            && self.capture_scope.is_none()
            && self.sub_is_api(sub.as_deref(), "TwoPiles")
            && let Some(effs) = self.wave28d_dig_exile_two_piles_play(p, sub)
        {
            return Some(effs);
        }
        // Yield to existing specialized trigger handlers (`wave24e` Dig triggers
        // and `wave26f` Token+RememberTokens triggers) so existing generated cards
        // keep their exact replay identities.
        if self.trigger_mode.is_some()
            && chain.effects.is_empty()
            && self.capture_scope.is_none()
            && matches!(api, "Dig" | "Token")
        {
            let saved_statics = self.body.statics.len();
            let saved_unclaimed = self.unclaimed.borrow().clone();
            let mut probe_chain = Chain::default();
            let existing_handles = if api == "Dig" {
                self.wave24e_dig_trigger_chain(spec, &mut probe_chain)
                    .is_some()
            } else {
                self.wave26f_token_found_trigger_chain(spec, &mut probe_chain)
                    .is_some()
            };
            self.body.statics.truncate(saved_statics);
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            if existing_handles {
                return None;
            }
        }
        let saved_p = p.clone();
        let saved_sub = sub.clone();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        if let Some(effects) =
            self.wave27c_capture_affected_chain(api, p, sub, target, flags, chain, None)
        {
            Some(effects)
        } else {
            *p = saved_p;
            *sub = saved_sub;
            *self.unclaimed.borrow_mut() = saved_unclaimed;
            self.body.statics.truncate(saved_statics_len);
            None
        }
    }

    /// General Wave 27c fallback: lower any supported `Remember*` step + its
    /// follow-up `SubAbility$` chain into `Effect::CaptureAffected { first, then }`.
    ///
    /// Returns `Some(effects)` and updates `*sub` to any post-`Cleanup`
    /// continuation; returns `None` without mutating caller state on failure.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(super) fn wave27c_capture_affected_chain(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        flags: LineFlags,
        chain: &mut Chain,
        line_first_override: Option<Vec<String>>,
    ) -> Option<Vec<String>> {
        let sub_name = sub.as_deref()?.trim().to_string();
        let flag_key = Self::wave27c_capture_flag(api, p)?;
        if flag_key == "RememberTargets"
            && (!flags.targets_here
                || !flags.single_target
                || self.has_linked_exile_reader()
                || self.oracle.to_ascii_lowercase().contains("you may "))
        {
            return None;
        }
        if Self::has_unsupported_memory_param(p, flag_key) {
            return None;
        }
        // Nested capture inside an already-open CaptureAffected::then MUST carry
        // `ForgetOtherRemembered$ True` so it matches CaptureAffected's shadow semantics
        // (and never silently drops append-to-remembered semantics).
        let is_nested = self.capture_scope.is_some();
        let has_forget_other = p.peek("ForgetOtherRemembered") == Some("True");
        if p.has("ForgetOtherRemembered") && !has_forget_other {
            return None;
        }
        if is_nested && !has_forget_other {
            return None;
        }
        // Never allow persistent Effect / DelayedTrigger in `then` to leak memory
        // beyond resolution scope.
        let mut probe_p = p.clone();
        probe_p.take(flag_key);
        if has_forget_other {
            probe_p.take("ForgetOtherRemembered");
        }
        let scope = Self::compute_capture_scope(api, &probe_p, target, flags, chain);

        // Lower `first` without `flag_key` (keeping outer capture_scope active
        // while lowering `first` so nested steps like `ChangeZone | ChangeType$ Land.IsRemembered`
        // can still read the outer remembered set!).
        let first_effects = if let Some(override_effs) = line_first_override {
            override_effs
        } else {
            self.wave27c_lower_capture_first(api, &mut probe_p, target, flags)?
        };
        let allow_line_cond = api == "Sacrifice"
            && probe_p.peek("ConditionPresent").is_some()
            && Self::only_condition_keys_left(&probe_p);
        if (!probe_p.exhausted() && !allow_line_cond) || first_effects.is_empty() {
            return None;
        }

        // Now lower `then` inside `CaptureScope`, stopping at `Cleanup | ClearRemembered$ True`.
        let saved_scope = self.capture_scope.replace(scope);
        let mut then_chain = Chain {
            target: chain.target.clone(),
            target_is_x: chain.target_is_x,
            target_up_to: chain.target_up_to,
            target_exact: chain.target_exact,
            target_range: chain.target_range,
            target_unique: chain.target_unique,
            target_constraint: chain.target_constraint,
            second_target: chain.second_target.clone(),
            second_target_up_to: chain.second_target_up_to,
            second_target_range: chain.second_target_range,
            second_target_constraint: chain.second_target_constraint,
            third_target: chain.third_target.clone(),
            third_target_up_to: chain.third_target_up_to,
            third_target_range: chain.third_target_range,
            third_target_constraint: chain.third_target_constraint,
            ..Chain::default()
        };
        let mut visited: BTreeSet<String> = BTreeSet::new();
        let walked = self.wave27c_walk_capture_then(&sub_name, &mut then_chain, &mut visited);
        self.capture_scope = saved_scope;
        let (closed_by_cleanup, after_cleanup) = walked?;

        if then_chain.effects.is_empty() {
            return None;
        }
        // When `first` carried a line condition (e.g. `Daredevil Dragster`:
        // `Sacrifice | SacValid$ Self | RememberSacrificed$ True | ConditionPresent$ ...`),
        // wrapping `CaptureAffected` in `IfCondition` is equivalent ONLY when `then`
        // itself requires at least one remembered object (`RememberedCount >= 1`).
        if allow_line_cond
            && !then_chain.effects.iter().all(|e| {
                e.starts_with(
                    "Effect::IfCondition { condition: Condition::AmountCompare { \
                     left: &Amount::RememberedCount, op: AmountCmp::AtLeast, \
                     right: &Amount::Fixed(1) }",
                )
            })
        {
            return None;
        }
        // On permanent abilities (triggers / activated abilities), omitting
        // `DB$ Cleanup | ClearRemembered$ True` would retain remembered objects
        // across separate resolutions in Forge; require explicit `Cleanup`.
        // On spells (which leave the stack on resolution), allow omitted
        // `Cleanup` only when no unvisited SVar reads `Remembered`.
        if !closed_by_cleanup
            && ((!self.on_a_spell && !self.preceded_by_cleanup)
                || self.wave27c_has_unvisited_remembered(&visited))
        {
            return None;
        }

        // Propagate any targets or flags acquired in `then` back into `chain`.
        chain.target = then_chain.target;
        chain.target_is_x = then_chain.target_is_x;
        chain.target_up_to = then_chain.target_up_to;
        chain.target_exact = then_chain.target_exact;
        chain.target_range = then_chain.target_range;
        chain.target_unique = then_chain.target_unique;
        chain.target_constraint = then_chain.target_constraint;
        chain.second_target = then_chain.second_target;
        chain.second_target_up_to = then_chain.second_target_up_to;
        chain.second_target_range = then_chain.second_target_range;
        chain.second_target_constraint = then_chain.second_target_constraint;
        chain.third_target = then_chain.third_target;
        chain.third_target_up_to = then_chain.third_target_up_to;
        chain.third_target_range = then_chain.third_target_range;
        chain.third_target_constraint = then_chain.third_target_constraint;
        chain.could_add_mana |= then_chain.could_add_mana;
        chain.moves_library |= then_chain.moves_library;

        *p = probe_p;
        *sub = after_cleanup;
        // If `first_effects` starts with `Effect::RevealHand { .. }` before a
        // discard (`Traumatic Revelation`), hoist `RevealHand` OUTSIDE
        // `CaptureAffected::first` so `harvest_captured` never falls back to
        // harvesting the revealed hand cards as "discarded" when 0 cards are discarded.
        if api == "Discard"
            && first_effects.len() >= 2
            && first_effects[0].starts_with("Effect::RevealHand ")
        {
            let reveal = first_effects[0].clone();
            let rest_first = first_effects[1..].join(", ");
            return Some(vec![
                reveal,
                format!(
                    "Effect::CaptureAffected {{ first: &[{rest_first}], then: &[{}] }}",
                    then_chain.effects.join(", "),
                ),
            ]);
        }
        if api == "Discard"
            && first_effects
                .iter()
                .any(|e| e.starts_with("Effect::RevealHand"))
        {
            return None;
        }
        Some(vec![format!(
            "Effect::CaptureAffected {{ first: &[{}], then: &[{}] }}",
            first_effects.join(", "),
            then_chain.effects.join(", "),
        )])
    }

    /// Lower the `first` instruction of `Effect::CaptureAffected`.
    fn wave27c_lower_capture_first(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        // Inside an outer capture, a nested `ChangeZone | Hidden$ True | ChangeType$ <F>.IsRemembered`
        // (Ainok Wayfarer, Airlift Chaplain, Cache Grab) lowers to `ChooseCard { zone: Remembered }`.
        if self.capture_scope.is_some()
            && let Some(effs) = self.wave27c_remembered_step(api, p, target, flags)
        {
            return Some(effs);
        }
        // `Sacrifice | Optional$ True | ...` or `Sacrifice | StrictAmount$ True | Optional$ True`:
        // Never fall through to `sacrifice_effect` when `Optional$ True` is present,
        // because dynamic `Amount$ SacX` with mismatched filter must stay refused!
        if api == "Sacrifice" && p.peek("Optional") == Some("True") {
            let mut probe = p.clone();
            if let Some(effs) = self.wave27c_optional_sacrifice_first(&mut probe, target, flags)
                && probe.exhausted()
            {
                *p = probe;
                return Some(effs);
            }
            return None;
        }
        // `Sacrifice | SacValid$ Self` (without `Optional$ True`, e.g. *Daredevil Dragster*):
        // lower cleanly to `Effect::SacrificeSelf` even when line conditions remain on `p`.
        if api == "Sacrifice"
            && !self.on_a_spell
            && p.peek("SacValid") == Some("Self")
            && !p.has("Defined")
            && !p.has("Amount")
            && !p.has("Optional")
        {
            p.take("SacValid");
            p.take("SacMessage");
            return Some(vec!["Effect::SacrificeSelf".to_string()]);
        }
        // `Dig` with `ChangeNum$ Any` (Search for Blex): let `dig_effect` handle standard Dig first.
        if api == "Dig" {
            let mut probe = p.clone();
            if let Some(effs) = self.wave27c_dig_first(&mut probe, target, flags)
                && probe.exhausted()
            {
                *p = probe;
                return Some(effs);
            }
        }
        // `ChangeZone | Optional$ True | Origin$ Library | ShuffleNonMandatory$ True` (Courier of Comestibles)
        if api == "ChangeZone" && p.peek("Origin") == Some("Library") {
            let mut probe = p.clone();
            if probe.peek("ShuffleNonMandatory") == Some("True") {
                probe.take("ShuffleNonMandatory");
            }
            if let Some(effs) = self.effect_of_inner(api, &mut probe, target, flags)
                && probe.exhausted()
            {
                *p = probe;
                return Some(effs);
            }
        }
        self.effect_of_inner(api, p, target, flags)
    }

    /// Lower `Sacrifice | Optional$ True` (including `StrictAmount$ True` for
    /// exact-N optional sacrifice like *Giant Opportunity*, and single-opponent
    /// optional sacrifice like *Desecration Demon* when supported) for `CaptureAffected::first`.
    fn wave27c_optional_sacrifice_first(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        if p.take("Optional").as_deref() != Some("True") {
            return None;
        }
        p.take("SacMessage");
        let strict = match p.take("StrictAmount").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let amount_raw = p.take("Amount").unwrap_or_else(|| "1".to_string());
        let valid = p.take("SacValid")?;
        if valid == "Self" {
            if amount_raw.trim() != "1" || p.has("Defined") {
                return None;
            }
            return Some(vec![
                "Effect::MayDo { effects: &[Effect::SacrificeSelf] }".to_string(),
            ]);
        }
        let defined = p.take("Defined");
        let who = self.wave11_player_of_line(defined.as_deref(), target, flags.targets_a_player)?;
        if !matches!(
            who,
            "PlayerRel::You"
                | "PlayerRel::Chosen"
                | "PlayerRel::ActivePlayer"
                | "PlayerRel::EnchantedPlayer"
                | "PlayerRel::EventPlayer"
                | "PlayerRel::ControllerOfEvent"
                | "PlayerRel::ControllerOfTarget"
        ) {
            return None;
        }
        let filter_expr = self.filter_expr(&valid)?;
        let filter_name = self.body.filter_static("SAC", &filter_expr);
        if !strict && self.is_sacrifice_any_number(&amount_raw, &valid, who) {
            return Some(vec![format!(
                "Effect::SacrificeAnyNumber {{ who: {who}, filter: &{filter_name} }}"
            )]);
        }
        let amount: u8 = amount_raw.trim().parse().ok()?;
        if amount == 0 {
            return None;
        }
        if amount == 1 {
            let inner = format!("Effect::SacrificeFilter {{ who: {who}, filter: &{filter_name} }}");
            if who == "PlayerRel::You" {
                return Some(vec![format!("Effect::MayDo {{ effects: &[{inner}] }}")]);
            }
            return Some(vec![format!(
                "Effect::MayDoFor {{ player: {who}, effects: &[{inner}] }}"
            )]);
        }
        if strict
            && who == "PlayerRel::You"
            && !filter_expr.contains("Filter::Another")
            && !filter_expr.contains("Filter::This")
        {
            // "You may sacrifice N <F>s": exact-N optional sacrifice guarded by
            // ControlCount so a player with fewer than N cannot partially sacrifice.
            let cond = format!("Condition::ControlCount(&{filter_name}, {amount})");
            let sac_n = format!(
                "Effect::SacrificeAmountForPlayers {{ who: PlayerRel::You, filter: &{filter_name}, \
                 amount: Amount::Fixed({amount}) }}"
            );
            return Some(vec![format!(
                "Effect::IfCondition {{ condition: {cond}, \
                 then: &[Effect::MayDo {{ effects: &[{sac_n}] }}], otherwise: &[] }}"
            )]);
        }
        None
    }

    /// Lower `Dig` variants for `CaptureAffected::first` (including `Dig | ChangeNum$ Any |
    /// DestinationZone$ Hand | DestinationZone2$ Graveyard` like *Search for Blex*).
    fn wave27c_dig_first(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        // First try the existing `effect_of_inner("Dig", ...)` directly.
        {
            let mut probe = p.clone();
            if let Some(effs) = self.effect_of_inner("Dig", &mut probe, target, flags)
                && probe.exhausted()
            {
                *p = probe;
                return Some(effs);
            }
        }
        // `Dig | DigNum$ N | ChangeNum$ Any | DestinationZone$ Hand | DestinationZone2$ Graveyard`
        if p.peek("ChangeNum") == Some("Any")
            && matches!(p.peek("DestinationZone"), None | Some("Hand"))
            && p.peek("DestinationZone2") == Some("Graveyard")
            && !flags.targets_here
            && matches!(p.peek("Defined"), None | Some("You"))
        {
            let mut probe = p.clone();
            probe.take("Defined");
            probe.take("ChangeNum");
            probe.take("DestinationZone");
            probe.take("DestinationZone2");
            let reveal_all = match probe.take("Reveal").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let count_raw = probe.take("DigNum")?;
            let count: u8 = count_raw.trim().parse().ok()?;
            let filter = match probe.take("ChangeValid") {
                None => "&Filter::Any".to_string(),
                Some(v) => {
                    let expr = self.filter_expr_in_zone(&v, true)?;
                    format!("&{}", self.body.filter_static("DIG", &expr))
                }
            };
            if !probe.exhausted() {
                return None;
            }
            *p = probe;
            return Some(vec![format!(
                "Effect::PeekAndReveal {{ library: PlayerRel::You, count: Amount::Fixed({count}), \
                 reveal_all: {reveal_all}, reveal_chosen: false, filter: {filter}, \
                 min_pick: 0, max_pick: {count}, picked: PeekPickDest::Hand, \
                 rest: PeekRestDest::Graveyard, if_picked: &[] }}"
            )]);
        }
        None
    }

    /// Walk `then` sub-abilities inside an open `CaptureAffected` scope until
    /// `Cleanup | ClearRemembered$ True` (returning `(true, after_cleanup)`) or
    /// end-of-chain (returning `(false, None)`).
    fn wave27c_walk_capture_then(
        &mut self,
        start_svar: &str,
        then_chain: &mut Chain,
        visited: &mut BTreeSet<String>,
    ) -> Option<(bool, Option<String>)> {
        let mut cursor = Some(start_svar.trim().to_string());
        while let Some(name) = cursor.take() {
            if !visited.insert(name.clone()) {
                return None;
            }
            let body = self.svars.get(&name)?.clone();
            let (api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            // Stop cleanly at `DB$ Cleanup | ClearRemembered$ True`!
            if api == "Cleanup" && sp.peek("ClearRemembered") == Some("True") {
                sp.take("ClearRemembered");
                let after = sp.take("SubAbility");
                if !sp.exhausted() {
                    // Never silently drop `ClearImprinted$`, `ClearChosenCard$`, etc.
                    return None;
                }
                return Some((true, after));
            }
            // Never allow persistent `Effect` or `DelayedTrigger` reading Remembered
            // inside resolution-scoped CaptureAffected.
            if matches!(api.as_str(), "Effect" | "DelayedTrigger")
                && (sp.has("RememberObjects")
                    || sp.has("ForgetOnMoved")
                    || body.contains("Remembered"))
            {
                return None;
            }
            // If this step itself starts a nested `CaptureAffected` (requires
            // `ForgetOtherRemembered$ True`), let `chain_step` handle the whole
            // nested sub-chain including its own `Cleanup`!
            if Self::wave27c_capture_flag(&api, &sp).is_some()
                && sp.peek("ForgetOtherRemembered") == Some("True")
                && sp.has("SubAbility")
            {
                self.chain(&body, then_chain)?;
                return Some((true, None));
            }
            // Refuse multi-player follow-up steps (`Defined$ Player|Opponent`) that
            // read `Remembered$Amount` when `first` was itself multi-player (e.g.
            // Wheel/Whirl effects where each player draws their *own* count, not the
            // combined count across all players).
            if self.capture_scope.as_ref().is_some_and(|s| s.multi_player)
                && matches!(
                    sp.peek("Defined"),
                    Some("Player" | "Opponent" | "Player.Opponent" | "Player.Other")
                )
                && self.step_reads_remembered_count(&sp)
            {
                return None;
            }
            // Strip `SubAbility$` from this line so we advance step-by-step and
            // detect `DB$ Cleanup | ClearRemembered$ True` cleanly.
            let next_sub = sp.peek("SubAbility").map(str::to_string);
            let step_spec = if next_sub.is_some() {
                Self::strip_param_key(&body, "SubAbility")
            } else {
                body
            };
            self.chain(&step_spec, then_chain)?;
            cursor = next_sub;
        }
        Some((false, None))
    }

    /// True if any numeric/condition parameter on `sp` (or an `SVar` it names)
    /// reads `Remembered$Amount` or `Count$RememberedSize`.
    fn step_reads_remembered_count(&self, sp: &Params) -> bool {
        sp.entries.iter().any(|(_, v)| {
            let trimmed = v.trim();
            if trimmed.contains("Remembered$Amount") || trimmed.contains("Count$RememberedSize") {
                return true;
            }
            self.svars.get(trimmed).is_some_and(|def| {
                def.contains("Remembered$Amount") || def.contains("Count$RememberedSize")
            })
        })
    }

    /// Remove a single `Key$ Value` segment from a Forge ability line.
    fn strip_param_key(spec: &str, key: &str) -> String {
        let mut parts = spec.split(" | ");
        let Some(head) = parts.next() else {
            return spec.to_string();
        };
        let mut out = vec![head.trim()];
        for part in parts {
            if let Some((k, _)) = part.split_once('$')
                && k.trim() == key
            {
                continue;
            }
            out.push(part.trim());
        }
        out.join(" | ")
    }

    /// True if any unvisited `SVar` on this card references `Remembered` or
    /// `IsRemembered` (excluding `AI*` hints and `Remembered$Amount`-only `SVars`
    /// that were read as numeric amounts).
    fn wave27c_has_unvisited_remembered(&self, visited: &BTreeSet<String>) -> bool {
        self.svars.iter().any(|(name, body)| {
            if visited.contains(name) || name.starts_with("AI") {
                return false;
            }
            let b = body.trim();
            // Numeric SVars like `SVar:X:Remembered$Amount` are consumed by value,
            // not as sub-abilities.
            if b.starts_with("Remembered$")
                || b.starts_with("RememberedLKI$")
                || b.starts_with("Count$RememberedSize")
            {
                return false;
            }
            b.contains("IsRemembered")
                || b.contains("Defined$ Remembered")
                || b.contains("RememberObjects$ Remembered")
        })
    }

    /// Strip `IsRemembered` from every comma-separated alternative in `valid`.
    /// Returns `None` if ANY alternative lacks `IsRemembered` or uses `!IsRemembered`.
    pub(super) fn strip_is_remembered_all_alts(valid: &str) -> Option<String> {
        let mut out = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() || alt.contains("!IsRemembered") {
                return None;
            }
            let stripped = Self::strip_filter_attr(alt, "IsRemembered")?;
            out.push(stripped);
        }
        (!out.is_empty()).then(|| out.join(","))
    }

    /// True when `probe` has no unclaimed parameters other than line-level
    /// `Condition*` / `Present*` keys that `wrap_line_condition` claims next.
    fn only_condition_keys_left(p: &Params) -> bool {
        p.entries
            .iter()
            .all(|(k, _)| k.starts_with("Condition") || k.starts_with("Present"))
    }

    /// Lower Wave 27c `Remembered`-referencing steps inside `CaptureAffected::then`:
    /// 1. `ChangeZone | Hidden$ True | Origin$ Graveyard[,Exile] | ChangeType$ <F>.IsRemembered`
    ///    -> `Effect::ChooseCard { zone: ChooseZone::Remembered, ... }`.
    /// 2. `ChooseCard | Choices$ <F>.IsRemembered` -> `Effect::ChooseCard { zone: ChooseZone::Remembered, ... }`.
    /// 3. `Defined$ Remembered` on `Untap`, `Tap`, `Pump`, `PutCounter`, `Destroy`,
    ///    `ChangeZone`, `GainControl`, `Goad`, `Detain`, `Phases`, `Sacrifice`, `CopyPermanent`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave27c_remembered_step(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        flags: LineFlags,
    ) -> Option<Vec<String>> {
        let scope = self.capture_scope.as_ref()?.clone();
        if flags.targets_here {
            return None;
        }

        // Case 1: `ChangeZone | Hidden$ True | ChangeType$ <F>.IsRemembered` -> ChooseCard over Remembered!
        if api == "ChangeZone"
            && p.peek("Hidden") == Some("True")
            && p.peek("ChangeType")
                .is_some_and(|ct| ct.contains("IsRemembered") && !ct.contains("!IsRemembered"))
            && !p.has("Defined")
        {
            let mut probe = p.clone();
            probe.take("Hidden");
            probe.take("SelectPrompt");
            probe.take("ChangeTypeDesc");
            probe.take("IsCurse");
            let origin = probe.take("Origin")?;
            if !matches!(
                origin.as_str(),
                "Graveyard" | "Exile" | "Graveyard,Exile" | "Exile,Graveyard" | "Hand"
            ) {
                return None;
            }
            let dest = probe.take("Destination")?;
            let change_type = probe.take("ChangeType")?;
            let stripped = Self::strip_is_remembered_all_alts(&change_type)?;
            let mandatory = match probe.take("Mandatory").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let optional = match probe.take("Optional").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let max_raw = probe.take("ChangeNum").unwrap_or_else(|| "1".to_string());
            let max_u8: Option<u8> = max_raw.trim().parse().ok();
            let max_expr = if let Some(n) = max_u8 {
                format!("Amount::Fixed({n})")
            } else if !mandatory
                && self.wave28d_capture_hidden_all_change_num(&max_raw, &origin, &change_type)
            {
                "Amount::Fixed(255)".to_string()
            } else {
                self.amount_or_count_ext(&max_raw)?
            };
            let min = if mandatory && !optional { max_u8? } else { 0 };
            let chooser = match probe.take("DefinedPlayer").as_deref() {
                None | Some("You") => "PlayerRel::You",
                Some(other) => {
                    let rel = self.wave11_player_of_line(Some(other), target, false)?;
                    if !Self::is_single_player_rel(rel) {
                        return None;
                    }
                    rel
                }
            };
            let movement = match dest.as_str() {
                "Hand" => "ChooseMove::Hand".to_string(),
                "Exile" => "ChooseMove::Exile".to_string(),
                "Graveyard" => "ChooseMove::Graveyard".to_string(),
                "Battlefield" => {
                    let tapped = match probe.take("Tapped").as_deref() {
                        None | Some("False") => false,
                        Some("True") => true,
                        Some(_) => return None,
                    };
                    let gain = match probe.take("GainControl").as_deref() {
                        None | Some("False") => false,
                        Some("True") => true,
                        Some(_) => return None,
                    };
                    // `ChooseMove::Battlefield` enters under `you`'s control; only valid
                    // when `GainControl$ True` OR every remembered card is owned by `You`.
                    if !gain && !scope.you_own_only {
                        return None;
                    }
                    format!("ChooseMove::Battlefield {{ tapped: {tapped} }}")
                }
                "Library" => match probe.take("LibraryPosition").as_deref() {
                    None | Some("0") => "ChooseMove::TopOfLibrary".to_string(),
                    Some("-1") => match probe.take("RandomOrder").as_deref() {
                        Some("True") => "ChooseMove::BottomOfLibraryRandom".to_string(),
                        None | Some("False") => "ChooseMove::BottomOfLibrary".to_string(),
                        Some(_) => return None,
                    },
                    Some(_) => return None,
                },
                _ => return None,
            };
            if !Self::only_condition_keys_left(&probe) {
                return None;
            }
            let filter_expr = self.filter_expr_in_zone(&stripped, true)?;
            let filter_name = self.body.filter_static("CHOOSE", &filter_expr);
            *p = probe;
            return Some(vec![format!(
                "Effect::ChooseCard {{ chooser: {chooser}, zone: ChooseZone::Remembered, \
                 filter: &{filter_name}, min: {min}, max: {max_expr}, movement: {movement}, \
                 then: &[] }}"
            )]);
        }

        // Case 1b: `Play` over remembered cards in exile (*Etali, Primal Storm*,
        // *Villainous Wealth*, *Epic Experiment*, *Collected Conjuring*,
        // *Hellcarver Demon*, *Muse Vortex*, *Kotis, the Fangkeeper*).
        if api == "Play"
            && let Some(effs) = self.wave28d_capture_play_step(p, target, flags)
        {
            return Some(effs);
        }

        // Case 2: `Defined$ Remembered` on standard battlefield/zone operations.
        if p.peek("Defined") != Some("Remembered") {
            return None;
        }
        match api {
            "Untap" => {
                let mut probe = p.clone();
                probe.take("Defined");
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(vec![
                    "Effect::UntapAll { filter: &Filter::IsRemembered }".to_string(),
                ])
            }
            "Tap" => {
                let mut probe = p.clone();
                probe.take("Defined");
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(vec![
                    "Effect::TapAll { filter: &Filter::IsRemembered }".to_string(),
                ])
            }
            "Pump" => {
                // Delegate to `pump_all_effect` with `ValidCards$ Card.IsRemembered`!
                // Refuses `Duration$ Perpetual` and `PumpZone$` automatically.
                let mut probe = p.clone();
                probe.take("Defined");
                if probe.has("ValidCards") || probe.has("PumpZone") {
                    return None;
                }
                probe
                    .entries
                    .push(("ValidCards".to_string(), "Card.IsRemembered".to_string()));
                let effs = self.pump_all_effect(&mut probe, None, false, false, true)?;
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(effs)
            }
            "PutCounter" => {
                let mut probe = p.clone();
                probe.take("Defined");
                if probe.has("ValidCards") {
                    return None;
                }
                probe
                    .entries
                    .push(("ValidCards".to_string(), "Card.IsRemembered".to_string()));
                let effs = self.effect_of_inner(
                    "PutCounterAll",
                    &mut probe,
                    None,
                    LineFlags {
                        targets_here: false,
                        targets_a_player: false,
                        single_target: false,
                        prior_target_used: true,
                    },
                )?;
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(effs)
            }
            "Destroy" => {
                let mut probe = p.clone();
                probe.take("Defined");
                if probe.has("ValidCards") {
                    return None;
                }
                probe
                    .entries
                    .push(("ValidCards".to_string(), "Card.IsRemembered".to_string()));
                let effs = self.effect_of_inner(
                    "DestroyAll",
                    &mut probe,
                    None,
                    LineFlags {
                        targets_here: false,
                        targets_a_player: false,
                        single_target: false,
                        prior_target_used: true,
                    },
                )?;
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(effs)
            }
            "Sacrifice" => {
                let mut probe = p.clone();
                probe.take("Defined");
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(vec![
                    "Effect::SacrificeAll { filter: &Filter::IsRemembered }".to_string(),
                ])
            }
            "AlterAttribute" => Self::wave28d_capture_alter_attribute(p),
            "ChangeZone" => {
                let mut probe = p.clone();
                probe.take("Defined");
                probe.take("IsCurse");
                let origin = probe.take("Origin")?;
                let dest = probe.take("Destination")?;
                let inner = match (origin.as_str(), dest.as_str()) {
                    ("Battlefield" | "Graveyard" | "Exile" | "Graveyard,Exile", "Hand") => {
                        "Effect::ReturnToHand { target: TargetSpec::EventObject }".to_string()
                    }
                    ("Battlefield" | "Graveyard" | "Hand" | "Stack", "Exile") => {
                        "Effect::exile(TargetSpec::EventObject)".to_string()
                    }
                    ("Graveyard" | "Exile" | "Hand", "Battlefield") => {
                        let tapped = match probe.take("Tapped").as_deref() {
                            None | Some("False") => false,
                            Some("True") => true,
                            Some(_) => return None,
                        };
                        let gain = match probe.take("GainControl").as_deref() {
                            None | Some("False") => false,
                            Some("True") => true,
                            Some(_) => return None,
                        };
                        let owner_control = !gain;
                        let origin_ref = match origin.as_str() {
                            "Graveyard" => "Some(ZoneRef::Graveyard)",
                            "Exile" => "Some(ZoneRef::Exile)",
                            "Hand" => "Some(ZoneRef::Hand)",
                            _ => "None",
                        };
                        format!(
                            "Effect::MoveAndModify {{ subject: MoveSubject::Target(TargetSpec::EventObject), \
                             expected_origin: {origin_ref}, blink: false, owner_control: {owner_control}, \
                             tapped: {tapped}, face: None, counters: None, modifiers: &[], \
                             duration: Duration::Indefinitely, attach_source: false, \
                             when_source_leaves: None }}"
                        )
                    }
                    _ => return None,
                };
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(vec![format!(
                    "Effect::ForEachRemembered {{ filter: &Filter::Any, effects: &[{inner}] }}"
                )])
            }
            "Goad" => {
                let mut probe = p.clone();
                probe.take("Defined");
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(vec![
                    "Effect::ForEachRemembered { filter: &Filter::Any, \
                     effects: &[Effect::Goad { target: TargetSpec::EventObject }] }"
                        .to_string(),
                ])
            }
            "Detain" => {
                let mut probe = p.clone();
                probe.take("Defined");
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(vec![
                    "Effect::ForEachRemembered { filter: &Filter::Any, \
                     effects: &[Effect::Detain { target: TargetSpec::EventObject }] }"
                        .to_string(),
                ])
            }
            "Phases" => {
                let mut probe = p.clone();
                probe.take("Defined");
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(vec![
                    "Effect::ForEachRemembered { filter: &Filter::Any, \
                     effects: &[Effect::PhaseOut { target: Some(TargetSpec::EventObject) }] }"
                        .to_string(),
                ])
            }
            "GainControl" => {
                let mut probe = p.clone();
                probe.take("Defined");
                let to = self.gain_control_ex_new_controller(
                    probe.take("NewController").as_deref(),
                    None,
                    false,
                )?;
                let (duration, untap, haste_until_eot) = self.gain_control_ex_riders(&mut probe)?;
                if !Self::only_condition_keys_left(&probe) {
                    return None;
                }
                *p = probe;
                Some(vec![format!(
                    "Effect::ForEachRemembered {{ filter: &Filter::Any, \
                     effects: &[Effect::GainControlEx {{ target: TargetSpec::EventObject, \
                     to: {to}, duration: {duration}, untap: {untap}, \
                     haste_until_eot: {haste_until_eot} }}] }}"
                )])
            }
            _ => self.wave28a_remembered_step(api, p, target, flags),
        }
    }

    /// Lower `ConditionDefined$ Remembered[LKI] | ConditionPresent$ <P> | [ConditionCompare$ <C>]`
    /// inside `Effect::CaptureAffected::then`.
    pub(super) fn wave27c_wrap_remembered_condition(
        &mut self,
        p: &mut Params,
        effects: &[String],
    ) -> Option<Vec<String>> {
        let scope = self.capture_scope.as_ref()?.clone();
        if !matches!(
            p.peek("ConditionDefined"),
            Some("Remembered" | "RememberedLKI")
        ) || !p.has("ConditionPresent")
            || p.has("PresentPlayer")
            || p.has("PresentZone")
            || p.has("ConditionZone")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("ConditionDefined");
        probe.take("ConditionDescription");
        let present = probe.take("ConditionPresent")?;
        let cmp = probe
            .take("PresentCompare")
            .or_else(|| probe.take("ConditionCompare"))
            .unwrap_or_else(|| "GE1".to_string());
        if !probe.exhausted() {
            return None;
        }

        // Path A: every remembered object is guaranteed to satisfy `present` ->
        // compare `Amount::RememberedCount` directly!
        if Self::scope_guarantees_present(&scope, &present) {
            let cmp = cmp.trim();
            if cmp.len() < 3 {
                return None;
            }
            let (op_code, rhs_raw) = cmp.split_at(2);
            let rhs: u32 = rhs_raw.parse().ok()?;
            let op = match op_code {
                "GE" => "AmountCmp::AtLeast",
                "GT" => "AmountCmp::GreaterThan",
                "LE" => "AmountCmp::AtMost",
                "LT" => "AmountCmp::LessThan",
                "EQ" => "AmountCmp::Equal",
                "NE" => "AmountCmp::NotEqual",
                _ => return None,
            };
            *p = probe;
            return Some(vec![format!(
                "Effect::IfCondition {{ condition: Condition::AmountCompare {{ \
                 left: &Amount::RememberedCount, op: {op}, right: &Amount::Fixed({rhs}) }}, \
                 then: &[{}], otherwise: &[] }}",
                effects.join(", ")
            )]);
        }

        // Path B: `first` affects at most 1 object AND comparison asks "at least 1
        // remembered object matches `<present>`" -> `Effect::ForEachRemembered { filter, effects }`!
        // (e.g. Shredder's Technique: Destroy Creature,Enchantment -> if Enchantment, lose 2 life).
        if scope.single_object && matches!(cmp.trim(), "GE1" | "EQ1" | "GT0") {
            let filter_expr = self.filter_expr_in_zone(&present, !scope.from_battlefield)?;
            let filter_name = self.body.filter_static("REM_COND", &filter_expr);
            *p = probe;
            return Some(vec![format!(
                "Effect::ForEachRemembered {{ filter: &{filter_name}, effects: &[{}] }}",
                effects.join(", ")
            )]);
        }

        None
    }

    // =========================================================================
    // Part 2: 2–3 slot independent target chains & `Effect::WithAllTargets`
    // =========================================================================

    /// Format `second_targets` (and optional `third_targets`) for ability macros.
    ///
    /// Called by `Tx::second_target_req_expr`: every macro site wraps the
    /// returned string in `format!(", second_targets = Some({req})")`. When
    /// `third_target` is present, this emits
    /// `"{second_req}), third_targets = Some({third_req}"` so all macros
    /// (`spell!`, `activated!`, `triggered!`, `loyalty!`, `chapter!`,
    /// `chapters!`) emit both fields cleanly.
    pub(super) fn wave27c_extra_target_req_expr(chain: &Chain) -> Option<String> {
        let second = Self::format_slot_req(
            chain.second_target.as_deref()?,
            chain.second_target_up_to,
            chain.second_target_range,
            chain.second_target_constraint,
        );
        if let Some(third_spec) = chain.third_target.as_deref() {
            let third = Self::format_slot_req(
                third_spec,
                chain.third_target_up_to,
                chain.third_target_range,
                chain.third_target_constraint,
            );
            Some(format!("{second}), third_targets = Some({third}"))
        } else {
            Some(second)
        }
    }

    fn format_slot_req(
        spec: &str,
        up_to: Option<u8>,
        range: Option<(u8, u8)>,
        constraint: Option<&'static str>,
    ) -> String {
        let base = if let Some(max) = up_to {
            if max == 1 {
                format!("TargetReq::up_to_one({spec})")
            } else {
                format!("TargetReq::up_to({spec}, {max})")
            }
        } else if let Some((min, max)) = range {
            Self::wave28a_format_range_req(spec, min, max)
        } else {
            format!("TargetReq::one({spec})")
        };
        match constraint {
            Some(c) => format!("{base}.with_constraint({c})"),
            None => base,
        }
    }

    /// Parse `TargetMin$` / `TargetMax$` / `TargetUnique$` for Slot 2 or Slot 3.
    #[allow(clippy::type_complexity)]
    fn take_slot_bounds_and_unique(
        &self,
        api: &str,
        p: &mut Params,
        valid: &str,
        spec: &str,
        allow_multi: bool,
    ) -> Option<(Option<u8>, Option<(u8, u8)>, Option<&'static str>)> {
        let constraint = match p.peek("TargetUnique") {
            None => None,
            Some("True") => {
                p.take("TargetUnique");
                Some("TargetConstraint::DistinctFromPriorTargetGroups")
            }
            Some(_) => return None,
        };
        let is_obj_spec = spec.starts_with("TargetSpec::Object(");
        let mut range = None;
        let up_to = if p.peek("TargetMin") == Some("1")
            && matches!(p.peek("TargetMax"), None | Some("1"))
        {
            p.take("TargetMin");
            p.take("TargetMax");
            None
        } else if p.peek("TargetMin") == Some("0")
            && matches!(p.peek("TargetMax"), None | Some("1"))
        {
            p.take("TargetMin");
            p.take("TargetMax");
            Some(1)
        } else if allow_multi
            && p.peek("TargetMin") == Some("0")
            && let Some(max) = p
                .peek("TargetMax")
                .and_then(|s| s.trim().parse::<u8>().ok())
            && max > 1
        {
            p.take("TargetMin");
            p.take("TargetMax");
            Some(max)
        } else if allow_multi
            && p.peek("TargetMin") == Some("0")
            && let Some(max_var) = p.peek("TargetMax")
            && self.svars.get(max_var.trim()).is_some_and(|def| {
                let def = def.trim();
                def == format!("Count$Valid {valid}")
                    || def
                        .strip_prefix("Count$Valid ")
                        .is_some_and(|cv| Self::same_valid_atoms(cv, valid))
                    || (is_obj_spec && matches!(def, "Count$Valid Card" | "Count$Valid Permanent"))
                    || (api == "ChangeZone"
                        && p.peek("Origin") == Some("Graveyard")
                        && def.strip_prefix("Count$ValidGraveyard ").is_some_and(|cv| {
                            cv == valid
                                || cv == "Card"
                                || (matches!(cv, "Card.YouOwn" | "Card.YouCtrl")
                                    && (valid.contains("YouOwn") || valid.contains("YouCtrl")))
                                || Self::same_valid_atoms(
                                    &cv.replace("YouOwn", "YouCtrl"),
                                    &valid.replace("YouOwn", "YouCtrl"),
                                )
                        }))
            })
        {
            p.take("TargetMin");
            p.take("TargetMax");
            Some(u8::MAX)
        } else if let Some(r) = self.wave28a_take_extra_slot_range(api, p, valid, spec, allow_multi)
        {
            range = Some(r);
            None
        } else if p.has("TargetMin") || p.has("TargetMax") {
            return None;
        } else {
            None
        };
        Some((up_to, range, constraint))
    }

    /// Wave 27c hook in `chain_target`: handles
    /// 1. 3rd-target registration (`chain.second_target.is_some() && chain.third_target.is_none()`),
    /// 2. Enhanced 2nd-target registration when `wave26c` refused (e.g., `Bazaar Trader`
    ///    `GainControl | NewController$ ParentTarget`, `Assassin Gauntlet` `TapAll`,
    ///    `Cybernetica Datasmith` `Token | TokenOwner$ ThisTargetedPlayer | TargetUnique$ True`,
    ///    3-slot `Pump` / `TargetOnly` chains where `later_line_targets` is true!).
    ///
    /// Returns `Some(true)` if handled, `Some(false)` to fall back to existing logic,
    /// or `None` to deny explicitly.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave27c_chain_extra_target(
        &mut self,
        api: &str,
        p: &mut Params,
        valid: &str,
        spec: &str,
        chain: &mut Chain,
    ) -> Option<bool> {
        if chain.target.is_none() {
            return Some(false);
        }
        // Refuse 4+ independent target slots honestly!
        if chain.third_target.is_some() {
            return self.deny("unsupported independent target in one chain".to_string());
        }

        // Case A: Registering Slot 3 (`chain.second_target.is_some()`).
        if chain.second_target.is_some() {
            if !self.wave27c_third_target_allowed(api, p, chain, spec) {
                return Some(false);
            }
            let allow_multi = Self::wave26c_second_target_multi_ok(api, p);
            let Some((up_to, range, constraint)) =
                self.take_slot_bounds_and_unique(api, p, valid, spec, allow_multi)
            else {
                return Some(false);
            };
            chain.third_target = Some(spec.to_string());
            chain.third_target_up_to = up_to;
            chain.third_target_range = range;
            chain.third_target_constraint = constraint;
            return Some(true);
        }

        // Case B: Registering Slot 2 for Wave 27c shapes not covered by Wave 26c:
        // - `TargetOnly` chaining into a 3rd `TargetOnly` (`Boom Box`, `Convert to Slime`)
        // - `GainControl | NewController$ ParentTarget` (`Bazaar Trader`)
        // - `TapAll` / `UntapAll` / `PumpAll` / `DestroyAll` / `SacrificeAll` / `Token`
        //   (`Assassin Gauntlet`, `Cybernetica Datasmith`, `Cut In`, `Eriette's Whisper`)
        // - Any general Slot 2 line whose `SubAbility$` declares a valid Slot 3
        //   (`Arm the Cathars`, `Blue Dragon`, `Bounty of Might`, `Everything Pizza`)
        // - Slot 2 lines carrying `TargetUnique$ True` -> `DistinctFromPriorTargetGroups`
        let is_bazaar_trader = api == "GainControl"
            && p.peek("NewController") == Some("ParentTarget")
            && !p.has("Defined")
            && !p.has("AllValid")
            && spec.starts_with("TargetSpec::Object(")
            && matches!(
                chain.target.as_deref(),
                Some("TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent")
            )
            && !self.later_line_targets(p.peek("SubAbility"));

        let is_three_target_only = api == "TargetOnly"
            && chain.effects.is_empty()
            && self.wave27c_can_three_target_only_slot2(p.peek("SubAbility"), chain, spec);

        let is_slot2_before_slot3 = self.wave27c_slot2_general_allowed(api, p, chain, spec);

        if !is_bazaar_trader && !is_three_target_only && !is_slot2_before_slot3 {
            return Some(false);
        }
        let allow_multi = Self::wave26c_second_target_multi_ok(api, p)
            || (api == "GainControl" && p.peek("NewController") == Some("ParentTarget"));
        let Some((up_to, range, constraint)) =
            self.take_slot_bounds_and_unique(api, p, valid, spec, allow_multi)
        else {
            return Some(false);
        };
        chain.second_target = Some(spec.to_string());
        chain.second_target_up_to = up_to;
        chain.second_target_range = range;
        chain.second_target_constraint = constraint;
        Some(true)
    }

    /// True when Slot 2 `TargetOnly` leads into Slot 3 `TargetOnly` followed by
    /// an object-union `Defined$ Targeted` operation (`WithAllTargets`).
    fn wave27c_can_three_target_only_slot2(
        &self,
        sub: Option<&str>,
        chain: &Chain,
        second_spec: &str,
    ) -> bool {
        let Some(sub_name) = sub else {
            return false;
        };
        let Some(body) = self.svars.get(sub_name.trim()) else {
            return false;
        };
        let Some((api, sp)) = Params::parse(body) else {
            return false;
        };
        if api != "TargetOnly" || !sp.has("ValidTgts") {
            return false;
        }
        let first_spec = chain.target.as_deref().unwrap_or_default();
        if !first_spec.starts_with("TargetSpec::Object(")
            || !second_spec.starts_with("TargetSpec::Object(")
        {
            return false;
        }
        self.wave27c_can_all_targets_followup(sp.peek("SubAbility"))
    }

    /// True when `sub` is a `Defined$ Targeted` line that can run inside
    /// `Effect::WithAllTargets(&[...])` (with no further target declarations).
    fn wave27c_can_all_targets_followup(&self, sub: Option<&str>) -> bool {
        let Some(sub_name) = sub else {
            return false;
        };
        let Some(body) = self.svars.get(sub_name.trim()) else {
            return false;
        };
        let Some((api, sp)) = Params::parse(body) else {
            return false;
        };
        if sp.has("ValidTgts")
            || sp.peek("Defined") != Some("Targeted")
            || self.later_line_targets(sp.peek("SubAbility"))
        {
            return false;
        }
        matches!(
            api.as_str(),
            "Destroy"
                | "Tap"
                | "Untap"
                | "Pump"
                | "PutCounter"
                | "RemoveCounter"
                | "ChangeZone"
                | "Phases"
                | "Detain"
                | "Goad"
                | "Regenerate"
        )
    }

    /// Count how many `ValidTgts$` lines remain in a `SubAbility$` chain.
    fn count_later_target_lines(&self, sub: Option<&str>) -> usize {
        let mut count = 0usize;
        let mut seen = BTreeSet::new();
        let mut cursor = sub.map(|s| s.trim().to_string());
        while let Some(name) = cursor.take() {
            if !seen.insert(name.clone()) {
                return usize::MAX;
            }
            let Some(body) = self.svars.get(&name) else {
                break;
            };
            let Some((_, sp)) = Params::parse(body) else {
                break;
            };
            if sp.has("ValidTgts") {
                count += 1;
            }
            cursor = sp.peek("SubAbility").map(|s| s.trim().to_string());
        }
        count
    }

    /// True when a Slot 2 line can safely lower via `WithSecondTarget` even when
    /// it uses Wave 27c APIs (`TapAll`, `UntapAll`, `PumpAll`, `Token`, etc.),
    /// carries `TargetUnique$ True`, or precedes a single Slot 3 target line.
    fn wave27c_slot2_general_allowed(
        &self,
        api: &str,
        p: &Params,
        chain: &Chain,
        spec: &str,
    ) -> bool {
        let later_targets = self.count_later_target_lines(p.peek("SubAbility"));
        if later_targets > 1 {
            return false;
        }
        if spec.starts_with("TargetSpec::CardInFirstTargetsGraveyard(")
            && !matches!(
                chain.target.as_deref(),
                Some("TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent")
            )
        {
            return false;
        }
        if p.has("UnlessResolveSubs")
            || !matches!(p.peek("Defined"), None | Some("Targeted" | "You"))
            || p.has("DamageSource")
            || p.has("Source")
            || p.has("CloneTarget")
            || p.has("Object")
            || p.has("TargetsWithDefinedController")
            || p.peek("ConditionDefined")
                .is_some_and(|d| d.contains("ParentTarget"))
        {
            return false;
        }
        // Only allow `Defined$ You` on `Draw` (e.g. Biomantic Mastery) when not hiding a target.
        if p.peek("Defined") == Some("You") && api != "Draw" {
            return false;
        }
        if api != "GainControl" && p.has("NewController") {
            return false;
        }
        Self::wave27c_general_slot_api_ok(api, p, spec)
    }

    /// True when a Slot 3 line can safely lower via `WithThirdTarget` (or 3-slot `TargetOnly`).
    fn wave27c_third_target_allowed(
        &self,
        api: &str,
        p: &Params,
        chain: &Chain,
        spec: &str,
    ) -> bool {
        if self.later_line_targets(p.peek("SubAbility")) {
            return false;
        }
        if api == "TargetOnly" {
            return chain.effects.is_empty()
                && spec.starts_with("TargetSpec::Object(")
                && self.wave27c_can_all_targets_followup(p.peek("SubAbility"));
        }
        if p.has("UnlessResolveSubs")
            || !matches!(p.peek("Defined"), None | Some("Targeted"))
            || p.has("DamageSource")
            || p.has("Source")
            || p.has("CloneTarget")
            || p.has("Object")
            || p.has("TargetsWithDefinedController")
            || p.peek("ConditionDefined")
                .is_some_and(|d| d.contains("ParentTarget"))
        {
            return false;
        }
        if api != "GainControl" && p.has("NewController") {
            return false;
        }
        Self::wave27c_general_slot_api_ok(api, p, spec)
    }

    /// Shared allowlist for Slot 2 / Slot 3 `WithSecondTarget` / `WithThirdTarget` APIs.
    fn wave27c_general_slot_api_ok(api: &str, p: &Params, spec: &str) -> bool {
        match api {
            "ChangeZone" => {
                !p.has("RememberChanged")
                    && !p.has("RememberTargets")
                    && !p.has("RememberLKI")
                    && !p.has("Imprint")
                    && !p.has("ThisDefinedAndTgts")
            }
            "PutCounter" => !p.has("DividedAsYouChoose") && !p.has("Support"),
            "GainControl" => {
                p.peek("NewController")
                    .is_none_or(|v| matches!(v, "You" | "ParentTarget"))
                    && !p.has("AllValid")
            }
            "Phases" => !p.has("WontPhaseInNormal"),
            "DealDamage" => !p.has("DividedAsYouChoose") && !p.has("ExcessSVar"),
            "TapAll" | "UntapAll" | "PumpAll" | "DestroyAll" | "SacrificeAll" => {
                matches!(spec, "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent")
            }
            "Token" => {
                (matches!(spec, "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent")
                    && matches!(
                        p.peek("TokenOwner"),
                        Some("ThisTargetedPlayer" | "Targeted")
                    ))
                    || (spec.starts_with("TargetSpec::Object(")
                        && p.peek("AttachedTo") == Some("ThisTargetedCard"))
            }
            "Destroy" | "Pump" | "RemoveCounter" | "Tap" | "Untap" | "TapOrUntap" | "Detain"
            | "Goad" | "Animate" | "Regenerate" | "SetState" | "MustBlock" | "Connive"
            | "Explore" | "Suspect" | "CopyPermanent" | "Counter" | "PreventDamage" | "Draw"
            | "Discard" | "Mill" | "GainLife" | "LoseLife" | "Poison" | "Sacrifice" | "SetLife"
            | "SkipPhase" | "TakeInitiative" => true,
            _ => false,
        }
    }

    /// Lower a 2nd-target `Token` (e.g. `TokenOwner$ ThisTargetedPlayer` -> `Targeted`,
    /// or `AttachedTo$ ThisTargetedCard` -> `Targeted`) or `GainControl | NewController$ ParentTarget`
    /// (`Bazaar Trader` -> `PlayerRel::FirstTargetPlayerOrController`)!
    pub(super) fn wave27c_slot2_or_slot3_effect(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
        single_target: bool,
        is_second_slot: bool,
    ) -> Option<Vec<String>> {
        if api == "GainControl"
            && is_second_slot
            && p.peek("NewController") == Some("ParentTarget")
            && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
        {
            let mut probe = p.clone();
            probe.take("NewController");
            match probe.take("Defined").as_deref() {
                None | Some("Targeted") => {}
                Some(_) => return None,
            }
            let (duration, untap, haste_until_eot) = self.gain_control_ex_riders(&mut probe)?;
            if !probe.exhausted() {
                return None;
            }
            let tgt = target?;
            *p = probe;
            return Some(vec![format!(
                "Effect::GainControlEx {{ target: {tgt}, to: PlayerRel::FirstTargetPlayerOrController, \
                 duration: {duration}, untap: {untap}, haste_until_eot: {haste_until_eot} }}"
            )]);
        }
        if api == "Token" {
            let mut probe = p.clone();
            if probe.peek("TokenOwner") == Some("ThisTargetedPlayer") && targets_a_player {
                probe.take("TokenOwner");
                probe
                    .entries
                    .push(("TokenOwner".to_string(), "Targeted".to_string()));
            }
            if probe.peek("AttachedTo") == Some("ThisTargetedCard")
                && target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            {
                probe.take("AttachedTo");
                probe
                    .entries
                    .push(("AttachedTo".to_string(), "Targeted".to_string()));
            }
            let effs = self.effect_of_inner(
                api,
                &mut probe,
                target,
                LineFlags {
                    targets_here: true,
                    targets_a_player,
                    single_target,
                    prior_target_used: false,
                },
            )?;
            if !probe.exhausted() {
                return None;
            }
            *p = probe;
            return Some(effs);
        }
        self.effect_of_inner(
            api,
            p,
            target,
            LineFlags {
                targets_here: true,
                targets_a_player,
                single_target,
                prior_target_used: false,
            },
        )
    }

    /// Lower a Slot 3 target line (`third_target_here`) or a trailing
    /// `Defined$ Targeted` line after a 2- or 3-slot multi-target chain
    /// (`Effect::WithAllTargets(&[...])`, including when combined with
    /// `RememberDestroyed$ True` like *Convert to Slime*).
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(super) fn wave27c_multi_target_step(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &mut Chain,
        third_target_here: bool,
        had_second_target: bool,
        targets_here: bool,
    ) -> Option<(Vec<String>, Option<MultiTargetAction>)> {
        // 1. Slot 3 `TargetOnly` -> produces no effects of its own; advances to `sub`!
        if third_target_here && api == "TargetOnly" {
            p.take("IsCurse");
            if !p.exhausted() {
                return None;
            }
            return Some((Vec::new(), None));
        }

        // 2. Slot 3 normal targeting line -> wrap in `Effect::WithThirdTarget(&[...])`!
        if third_target_here {
            let third_spec = chain.third_target.as_deref().map(Self::effect_target_of)?;
            let targets_a_player = third_spec == "TargetSpec::Player(PlayerRel::Chosen)";
            let third_single = chain.third_target_up_to.is_none_or(|n| n <= 1)
                && chain.third_target_range.is_none();
            let raw = self.wave27c_slot2_or_slot3_effect(
                api,
                p,
                Some(&third_spec),
                targets_a_player,
                third_single,
                false,
            )?;
            let raw = if api == "GainControl" && !third_single && !targets_a_player {
                Self::rewrite_multi_target_gain_control(raw)?
            } else {
                raw
            };
            return Some((raw, Some(MultiTargetAction::ThirdTarget)));
        }

        // 3. Trailing `Defined$ Targeted` after a 2- or 3-slot chain -> `Effect::WithAllTargets(&[...])`!
        //    Works for *Arm the Cathars* (`DBPump3: Pump | Defined$ Targeted | KW$ Vigilance`),
        //    *Boom Box* (`Destroy | Defined$ Targeted`), and *Convert to Slime*
        //    (`Destroy | Defined$ Targeted | RememberDestroyed$ True | SubAbility$ DBToken`)!
        if had_second_target
            && !targets_here
            && p.peek("Defined") == Some("Targeted")
            && chain
                .target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && chain
                .second_target
                .as_deref()
                .is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && chain
                .third_target
                .as_deref()
                .is_none_or(|t| t.starts_with("TargetSpec::Object("))
        {
            let union_target = "TargetSpec::Object(&Filter::Any)";
            let mut probe = p.clone();
            let has_remember = Self::wave27c_capture_flag(api, &probe).is_some() && sub.is_some();
            if has_remember {
                // Keep `Remember*` on `p`, strip only from `probe` to build the
                // `WithAllTargets` `first` slice, then delegate to `wave27c_capture_affected_chain`!
                let flag_key = Self::wave27c_capture_flag(api, &probe)?;
                probe.take(flag_key);
                probe.take("Defined");
                let inner = self.effect_of_inner(
                    api,
                    &mut probe,
                    Some(union_target),
                    LineFlags {
                        targets_here: true,
                        targets_a_player: false,
                        single_target: false,
                        prior_target_used: false,
                    },
                )?;
                if !probe.exhausted() || inner.is_empty() {
                    return None;
                }
                let wrapped_first =
                    vec![format!("Effect::WithAllTargets(&[{}])", inner.join(", "))];
                // Strip `Defined` from a copy of `p` before calling capture_affected_chain.
                let mut cap_p = p.clone();
                cap_p.take("Defined");
                let cap_effs = self.wave27c_capture_affected_chain(
                    api,
                    &mut cap_p,
                    sub,
                    Some(union_target),
                    LineFlags {
                        targets_here: true,
                        targets_a_player: false,
                        single_target: false,
                        prior_target_used: false,
                    },
                    chain,
                    Some(wrapped_first),
                )?;
                if !cap_p.exhausted() {
                    return None;
                }
                *p = cap_p;
                return Some((cap_effs, None));
            }
            probe.take("Defined");
            let inner = self.effect_of_inner(
                api,
                &mut probe,
                Some(union_target),
                LineFlags {
                    targets_here: true,
                    targets_a_player: false,
                    single_target: false,
                    prior_target_used: false,
                },
            )?;
            if !probe.exhausted() || inner.is_empty() {
                return None;
            }
            *p = probe;
            return Some((inner, Some(MultiTargetAction::AllTargets)));
        }

        None
    }

    /// True when Slot 2 `TargetOnly` should simply register Slot 2 and return
    /// `Vec::new()` because Slot 3 is another `TargetOnly` (`Boom Box`, `Convert to Slime`).
    pub(super) fn wave27c_is_chained_target_only_slot2(&self, sub: Option<&str>) -> bool {
        let Some(sub_name) = sub else {
            return false;
        };
        let Some(body) = self.svars.get(sub_name.trim()) else {
            return false;
        };
        let Some((api, sp)) = Params::parse(body) else {
            return false;
        };
        api == "TargetOnly" && sp.has("ValidTgts")
    }
}
