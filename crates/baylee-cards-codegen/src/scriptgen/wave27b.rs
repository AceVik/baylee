//! Codegen Wave 27b: Paper-MTG zone-entry attachment, entering attacking,
//! constrained/random/distinct-name choices and searches, optional/untargeted
//! attachment, and defined discard mapped onto Engine Wave 15b DSL primitives.
//!
//! Pure Paper-MTG only: refuses Alchemy/digital mechanics, Un-set/Acorn,
//! Planechase, `RememberChanged$` (owned by Wave 27c), and any Forge parameter
//! not faithfully represented by the target Wave 15b DSL variant.

use super::{Chain, Params, Tx};

impl Tx<'_> {
    /// `Token` with `AttachedTo$` / `AttachedToPlayer$` ->
    /// `Effect::CreateTokenAttached` (Role tokens, Aura tokens, Equipment
    /// tokens; CR 303.4f–g, CR 301.5c).
    pub(super) fn wave27b_token_attached(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let raw_owner = p.take("TokenOwner");
        match raw_owner.as_deref() {
            None | Some("You") => {}
            Some("TriggeredCardController") if self.trigger_is_self => {}
            Some(other) => {
                return self.deny(format!("attached token owner `{other}`"));
            }
        }
        if p.has("TokenPower")
            || p.has("TokenToughness")
            || p.has("TokenTapped")
            || p.has("TokenAttacking")
            || p.has("PumpKeywords")
            || p.has("PumpDuration")
            || p.has("AtEOT")
            || p.has("RememberTokens")
        {
            return self.deny("attached token with unsupported entry modifiers".to_string());
        }
        let (host_raw, is_player) = match (p.take("AttachedTo"), p.take("AttachedToPlayer")) {
            (Some(obj), None) => (obj, false),
            (None, Some(pl)) => (pl, true),
            _ => return self.deny("attached token with conflicting host keys".to_string()),
        };
        let host = self.wave27b_attach_host(&host_raw, is_player, target, targets_a_player)?;
        let Some(stem) = p.take("TokenScript") else {
            return self.deny("a `Token` effect naming no `TokenScript$`".to_string());
        };
        let raw_amount = p.take("TokenAmount");
        let amount = match raw_amount.as_deref() {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => {
                let expr = self.amount_or_count_ext(raw)?;
                if expr == "Amount::Fixed(0)" {
                    return self.deny("attached token amount of zero".to_string());
                }
                expr
            }
        };
        let Some(tokens) = self.tokens else {
            return self.deny("`Token` with no token scripts to read it against".to_string());
        };
        let Some(body) = tokens.body_fixed(&stem, self.cats) else {
            return self.deny(format!("token script `{stem}`"));
        };
        let token = format!("&generated_tokens::{}", body.constant);
        Some(vec![format!(
            "Effect::CreateTokenAttached {{ token: {token}, amount: {amount}, host: {host} }}"
        )])
    }

    /// `ChangeZone` with `Destination$ Battlefield` and `AttachedTo$` /
    /// `AttachedToPlayer$` -> `Effect::PutOntoBattlefieldAttached`
    /// (CR 303.4f–g, CR 301.5c, CR 301.6).
    ///
    /// Library searches (`Origin$ Library` or `OriginAlternative$`) are
    /// honestly refused because `SearchLibrary` cannot attach an Aura as it
    /// enters without violating CR 303.4f–g.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave27b_change_zone_attached(
        &mut self,
        p: &mut Params,
        origin: &str,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if !matches!(origin, "Graveyard" | "Hand" | "Exile")
            || p.has("OriginAlternative")
            || p.has("RememberChanged")
            || p.has("Transformed")
            || p.has("WithCountersType")
            || p.has("WithCountersAmount")
            || p.has("StaticEffect")
            || p.has("Attacking")
            || p.has("AtEOT")
            || p.has("Imprint")
            || p.has("ExileFaceDown")
        {
            return None;
        }
        let (host_raw, is_player) = match (p.take("AttachedTo"), p.take("AttachedToPlayer")) {
            (Some(obj), None) => (obj, false),
            (None, Some(pl)) => (pl, true),
            _ => return None,
        };
        p.take("IsCurse");
        p.take("SelectPrompt");
        p.take("TgtPrompt");
        p.claim_label("ChangeTypeDesc", "ChangeType");
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`ChangeZone` with `Tapped$ {other}`")),
        };
        let raw_gain = p.take("GainControl");
        let raw_defined = p.take("Defined");

        // Case 1: `Defined$ Self` (Dragon Breath/Fangs/Scales/Shadow/Wings,
        // Gryff's Boon, Eagle's Rescue).
        if raw_defined.as_deref() == Some("Self") && !targets_here {
            let owner_control = match raw_gain.as_deref() {
                None | Some("True") => false,
                Some("False") => true,
                Some(g) => return self.deny(format!("`ChangeZone` with `GainControl$ {g}`")),
            };
            let host = self.wave27b_attach_host(
                &host_raw,
                is_player,
                target,
                target.is_some_and(Self::wave27b_is_player_spec),
            )?;
            if host == "AttachHost::Spec(TargetSpec::ThisObject)" {
                return None;
            }
            return Some(vec![format!(
                "Effect::PutOntoBattlefieldAttached {{ \
                 subject: DefinedObjects::Spec(TargetSpec::ThisObject), \
                 host: {host}, owner_control: {owner_control}, tapped: {tapped} }}"
            )]);
        }

        // Case 2: Event-object defined (`DelayTriggerRememberedLKI`,
        // `TriggeredNewCardLKICopy`, etc.; e.g. Lynde's Curse return).
        if let Some(def) = raw_defined.as_deref()
            && !targets_here
            && self.is_event_object_defined(def)
        {
            let owner_control = match raw_gain.as_deref() {
                None | Some("False") => true,
                Some("True") => false,
                Some(g) => return self.deny(format!("`ChangeZone` with `GainControl$ {g}`")),
            };
            let host = self.wave27b_attach_host(
                &host_raw,
                is_player,
                target,
                target.is_some_and(Self::wave27b_is_player_spec),
            )?;
            if host == "AttachHost::Spec(TargetSpec::EventObject)" {
                return None;
            }
            return Some(vec![format!(
                "Effect::PutOntoBattlefieldAttached {{ \
                 subject: DefinedObjects::Spec(TargetSpec::EventObject), \
                 host: {host}, owner_control: {owner_control}, tapped: {tapped} }}"
            )]);
        }

        // Case 3: `Defined$ ValidGraveyard <filter>` (*Retether*).
        if let Some(def) = raw_defined.as_deref()
            && origin == "Graveyard"
            && !targets_here
            && target.is_none()
            && let Some(valid) = def.strip_prefix("ValidGraveyard ")
        {
            let (zone, default_owner_ctrl, stripped) = Self::wave27b_graveyard_zone_sel(valid)?;
            let owner_control = match raw_gain.as_deref() {
                None => default_owner_ctrl,
                Some("True") => false,
                Some("False") => true,
                Some(g) => return self.deny(format!("`ChangeZone` with `GainControl$ {g}`")),
            };
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            let name = self.body.filter_static("ATTACH_SUBJECT", &expr);
            let host = self.wave27b_attach_host(&host_raw, is_player, None, false)?;
            return Some(vec![format!(
                "Effect::PutOntoBattlefieldAttached {{ \
                 subject: DefinedObjects::AllInZone {{ zone: {zone}, filter: &{name} }}, \
                 host: {host}, owner_control: {owner_control}, tapped: {tapped} }}"
            )]);
        }

        // Case 4: Targeted graveyard Aura/Equipment (*Iridescent Drake*,
        // *Magnetic Snuffler*, *Hakim, Loreweaver*, *Nomad Mythmaker*,
        // *One Last Job*).
        if targets_here
            && origin == "Graveyard"
            && matches!(raw_defined.as_deref(), None | Some("Targeted"))
            && let Some(tgt) = target.filter(|t| t.starts_with("TargetSpec::CardInGraveyard"))
        {
            if let Some(num) = p.take("ChangeNum")
                && num != "1"
            {
                return self.deny(format!("`ChangeZone` with `ChangeNum$ {num}`"));
            }
            if let Some(m) = p.take("Mandatory")
                && m != "True"
            {
                return self.deny(format!("`ChangeZone` with `Mandatory$ {m}`"));
            }
            let owner_control = match raw_gain.as_deref() {
                None | Some("True") => false,
                Some("False") => true,
                Some(g) => return self.deny(format!("`ChangeZone` with `GainControl$ {g}`")),
            };
            // The ability's target is the card being put onto the battlefield,
            // never the host.
            let host = self.wave27b_attach_host(&host_raw, is_player, None, false)?;
            return Some(vec![format!(
                "Effect::PutOntoBattlefieldAttached {{ \
                 subject: DefinedObjects::Spec({tgt}), \
                 host: {host}, owner_control: {owner_control}, tapped: {tapped} }}"
            )]);
        }

        // Case 5: Untargeted choice from Hand / Graveyard / Exile with
        // `ChangeType$` and no `Defined$`.
        if raw_defined.is_none() && !targets_here && p.has("ChangeType") {
            if !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
                || !matches!(p.take("Chooser").as_deref(), None | Some("You"))
            {
                return None;
            }
            if let Some(hidden) = p.take("Hidden")
                && hidden != "True"
            {
                return None;
            }
            let owner_control = match raw_gain.as_deref() {
                None | Some("True") => false,
                Some("False") => true,
                Some(g) => return self.deny(format!("`ChangeZone` with `GainControl$ {g}`")),
            };
            let optional = self.wave27b_untargeted_optional(p, origin)?;
            let raw_num = p.take("ChangeNum").unwrap_or_else(|| "1".to_string());
            let (min, max) = self.wave27b_choice_bounds(&raw_num, optional)?;
            let raw_valid = p.take("ChangeType")?;
            if raw_valid.contains("CanEnchant")
                || raw_valid.contains("ExiledWith")
                || raw_valid.contains("IsRemembered")
            {
                return None;
            }
            let stripped = Self::wave27b_strip_you_owner(&raw_valid);
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            let filter_name = self.body.filter_static("ATTACH_CARD", &expr);
            let host = self.wave27b_attach_host(
                &host_raw,
                is_player,
                target,
                target.is_some_and(Self::wave27b_is_player_spec),
            )?;
            let choose_zone = match origin {
                "Hand" => "ChooseZone::Hand(PlayerRel::You)",
                "Graveyard" => "ChooseZone::Graveyard(PlayerRel::You)",
                "Exile" => "ChooseZone::Exile(PlayerRel::You)",
                _ => return None,
            };
            return Some(vec![format!(
                "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: {choose_zone}, \
                 filter: &{filter_name}, min: {min}, max: {max}, reveal_chosen: false, \
                 movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
                 then: &[Effect::PutOntoBattlefieldAttached {{ \
                 subject: DefinedObjects::Found, host: {host}, \
                 owner_control: {owner_control}, tapped: {tapped} }}], \
                 for_unchosen: &[] }}"
            )]);
        }

        None
    }

    /// `ChangeZone` with `Destination$ Battlefield` and `Attacking$ True` ->
    /// `Effect::PutOntoBattlefieldAttacking` (CR 508.4, CR 506.3b).
    ///
    /// Strictly requires `Attacking$ True`: specific forced defenders
    /// (`Attacking$ TriggeredDefender`, `Attacking$ TriggeredTarget`) are
    /// refused honestly because `PutOntoBattlefieldAttacking` lets the
    /// controller choose any legal defender.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave27b_change_zone_attacking(
        &mut self,
        p: &mut Params,
        origin: &str,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if !matches!(origin, "Graveyard" | "Hand" | "Exile")
            || p.has("OriginAlternative")
            || p.has("RememberChanged")
            || p.has("WithCountersType")
            || p.has("WithCountersAmount")
            || p.has("Transformed")
            || p.has("StaticEffect")
            || p.has("AtEOT")
            || p.has("AttachedTo")
            || p.has("AttachedToPlayer")
            || p.has("Imprint")
        {
            return None;
        }
        let attacking = p.take("Attacking")?;
        if attacking != "True" {
            return self.deny(format!("`ChangeZone` with `Attacking$ {attacking}`"));
        }
        p.take("IsCurse");
        p.take("SelectPrompt");
        p.take("TgtPrompt");
        p.claim_label("ChangeTypeDesc", "ChangeType");
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`ChangeZone` with `Tapped$ {other}`")),
        };
        let raw_gain = p.take("GainControl");
        let raw_defined = p.take("Defined");

        // Case 1: `Defined$ Self` or omitted `Defined$` with no target/ChangeType
        // (*Interceptor, Shadow's Hound*, *Persistent Marshstalker*, *Jocasta*,
        // *Warcry Phoenix*).
        if !targets_here
            && (raw_defined.as_deref() == Some("Self")
                || (raw_defined.is_none()
                    && !p.has("ChangeType")
                    && !p.has("Hidden")
                    && !self.on_a_spell))
        {
            let owner_control = match raw_gain.as_deref() {
                None | Some("True") => false,
                Some("False") => true,
                Some(g) => return self.deny(format!("`ChangeZone` with `GainControl$ {g}`")),
            };
            return Some(vec![format!(
                "Effect::PutOntoBattlefieldAttacking {{ \
                 subject: DefinedObjects::Spec(TargetSpec::ThisObject), \
                 owner_control: {owner_control}, tapped: {tapped} }}"
            )]);
        }

        // Case 2: Event-object defined (`DelayTriggerRememberedLKI`, etc.;
        // e.g. *Meandering Towershell*).
        if let Some(def) = raw_defined.as_deref()
            && !targets_here
            && self.is_event_object_defined(def)
        {
            let owner_control = match raw_gain.as_deref() {
                None | Some("False") => true,
                Some("True") => false,
                Some(g) => return self.deny(format!("`ChangeZone` with `GainControl$ {g}`")),
            };
            return Some(vec![format!(
                "Effect::PutOntoBattlefieldAttacking {{ \
                 subject: DefinedObjects::Spec(TargetSpec::EventObject), \
                 owner_control: {owner_control}, tapped: {tapped} }}"
            )]);
        }

        // Case 3: Targeted graveyard creature (*Alesha, Who Smiles at Death*,
        // *Bereaved Survivor // Dauntless Avenger*, *Chorale of the Void*).
        if targets_here
            && origin == "Graveyard"
            && matches!(raw_defined.as_deref(), None | Some("Targeted"))
            && let Some(tgt) = target.filter(|t| t.starts_with("TargetSpec::CardInGraveyard"))
        {
            if let Some(num) = p.take("ChangeNum")
                && num != "1"
            {
                return self.deny(format!("`ChangeZone` with `ChangeNum$ {num}`"));
            }
            if let Some(m) = p.take("Mandatory")
                && m != "True"
            {
                return self.deny(format!("`ChangeZone` with `Mandatory$ {m}`"));
            }
            let owner_control = match raw_gain.as_deref() {
                None | Some("True") => false,
                Some("False") => true,
                Some(g) => return self.deny(format!("`ChangeZone` with `GainControl$ {g}`")),
            };
            return Some(vec![format!(
                "Effect::PutOntoBattlefieldAttacking {{ \
                 subject: DefinedObjects::Spec({tgt}), \
                 owner_control: {owner_control}, tapped: {tapped} }}"
            )]);
        }

        // Case 4: Untargeted choice from Hand / Graveyard / Exile
        // (*Preeminent Captain*, *Minion of the Mighty*, *Kinscaer Sentry*,
        // *Paladin Elizabeth Taggerdy*, *Shadowfax, Lord of Horses*).
        if raw_defined.is_none() && !targets_here && p.has("ChangeType") {
            if !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
                || !matches!(p.take("Chooser").as_deref(), None | Some("You"))
            {
                return None;
            }
            if let Some(hidden) = p.take("Hidden")
                && hidden != "True"
            {
                return None;
            }
            let owner_control = match raw_gain.as_deref() {
                None | Some("True") => false,
                Some("False") => true,
                Some(g) => return self.deny(format!("`ChangeZone` with `GainControl$ {g}`")),
            };
            let optional = self.wave27b_untargeted_optional(p, origin)?;
            let raw_num = p.take("ChangeNum").unwrap_or_else(|| "1".to_string());
            let (min, max) = self.wave27b_choice_bounds(&raw_num, optional)?;
            let raw_valid = p.take("ChangeType")?;
            if raw_valid.contains("ExiledWith") || raw_valid.contains("IsRemembered") {
                return None;
            }
            let stripped = Self::wave27b_strip_you_owner(&raw_valid);
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            let filter_name = self.body.filter_static("ATTACK_ENTRY", &expr);
            let choose_zone = match origin {
                "Hand" => "ChooseZone::Hand(PlayerRel::You)",
                "Graveyard" => "ChooseZone::Graveyard(PlayerRel::You)",
                "Exile" => "ChooseZone::Exile(PlayerRel::You)",
                _ => return None,
            };
            return Some(vec![format!(
                "Effect::ChooseCardEx {{ chooser: PlayerRel::You, zone: {choose_zone}, \
                 filter: &{filter_name}, min: {min}, max: {max}, reveal_chosen: false, \
                 movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, \
                 then: &[Effect::PutOntoBattlefieldAttacking {{ \
                 subject: DefinedObjects::Found, owner_control: {owner_control}, \
                 tapped: {tapped} }}], for_unchosen: &[] }}"
            )]);
        }

        None
    }

    /// `Origin$ Library` with `DifferentNames$ True` ->
    /// `Effect::SearchLibraryDistinctNames` (*Three Dreams*, *Shared Summons*,
    /// *Deathbellow War Cry*, *Reach the Horizon*, *Saheeli Rai*, *Tiamat*,
    /// *Alpine Houndmaster*, *Uncage the Menagerie*).
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub(super) fn wave27b_search_distinct_names(
        &mut self,
        p: &mut Params,
        destination: &str,
        is_own_library: bool,
        optional: bool,
        mandatory: bool,
        up_to: bool,
        reveal_requested: bool,
        base_find: &str,
        raw_num: Option<&str>,
    ) -> Option<Vec<String>> {
        if !is_own_library
            || p.has("RememberChanged")
            || p.has("WithTotalCMC")
            || p.has("Exactly")
            || p.has("AttachAfter")
        {
            return None;
        }
        if p.take("DifferentNames").as_deref() != Some("True") {
            return None;
        }
        let raw = raw_num?;
        let count_amt = self.amount_or_count_ext(raw)?;
        if count_amt == "Amount::Fixed(0)" {
            return None;
        }
        let filter = self.filter_expr_in_zone(&p.take("ChangeType")?, true)?;
        if reveal_requested
            && filter == "Filter::Any"
            && !matches!(destination, "Battlefield" | "Exile" | "Graveyard")
        {
            return self.deny("`ChangeZone` revealing an unrestricted hidden search".into());
        }
        // `SearchLibraryDistinctNames` is always "up to N" (optional: true).
        if !(up_to || optional || (!mandatory && filter != "Filter::Any")) {
            return None;
        }
        let name = self.body.filter_static("SEARCH", &filter);
        Some(vec![format!(
            "Effect::SearchLibraryDistinctNames {{ filter: &{name}, count: {count_amt}, \
             find: &{base_find} }}"
        )])
    }

    /// `Origin$ Library | Destination$ Battlefield | AttachAfter$ <filter>` for
    /// Equipment (*Stonehewer Giant*, *Quest for the Holy Relic*): Equipment
    /// enters the battlefield, then attaches to a chosen legal permanent
    /// (CR 301.5c). Refuses Auras because `SearchLibrary` would drop an Aura
    /// onto the battlefield before attaching.
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub(super) fn wave27b_search_attach_after(
        &mut self,
        p: &mut Params,
        is_own_library: bool,
        optional: bool,
        mandatory: bool,
        up_to: bool,
        base_find: &str,
        raw_num: Option<&str>,
    ) -> Option<Vec<String>> {
        if !is_own_library
            || p.has("RememberChanged")
            || p.has("DifferentNames")
            || p.has("WithTotalCMC")
        {
            return None;
        }
        if !matches!(raw_num, None | Some("1")) {
            return None;
        }
        let attach_raw = p.take("AttachAfter")?;
        if attach_raw == "True"
            || attach_raw.contains("Remembered")
            || attach_raw.contains("ChosenCard")
            || attach_raw.contains("Targeted")
        {
            return None;
        }
        let raw_change_type = p.take("ChangeType")?;
        let cleaned = Self::wave27b_strip_you_owner(&raw_change_type);
        // Only Equipment searches may enter first and attach afterward (CR 301.5c).
        if !matches!(cleaned.as_str(), "Equipment" | "Card.Equipment") {
            return None;
        }
        let search_filter = self.filter_expr_in_zone(&cleaned, true)?;
        let host_filter = self.filter_expr(&attach_raw)?;
        let search_name = self.body.filter_static("SEARCH", &search_filter);
        let host_name = self.body.filter_static("ATTACH_HOST", &host_filter);
        let opt = up_to || optional || !mandatory;
        Some(vec![format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::SearchLibrary {{ filter: &{search_name}, finds: &[{base_find}], \
             optional: {opt} }}], \
             then: &[Effect::ForEachRemembered {{ filter: &Filter::Any, \
             effects: &[Effect::MoveAttachment {{ attachment: TargetSpec::EventObject, \
             recipient_filter: &{host_name}, must_change_recipient: false, \
             optional: false, then_if_moved: &[] }}] }}] }}"
        )])
    }

    /// Untargeted `ChangeZone` with `DifferentNames$ True` (*Eerie Ultimatum*)
    /// or `AtRandom$ True` on non-Library zones (`Graveyard`, `Hand`, `Exile`,
    /// `Battlefield`) -> `Effect::ChooseCardConstrained`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave27b_change_zone_constrained(
        &mut self,
        p: &mut Params,
        origin: &str,
        destination: &str,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || (!p.has("DifferentNames") && !p.has("AtRandom"))
            || !matches!(origin, "Graveyard" | "Hand" | "Exile" | "Battlefield")
            || p.has("RememberChanged")
            || p.has("WithCountersType")
            || p.has("WithCountersAmount")
            || p.has("StaticEffect")
            || p.has("ExileFaceDown")
            || p.has("Imprint")
            || p.has("AttachedTo")
            || p.has("AttachedToPlayer")
            || p.has("Attacking")
        {
            return None;
        }
        let at_random = match p.take("AtRandom").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let distinct_names = match p.take("DifferentNames").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !at_random && !distinct_names {
            return None;
        }
        p.take("IsCurse");
        p.take("SelectPrompt");
        p.claim_label("ChangeTypeDesc", "ChangeType");
        p.claim_label("ChangeNumDesc", "ChangeNum");
        if let Some(h) = p.take("Hidden")
            && h != "True"
        {
            return None;
        }
        if !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
            || !matches!(p.take("Chooser").as_deref(), None | Some("You"))
        {
            return None;
        }
        if target.is_some_and(|t| t != "TargetSpec::AnyPlayer") {
            return None;
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") if destination == "Battlefield" => true,
            Some(_) => return None,
        };
        let gain_control = match p.take("GainControl").as_deref() {
            None | Some("False") => false,
            Some("True") if destination == "Battlefield" => true,
            Some(_) => return None,
        };
        let raw_valid = p.take("ChangeType").unwrap_or_else(|| "Card".to_string());
        if raw_valid.contains("ExiledWith") || raw_valid.contains("IsRemembered") {
            return None;
        }
        let (zone_expr, is_own_zone, filter_expr) = match origin {
            "Graveyard" => {
                let (owner_rel, stripped) = Self::wave27b_strip_zone_owner(&raw_valid)?;
                let rel = owner_rel.unwrap_or("PlayerRel::You");
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                (
                    format!("ChooseZone::Graveyard({rel})"),
                    rel == "PlayerRel::You",
                    expr,
                )
            }
            "Hand" => {
                let stripped = Self::wave27b_strip_you_owner(&raw_valid);
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                ("ChooseZone::Hand(PlayerRel::You)".to_string(), true, expr)
            }
            "Exile" => {
                let (owner_rel, stripped) = Self::wave27b_strip_zone_owner(&raw_valid)?;
                let rel = owner_rel.unwrap_or("PlayerRel::You");
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                (
                    format!("ChooseZone::Exile({rel})"),
                    rel == "PlayerRel::You",
                    expr,
                )
            }
            "Battlefield" => {
                let expr = self.filter_expr(&raw_valid)?;
                ("ChooseZone::Battlefield".to_string(), true, expr)
            }
            _ => return None,
        };
        if destination == "Battlefield" && !is_own_zone && !gain_control {
            return None;
        }
        if destination == "Graveyard" && origin == "Hand" {
            // Hand -> Graveyard requires Cause::Discard, not Cause::Effect.
            return None;
        }
        if destination == "Exile" && self.has_linked_exile_reader() {
            return None;
        }
        let movement = match destination {
            "Battlefield" => format!("ChooseMove::Battlefield {{ tapped: {tapped} }}"),
            "Hand" => "ChooseMove::Hand".to_string(),
            "Graveyard" => "ChooseMove::Graveyard".to_string(),
            "Exile" => "ChooseMove::Exile".to_string(),
            "Library" => match p.take("LibraryPosition").as_deref() {
                Some("0") => "ChooseMove::TopOfLibrary".to_string(),
                Some("-1") => "ChooseMove::BottomOfLibrary".to_string(),
                _ => return None,
            },
            _ => return None,
        };
        let raw_num = p.take("ChangeNum").unwrap_or_else(|| "1".to_string());
        let any_number_distinct = distinct_names
            && origin == "Graveyard"
            && self.svars.get(raw_num.trim()).is_some_and(|def| {
                def.trim()
                    .strip_prefix("Count$ValidGraveyard ")
                    .and_then(|rest| rest.strip_suffix("$DifferentCardNames"))
                    .is_some_and(|v| v.trim() == raw_valid.trim())
            });
        let (min, max) = if at_random {
            p.take("Mandatory");
            let max = self.amount_or_count_ext(&raw_num)?;
            if max == "Amount::Fixed(0)" {
                return None;
            }
            (0u8, max)
        } else if any_number_distinct {
            p.take("Optional");
            (0u8, "Amount::Fixed(255)".to_string())
        } else {
            let opt = self.wave27b_untargeted_optional(p, origin)?;
            self.wave27b_choice_bounds(&raw_num, opt)?
        };
        let filter_name = self.body.filter_static("CHOOSE_CONSTRAINED", &filter_expr);
        Some(vec![format!(
            "Effect::ChooseCardConstrained {{ chooser: PlayerRel::You, zone: {zone_expr}, \
             filter: &{filter_name}, min: {min}, max: {max}, at_random: {at_random}, \
             distinct_names: {distinct_names}, reveal_chosen: false, movement: {movement}, \
             then: &[] }}"
        )])
    }

    /// `ChooseCard` with `AtRandom$ True` and/or `DifferentNames$ True` ->
    /// `Effect::ChooseCardConstrained` (*Cinderheart Giant*, *Deadbridge Chant*,
    /// *Last One Standing*, *Path of Mettle // Metzali, Tower of Triumph*,
    /// *The Nipton Lottery*).
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave27b_choose_card_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<()> {
        p.take("ChoiceTitle");
        p.take("ChoiceDesc");
        p.take("IsCurse");
        if p.has("AllCards")
            || p.has("IncludeSpellsOnStack")
            || p.has("RememberChosen")
            || p.has("ChosenMap")
            || p.has("LockInText")
            || p.entries.iter().any(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        let at_random = match p.take("AtRandom").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`ChooseCard` with `AtRandom$ {other}`")),
        };
        let distinct_names = match p.take("DifferentNames").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`ChooseCard` with `DifferentNames$ {other}`"));
            }
        };
        if !at_random && !distinct_names {
            return None;
        }
        let raw_defined = p.take("Defined");
        let chooser =
            self.wave11_player_of_line(raw_defined.as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(chooser) {
            return self.deny(format!(
                "constrained `ChooseCard` for multi-seat `{chooser}`"
            ));
        }
        let reveal_flag = match p.take("Reveal").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`ChooseCard` with `Reveal$ {other}`")),
        };
        let choices_raw = p.take("Choices")?;
        if choices_raw.contains("IsRemembered")
            || choices_raw.contains("IsImprinted")
            || choices_raw.contains("Remembered")
            || choices_raw.contains("Targeted")
            || choices_raw.contains("ExiledWith")
        {
            return self.deny(format!("`ChooseCard` with `Choices$ {choices_raw}`"));
        }
        let origin_name = match p.take("ChoiceZone").as_deref() {
            None | Some("Battlefield") => "Battlefield",
            Some("Graveyard") => "Graveyard",
            Some("Exile") => "Exile",
            Some("Hand") => "Hand",
            Some(other) => return self.deny(format!("`ChooseCard` in `ChoiceZone$ {other}`")),
        };
        let (zone, is_own_zone, filter_expr) = match origin_name {
            "Battlefield" => (
                "ChooseZone::Battlefield".to_string(),
                true,
                self.filter_expr(&choices_raw)?,
            ),
            "Graveyard" => {
                let (owner_rel, stripped) = Self::wave27b_strip_zone_owner(&choices_raw)?;
                let rel = owner_rel.unwrap_or("PlayerRel::EachPlayer");
                (
                    format!("ChooseZone::Graveyard({rel})"),
                    rel == "PlayerRel::You",
                    self.filter_expr_in_zone(&stripped, true)?,
                )
            }
            "Exile" => {
                let (owner_rel, stripped) = Self::wave27b_strip_zone_owner(&choices_raw)?;
                let rel = owner_rel.unwrap_or("PlayerRel::EachPlayer");
                (
                    format!("ChooseZone::Exile({rel})"),
                    rel == "PlayerRel::You",
                    self.filter_expr_in_zone(&stripped, true)?,
                )
            }
            "Hand" => {
                let (owner_rel, stripped) = Self::wave27b_strip_zone_owner(&choices_raw)?;
                let rel = owner_rel?;
                if !Self::is_single_player_rel(rel) || rel != chooser {
                    return None;
                }
                (
                    format!("ChooseZone::Hand({rel})"),
                    rel == "PlayerRel::You",
                    self.filter_expr_in_zone(&stripped, true)?,
                )
            }
            _ => unreachable!(),
        };
        let mandatory = match p.take("Mandatory").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`ChooseCard` with `Mandatory$ {other}`")),
        };
        let raw_amount = p.take("Amount");
        let max_expr = match raw_amount.as_deref() {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => self.amount_or_count_ext(raw)?,
        };
        if max_expr == "Amount::Fixed(0)" {
            return None;
        }
        let min: u8 = if at_random {
            p.take("MinAmount");
            1
        } else if let Some(raw_min) = p.take("MinAmount") {
            raw_min.trim().parse::<u8>().ok()?
        } else if mandatory {
            match raw_amount.as_deref() {
                None => 1,
                Some(raw) => raw.trim().parse::<u8>().unwrap_or(u8::MAX),
            }
        } else {
            0
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChooseCard.{key}`"));
        }
        let sub_name = sub?;
        let (movement, then_effects, trailing_sub) = self.wave27b_constrained_followup(
            sub_name,
            origin_name,
            is_own_zone,
            &choices_raw,
            &max_expr,
            chain.target.is_some() || chain.second_target.is_some(),
        )?;
        let filter_name = self.body.filter_static("CHOOSE_CONSTRAINED", &filter_expr);
        let reveal_chosen = reveal_flag && origin_name == "Hand";
        let then_joined = then_effects.join(", ");
        chain.effects.push(format!(
            "Effect::ChooseCardConstrained {{ chooser: {chooser}, zone: {zone}, \
             filter: &{filter_name}, min: {min}, max: {max_expr}, at_random: {at_random}, \
             distinct_names: {distinct_names}, reveal_chosen: {reveal_chosen}, \
             movement: {movement}, then: &[{then_joined}] }}"
        ));
        self.chain_sub_ability(trailing_sub, chain)
    }

    /// Follow-up parser for `wave27b_choose_card_chain`.
    #[allow(clippy::too_many_lines)]
    fn wave27b_constrained_followup(
        &mut self,
        sub_name: &str,
        origin_name: &str,
        is_own_zone: bool,
        choices_raw: &str,
        max_expr: &str,
        has_outer_target: bool,
    ) -> Option<(String, Vec<String>, Option<String>)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();

        // 1. Direct `DealDamage | Defined$ ChosenCard` (*Cinderheart Giant*).
        if api == "DealDamage"
            && origin_name == "Battlefield"
            && max_expr == "Amount::Fixed(1)"
            && sp.peek("Defined") == Some("ChosenCard")
        {
            sp.take("Defined");
            sp.take("IsCurse");
            let amount = self.amount_or_count_ext(&sp.take("NumDmg")?)?;
            let after = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let trailing = self.wave27b_strip_chosen_cleanup(after);
            return Some((
                "ChooseMove::None".to_string(),
                vec![format!(
                    "Effect::DealDamage {{ amount: {amount}, target: TargetSpec::EventObject }}"
                )],
                trailing,
            ));
        }

        // 2. Direct `Destroy | Defined$ ChosenCard` (*Metzali, Tower of Triumph*).
        if api == "Destroy"
            && origin_name == "Battlefield"
            && max_expr == "Amount::Fixed(1)"
            && sp.peek("Defined") == Some("ChosenCard")
        {
            sp.take("Defined");
            sp.take("IsCurse");
            let no_regen = match sp.take("NoRegen").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let after = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let trailing = self.wave27b_strip_chosen_cleanup(after);
            let fx = if no_regen {
                "Effect::destroy_no_regen(TargetSpec::EventObject)".to_string()
            } else {
                "Effect::destroy(TargetSpec::EventObject)".to_string()
            };
            return Some(("ChooseMove::None".to_string(), vec![fx], trailing));
        }

        // 3. Direct `Discard | Mode$ Defined | DefinedCards$ ChosenCard`.
        if api == "Discard"
            && origin_name == "Hand"
            && sp.peek("Mode") == Some("Defined")
            && sp.peek("DefinedCards") == Some("ChosenCard")
        {
            sp.take("Mode");
            sp.take("DefinedCards");
            sp.take("Defined");
            sp.take("IsCurse");
            let after = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let trailing = self.wave27b_strip_chosen_cleanup(after);
            return Some((
                "ChooseMove::None".to_string(),
                vec!["Effect::DiscardDefined { cards: DefinedObjects::Found }".to_string()],
                trailing,
            ));
        }

        // 4. Direct `ChangeZone | Defined$ ChosenCard` — single unconditional move,
        // or 2-branch conditional split (*Deadbridge Chant*: creature -> battlefield,
        // else -> hand).
        if api == "ChangeZone"
            && sp.peek("Defined") == Some("ChosenCard")
            && sp.peek("Origin") == Some(origin_name)
        {
            if sp.peek("ConditionDefined") == Some("ChosenCard")
                && origin_name == "Graveyard"
                && is_own_zone
                && max_expr == "Amount::Fixed(1)"
            {
                return self.wave27b_conditional_graveyard_split(sp);
            }
            sp.take("Defined");
            sp.take("Origin");
            sp.take("IsCurse");
            let tapped = match sp.take("Tapped").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let gain = match sp.take("GainControl").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let after = sp.take("SubAbility");
            let mv = match sp.take("Destination").as_deref() {
                Some("Hand") if !tapped && !gain => "ChooseMove::Hand".to_string(),
                Some("Graveyard") if !tapped && !gain && origin_name != "Hand" => {
                    "ChooseMove::Graveyard".to_string()
                }
                Some("Exile") if !tapped && !gain && !self.has_linked_exile_reader() => {
                    "ChooseMove::Exile".to_string()
                }
                Some("Battlefield") if is_own_zone || gain => {
                    format!("ChooseMove::Battlefield {{ tapped: {tapped} }}")
                }
                _ => return None,
            };
            if !sp.exhausted() {
                return None;
            }
            let trailing = self.wave27b_strip_chosen_cleanup(after);
            return Some((mv, Vec::new(), trailing));
        }

        // 5. `DestroyAll | ValidCards$ <matching>.nonChosenCard`
        // (*Last One Standing*): uses `Filter::Reference` inside `then` (safe
        // only with no outer target and `max == 1`).
        if api == "DestroyAll"
            && origin_name == "Battlefield"
            && !has_outer_target
            && max_expr == "Amount::Fixed(1)"
            && let Some(valid) = sp.peek("ValidCards").map(str::to_string)
            && let Some(base) = valid.strip_suffix(".nonChosenCard")
            && base == choices_raw
        {
            sp.take("ValidCards");
            sp.take("IsCurse");
            let no_regen = match sp.take("NoRegen").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let after = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let base_expr = self.filter_expr(base)?;
            let not_chosen =
                format!("Filter::And(&[{base_expr}, Filter::Not(&Filter::Reference)])");
            let name = self.body.filter_static("NOT_CHOSEN", &not_chosen);
            let trailing = self.wave27b_strip_chosen_cleanup(after);
            return Some((
                "ChooseMove::None".to_string(),
                vec![format!(
                    "Effect::DestroyAll {{ filter: &{name}, no_regen: {no_regen} }}"
                )],
                trailing,
            ));
        }

        // 6. `GainControl | Defined$ ChosenCard | LoseControl$ EOT | Untap$ True | AddKWs$ Haste`
        // followed by `DestroyAll | ValidCards$ <matching>.nonChosenCard`
        // (*The Nipton Lottery*).
        if api == "GainControl"
            && origin_name == "Battlefield"
            && !has_outer_target
            && max_expr == "Amount::Fixed(1)"
            && sp.peek("Defined") == Some("ChosenCard")
            && sp.peek("LoseControl") == Some("EOT")
        {
            sp.take("Defined");
            sp.take("LoseControl");
            sp.take("IsCurse");
            let untap = match sp.take("Untap").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let kw = match sp.take("AddKWs").as_deref() {
                None => None,
                Some("Haste") => Some("KeywordSet::HASTE"),
                Some(_) => return None,
            };
            let next = sp.take("SubAbility");
            if !sp.exhausted() {
                return None;
            }
            let mut then_fx = vec![
                "Effect::continuous(&Filter::Reference, Modifier::GainControl, Duration::UntilEndOfTurn)"
                    .to_string(),
            ];
            if untap {
                then_fx.push("Effect::UntapAll { filter: &Filter::Reference }".to_string());
            }
            if let Some(k) = kw {
                then_fx.push(format!(
                    "Effect::continuous(&Filter::Reference, Modifier::AddKeyword({k}), Duration::UntilEndOfTurn)"
                ));
            }
            let trailing = if let Some(next_name) = next {
                let next_body = self.svars.get(next_name.trim())?.clone();
                let (napi, mut np) = Params::parse(&next_body)?;
                np.drop_prose();
                if napi == "DestroyAll"
                    && let Some(valid) = np.take("ValidCards")
                    && let Some(base) = valid.strip_suffix(".nonChosenCard")
                    && base == choices_raw
                {
                    let no_regen = match np.take("NoRegen").as_deref() {
                        None | Some("False") => false,
                        Some("True") => true,
                        Some(_) => return None,
                    };
                    let after2 = np.take("SubAbility");
                    if !np.exhausted() {
                        return None;
                    }
                    let base_expr = self.filter_expr(base)?;
                    let not_chosen =
                        format!("Filter::And(&[{base_expr}, Filter::Not(&Filter::Reference)])");
                    let name = self.body.filter_static("NOT_CHOSEN", &not_chosen);
                    then_fx.push(format!(
                        "Effect::DestroyAll {{ filter: &{name}, no_regen: {no_regen} }}"
                    ));
                    self.wave27b_strip_chosen_cleanup(after2)
                } else {
                    self.wave27b_strip_chosen_cleanup(Some(next_name))
                }
            } else {
                None
            };
            return Some(("ChooseMove::None".to_string(), then_fx, trailing));
        }

        None
    }

    /// Conditional 2-branch graveyard split on `ChosenCard` (*Deadbridge Chant*):
    /// if `ConditionPresent$ <F>` with `GE1` -> Battlefield, else (`EQ0`) -> Hand.
    fn wave27b_conditional_graveyard_split(
        &mut self,
        mut sp1: Params,
    ) -> Option<(String, Vec<String>, Option<String>)> {
        sp1.take("Defined");
        sp1.take("Origin");
        sp1.take("ConditionDefined");
        let pred = sp1.take("ConditionPresent")?;
        if !matches!(sp1.take("ConditionCompare").as_deref(), Some("GE1" | "EQ1")) {
            return None;
        }
        if sp1.take("Destination").as_deref() != Some("Battlefield") {
            return None;
        }
        let next_name = sp1.take("SubAbility")?;
        if !sp1.exhausted() {
            return None;
        }
        let next_body = self.svars.get(next_name.trim())?.clone();
        let (api2, mut sp2) = Params::parse(&next_body)?;
        sp2.drop_prose();
        if api2 != "ChangeZone"
            || sp2.take("Defined").as_deref() != Some("ChosenCard")
            || sp2.take("Origin").as_deref() != Some("Graveyard")
            || sp2.take("Destination").as_deref() != Some("Hand")
            || sp2.take("ConditionDefined").as_deref() != Some("ChosenCard")
            || sp2.take("ConditionPresent").as_deref() != Some(pred.as_str())
            || sp2.take("ConditionCompare").as_deref() != Some("EQ0")
        {
            return None;
        }
        let after = sp2.take("SubAbility");
        if !sp2.exhausted() {
            return None;
        }
        let pred_expr = self.filter_expr_in_zone(&pred, true)?;
        let pos_name = self.body.filter_static("CHOSEN_IS", &pred_expr);
        let neg_expr = format!("Filter::Not(&{pos_name})");
        let neg_name = self.body.filter_static("CHOSEN_NOT", &neg_expr);
        let trailing = self.wave27b_strip_chosen_cleanup(after);
        Some((
            "ChooseMove::None".to_string(),
            vec![
                format!(
                    "Effect::IfEventObjectMatches {{ filter: &{pos_name}, \
                     then: &[Effect::reanimate(TargetSpec::EventObject)] }}"
                ),
                format!(
                    "Effect::IfEventObjectMatches {{ filter: &{neg_name}, \
                     then: &[Effect::GraveyardToHand {{ target: TargetSpec::EventObject }}] }}"
                ),
            ],
            trailing,
        ))
    }

    /// Hand `ChooseCard` follow-up `DB$ Discard | Mode$ Defined` ->
    /// `Effect::DiscardDefined { cards: DefinedObjects::Found }` in `then`
    /// (`DefinedCards$ ChosenCard`) or `for_unchosen` (*Monomania*:
    /// `DefinedCards$ ValidHand Card.!ChosenCard+OwnedBy Targeted`).
    pub(super) fn wave27b_choose_card_discard_followup(
        &self,
        mut sp: Params,
        zone_owner: Option<&'static str>,
        choices_raw: &str,
    ) -> Option<(Vec<String>, Vec<String>, Option<String>)> {
        if sp.take("Mode").as_deref() != Some("Defined") {
            return None;
        }
        sp.take("IsCurse");
        let raw_defined = sp.take("Defined");
        let cards_raw = sp.take("DefinedCards")?;
        let after = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        let trailing = self.wave27b_strip_chosen_cleanup(after);
        if cards_raw == "ChosenCard" {
            return Some((
                vec!["Effect::DiscardDefined { cards: DefinedObjects::Found }".to_string()],
                Vec::new(),
                trailing,
            ));
        }
        if choices_raw == "Card"
            && let Some(rest) = cards_raw.strip_prefix("ValidHand ")
        {
            let owner_matches = matches!(
                (rest, zone_owner, raw_defined.as_deref()),
                (
                    "Card.!ChosenCard+OwnedBy Targeted" | "Card.nonChosenCard+OwnedBy Targeted",
                    Some("PlayerRel::Chosen"),
                    Some("Targeted" | "TargetedPlayer"),
                ) | (
                    "Card.!ChosenCard+YouOwn" | "Card.nonChosenCard+YouOwn",
                    Some("PlayerRel::You"),
                    None | Some("You"),
                )
            );
            if owner_matches {
                return Some((
                    Vec::new(),
                    vec!["Effect::DiscardDefined { cards: DefinedObjects::Found }".to_string()],
                    trailing,
                ));
            }
        }
        None
    }

    /// `DB$ Attach` extensions mapped onto `Effect::AttachChosen` (CR 701.3a–b)
    /// and `Effect::AttachSlot` (`Object$ ThisTargetedCard`):
    /// - `Choices$ <filter> | Defined$ <host>` (*Yuffie*, *Winter Soldier*,
    ///   *Resolute Strike*, *Vow to Erebor*, *Zack Fair*)
    /// - `Object$ Valid <filter> | Defined$ <host>` (*Beatrix*, *Battlefield
    ///   Improvisation*, *Armed and Armored*)
    /// - `Object$ ThisTargetedCard | Defined$ Self` (*Amy Rose*)
    #[allow(clippy::too_many_lines)]
    pub(super) fn wave27b_attach_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        optional: bool,
    ) -> Option<Vec<String>> {
        // 1. `Object$ ThisTargetedCard | Defined$ Self` (*Amy Rose*).
        if p.peek("Object") == Some("ThisTargetedCard") && p.peek("Defined") == Some("Self") {
            if self.on_a_spell
                || !targets_here
                || !target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            {
                return None;
            }
            p.take("Object");
            p.take("Defined");
            return Some(vec![format!(
                "Effect::AttachSlot {{ object: TargetSlot::First, to: TargetSlot::This, \
                 optional: {optional} }}"
            )]);
        }

        if p.has("RememberAttached") || p.has("PlayerChoices") || p.has("Chooser") || p.has("Move")
        {
            return None;
        }
        p.take("ChoiceTitle");
        p.take("IsCurse");

        // 2. Attachment set & count:
        //    - `Choices$ <filter>` (without `Object$`) => choose up to/exactly 1
        //    - `Object$ Valid <filter>` (without `Choices$`) => choose any number
        let (att_raw, max_expr) = match (p.peek("Choices"), p.peek("Object")) {
            (Some(_), None) => (p.take("Choices")?, "Amount::Fixed(1)"),
            (None, Some(obj)) if obj.starts_with("Valid ") => {
                let raw = p.take("Object")?;
                (
                    raw.strip_prefix("Valid ")?.to_string(),
                    "Amount::Fixed(255)",
                )
            }
            _ => return None,
        };
        if att_raw.contains("Remembered")
            || att_raw.contains("Imprinted")
            || att_raw.contains("ChosenCard")
            || att_raw.contains("Targeted")
        {
            return None;
        }
        let att_expr = self.filter_expr(&att_raw)?;
        let att_name = self.body.filter_static("ATTACH_CHOSEN", &att_expr);

        // 3. Host (`Defined$`).
        let def_raw = p.take("Defined")?;
        if def_raw == "Self" {
            return Some(vec![format!(
                "Effect::AttachChosen {{ filter: &{att_name}, to: TargetSpec::ThisObject, \
                 max: {max_expr}, optional: {optional} }}"
            )]);
        }
        if matches!(def_raw.as_str(), "Targeted" | "ParentTarget") {
            let tgt = target.filter(|t| t.starts_with("TargetSpec::Object("))?;
            return Some(vec![format!(
                "Effect::AttachChosen {{ filter: &{att_name}, to: {tgt}, \
                 max: {max_expr}, optional: {optional} }}"
            )]);
        }
        // `Defined$ ParentTarget.<qual>` (e.g. *Battlefield Improvisation*:
        // `Defined$ ParentTarget.attacking`).
        if let Some(qual) = def_raw.strip_prefix("ParentTarget.")
            && !targets_here
            && let Some(tgt) = target.filter(|t| t.starts_with("TargetSpec::Object("))
        {
            let cond_expr = self.filter_expr(&format!("Card.{qual}"))?;
            let cond_name = self.body.filter_static("ATTACH_COND", &cond_expr);
            return Some(vec![format!(
                "Effect::IfTargetMatches {{ filter: &{cond_name}, \
                 then: &[Effect::AttachChosen {{ filter: &{att_name}, to: {tgt}, \
                 max: {max_expr}, optional: {optional} }}] }}"
            )]);
        }
        // `Defined$ Valid <host_filter>` (*Armed and Armored*: "Choose a Dwarf
        // you control. Attach any number of Equipment you control to it.").
        if let Some(host_valid) = def_raw.strip_prefix("Valid ")
            && !targets_here
            && !host_valid.contains("Remembered")
            && !host_valid.contains("ChosenCard")
            && !host_valid.contains("Targeted")
        {
            let host_expr = self.filter_expr(host_valid)?;
            let host_name = self.body.filter_static("ATTACH_HOST", &host_expr);
            return Some(vec![format!(
                "Effect::ChooseCard {{ chooser: PlayerRel::You, \
                 zone: ChooseZone::Battlefield, filter: &{host_name}, \
                 min: 1, max: Amount::Fixed(1), movement: ChooseMove::None, \
                 then: &[Effect::AttachChosen {{ filter: &{att_name}, \
                 to: TargetSpec::EventObject, max: {max_expr}, optional: {optional} }}] }}"
            )]);
        }
        if self.is_event_object_defined(&def_raw) && !targets_here {
            return Some(vec![format!(
                "Effect::AttachChosen {{ filter: &{att_name}, to: TargetSpec::EventObject, \
                 max: {max_expr}, optional: {optional} }}"
            )]);
        }

        None
    }

    /// `Token | RememberTokens$ True | SubAbility$ DBAttach` where `DBAttach`
    /// is `DB$ Attach | Choices$ <filter> | Defined$ Remembered | Optional$ ...`
    /// (*Nahiri, the Lithomancer*, *Nahiri, Heir of the Ancients*, *Reckless
    /// Crew*).
    pub(super) fn wave27b_token_then_attach_chosen(
        &mut self,
        np: &mut Params,
        token: &str,
    ) -> Option<String> {
        if np.has("Object")
            || np.has("RememberAttached")
            || np.has("PlayerChoices")
            || np.has("Chooser")
        {
            return None;
        }
        np.take("ChoiceTitle");
        let optional = match np.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let choices_raw = np.take("Choices")?;
        if choices_raw.contains("Remembered")
            || choices_raw.contains("Imprinted")
            || choices_raw.contains("ChosenCard")
        {
            return None;
        }
        let att_expr = self.filter_expr(&choices_raw)?;
        let att_name = self.body.filter_static("ATTACH_CHOSEN", &att_expr);
        Some(format!(
            "Effect::CaptureAffected {{ \
             first: &[Effect::CreateToken {{ token: {token} }}], \
             then: &[Effect::ForEachRemembered {{ filter: &Filter::Any, \
             effects: &[Effect::AttachChosen {{ filter: &{att_name}, \
             to: TargetSpec::EventObject, max: Amount::Fixed(1), optional: {optional} }}] }}] }}"
        ))
    }

    /// `DB$ Discard | Mode$ Defined | DefinedCards$ ...` ->
    /// `Effect::DiscardDefined` (CR 701.9a).
    ///
    /// Refuses `Remembered` / `Imprinted` / `IsRemembered` / `ChosenCard`
    /// (outside a `ChooseCard` chain) and `UnlessCost$`.
    pub(super) fn wave27b_discard_defined(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.take("Mode").as_deref() != Some("Defined") {
            return None;
        }
        p.take("IsCurse");
        p.take("DiscardMessage");
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let raw_defined_player = p.take("Defined");
        let cards_raw = p.take("DefinedCards")?;

        let cards_expr = if cards_raw == "Self" {
            if !matches!(raw_defined_player.as_deref(), None | Some("You")) {
                return None;
            }
            "DefinedObjects::Spec(TargetSpec::ThisObject)".to_string()
        } else if self.is_event_object_defined(&cards_raw) {
            "DefinedObjects::Spec(TargetSpec::EventObject)".to_string()
        } else {
            let valid = cards_raw.strip_prefix("ValidHand ")?;
            if valid.contains("IsRemembered")
                || valid.contains("ChosenCard")
                || valid.contains("NamedCard")
                || valid.contains("Imprinted")
                || valid.contains("RememberedPlayerCtrl")
            {
                return None;
            }
            let (owner_opt, stripped) = Self::wave27b_strip_zone_owner(valid)?;
            let who = match (owner_opt, raw_defined_player.as_deref()) {
                (Some(a), Some(raw)) => {
                    let b = self.wave11_player_of_line(Some(raw), target, targets_a_player)?;
                    if a != b {
                        return None;
                    }
                    a
                }
                (Some(a), None) => a,
                (None, other) => self.wave11_player_of_line(other, target, targets_a_player)?,
            };
            let zone = match who {
                "PlayerRel::You" => "ZoneSel::HandYou".to_string(),
                "PlayerRel::Chosen" => "ZoneSel::HandChosen".to_string(),
                "PlayerRel::ActivePlayer" => "ZoneSel::HandActivePlayer".to_string(),
                other => format!("ZoneSel::HandOf({other})"),
            };
            let filter_expr = self.filter_expr_in_zone(&stripped, true)?;
            let filter_name = self.body.filter_static("DISCARD_DEFINED", &filter_expr);
            format!("DefinedObjects::AllInZone {{ zone: {zone}, filter: &{filter_name} }}")
        };

        let discard = format!("Effect::DiscardDefined {{ cards: {cards_expr} }}");
        if optional {
            Some(vec![format!("Effect::MayDo {{ effects: &[{discard}] }}")])
        } else {
            Some(vec![discard])
        }
    }

    // -------------------------------------------------------------------------
    // Shared Wave 27b helpers
    // -------------------------------------------------------------------------

    fn wave27b_attach_host(
        &mut self,
        raw: &str,
        is_player: bool,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<String> {
        let trimmed = raw.trim();
        if is_player {
            return match trimmed {
                "You" => Some("AttachHost::Spec(TargetSpec::Player(PlayerRel::You))".to_string()),
                "Targeted" | "ParentTarget" | "ThisTargetedCard" if targets_a_player => {
                    let tgt = target?;
                    Some(format!("AttachHost::Spec({tgt})"))
                }
                "EnchantedPlayer" if !self.on_a_spell => Some(
                    "AttachHost::Spec(TargetSpec::Player(PlayerRel::EnchantedPlayer))".to_string(),
                ),
                "Opponent" | "Player.Opponent" => {
                    Some("AttachHost::ChosenPlayer(PlayerRel::EachOpponent)".to_string())
                }
                "Player" => Some("AttachHost::ChosenPlayer(PlayerRel::EachPlayer)".to_string()),
                other => {
                    let rel = self.wave11_player_of_line(Some(other), target, targets_a_player)?;
                    if Self::is_single_player_rel(rel) {
                        Some(format!("AttachHost::Spec(TargetSpec::Player({rel}))"))
                    } else {
                        Some(format!("AttachHost::ChosenPlayer({rel})"))
                    }
                }
            };
        }

        if trimmed == "Self" {
            return Some("AttachHost::Spec(TargetSpec::ThisObject)".to_string());
        }
        if matches!(trimmed, "Targeted" | "ThisTargetedCard" | "ParentTarget") {
            let tgt = target.filter(|t| {
                !t.starts_with("TargetSpec::CardInGraveyard") && *t != "TargetSpec::AnyPlayer"
            })?;
            return Some(format!("AttachHost::Spec({tgt})"));
        }
        if self.is_event_object_defined(trimmed) {
            return Some("AttachHost::Spec(TargetSpec::EventObject)".to_string());
        }
        if trimmed == "TriggeredDefendingPlayer" && self.trigger_mode.is_some() {
            return Some(
                "AttachHost::Spec(TargetSpec::Player(PlayerRel::DefendingPlayer))".to_string(),
            );
        }
        if trimmed.contains("Remembered")
            || trimmed.contains("Imprinted")
            || trimmed.contains("ChosenCard")
            || trimmed.contains("Targeted")
            || trimmed.contains("Triggered")
            || trimmed.contains("ParentTarget")
            || trimmed.contains("shares")
        {
            return None;
        }
        let clean = trimmed.strip_prefix("Valid ").unwrap_or(trimmed);
        let expr = self.filter_expr(clean)?;
        let name = self.body.filter_static("ATTACH_HOST", &expr);
        Some(format!("AttachHost::ChosenPermanent(&{name})"))
    }

    fn wave27b_is_player_spec(spec: &str) -> bool {
        matches!(spec, "TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent")
            || spec.starts_with("TargetSpec::Player(")
    }

    fn wave27b_untargeted_optional(&self, p: &mut Params, origin: &str) -> Option<bool> {
        let mandatory = match p.take("Mandatory").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        match p.take("Optional").as_deref() {
            Some("True" | "You") if !mandatory => Some(true),
            Some("False") => Some(false),
            Some(_) => None,
            None if mandatory => Some(false),
            None => {
                let lower = self.oracle.to_ascii_lowercase();
                if lower.contains("may put")
                    || lower.contains("you may return")
                    || (origin == "Hand" && lower.contains("put any number of"))
                {
                    Some(true)
                } else {
                    None
                }
            }
        }
    }

    fn wave27b_choice_bounds(&mut self, raw_num: &str, optional: bool) -> Option<(u8, String)> {
        if optional {
            let max = self.amount_or_count_ext(raw_num)?;
            if max == "Amount::Fixed(0)" {
                return None;
            }
            Some((0, max))
        } else {
            let n: u8 = self.plain_num(raw_num)?.try_into().ok()?;
            if n == 0 {
                return None;
            }
            Some((n, format!("Amount::Fixed({n})")))
        }
    }

    fn wave27b_strip_you_owner(valid: &str) -> String {
        valid
            .split(',')
            .map(|alt| {
                let alt = alt.trim();
                let Some((base, rest)) = alt.split_once('.') else {
                    return alt.to_string();
                };
                let kept: Vec<&str> = rest
                    .split('+')
                    .filter(|a| !matches!(*a, "YouOwn" | "YouCtrl"))
                    .collect();
                if kept.is_empty() {
                    base.to_string()
                } else {
                    format!("{base}.{}", kept.join("+"))
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    fn wave27b_strip_zone_owner(valid: &str) -> Option<(Option<&'static str>, String)> {
        let mut owner: Option<&'static str> = None;
        let mut out = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            let Some((base, rest)) = alt.split_once('.') else {
                out.push(alt.to_string());
                continue;
            };
            let mut kept = Vec::new();
            for atom in rest.split('+') {
                let mapped = match atom {
                    "YouOwn" | "YouCtrl" => Some("PlayerRel::You"),
                    "OppOwn" | "OppCtrl" => Some("PlayerRel::EachOpponent"),
                    "OwnedBy Targeted" | "TargetedPlayerOwn" => Some("PlayerRel::Chosen"),
                    _ => None,
                };
                if let Some(rel) = mapped {
                    if let Some(prev) = owner
                        && prev != rel
                    {
                        return None;
                    }
                    owner = Some(rel);
                } else {
                    kept.push(atom);
                }
            }
            if kept.is_empty() {
                out.push(base.to_string());
            } else {
                out.push(format!("{base}.{}", kept.join("+")));
            }
        }
        Some((owner, out.join(",")))
    }

    fn wave27b_graveyard_zone_sel(valid: &str) -> Option<(&'static str, bool, String)> {
        let (owner, stripped) = Self::wave27b_strip_zone_owner(valid)?;
        match owner {
            Some("PlayerRel::You") => Some(("ZoneSel::GraveyardYou", false, stripped)),
            Some("PlayerRel::EachOpponent") => {
                Some(("ZoneSel::GraveyardOpponents", true, stripped))
            }
            None | Some("PlayerRel::EachPlayer") => Some(("ZoneSel::GraveyardAll", true, stripped)),
            _ => None,
        }
    }

    fn wave27b_strip_chosen_cleanup(&self, after: Option<String>) -> Option<String> {
        let name = after?;
        let Some(body) = self.svars.get(name.trim()) else {
            return Some(name);
        };
        let Some((api, mut cp)) = Params::parse(body) else {
            return Some(name);
        };
        cp.drop_prose();
        if api == "Cleanup" && cp.take("ClearChosenCard").as_deref() == Some("True") {
            let next = cp.take("SubAbility");
            if cp.exhausted() {
                return next;
            }
        }
        Some(name)
    }
}
