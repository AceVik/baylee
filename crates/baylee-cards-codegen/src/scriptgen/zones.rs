//! Zone-change chain and untargeted graveyard/library lowering helpers.

use super::{Params, Tx, chains::LineFlags};

impl Tx<'_> {
    /// Parses omitted `Mandatory$`/`Optional$` on untargeted single-player
    /// `Graveyard -> Battlefield | Hand` lines when the Oracle text explicitly
    /// states a mandatory return from your graveyard.
    pub(super) fn parse_mandatory_graveyard_return_from_oracle(
        oracle: &str,
        destination: &str,
    ) -> Option<bool> {
        let lower = oracle.to_ascii_lowercase();
        if lower.contains("may ")
            || lower.contains("up to ")
            || lower.contains("at random")
            || lower.contains("target")
        {
            return None;
        }
        let matches_clause = match destination {
            "Battlefield" => lower.contains("from your graveyard to the battlefield"),
            "Hand" => lower.contains("from your graveyard to your hand"),
            _ => false,
        };
        matches_clause.then_some(false)
    }

    /// Lowers conditional `Count$Threshold.<hi>.<lo>` or
    /// `Count$wasCastFromGraveyard.<hi>.<lo>` library searches sharing one
    /// `Effect::IfCondition` structure.
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub(super) fn emit_conditional_search_library(
        &mut self,
        p: &mut Params,
        raw_num: Option<&str>,
        destination: &str,
        is_own_library: bool,
        optional: bool,
        mandatory: bool,
        up_to: bool,
        reveal_requested: bool,
        any_number: bool,
        find: &str,
    ) -> Option<Vec<String>> {
        if !is_own_library || any_number {
            return None;
        }
        let def = self.svars.get(raw_num?.trim())?.trim().to_string();
        let (condition, rest) = if let Some(r) = def.strip_prefix("Count$Threshold.") {
            ("Condition::GraveyardCountAtLeast(7)", r)
        } else {
            let r = def.strip_prefix("Count$wasCastFromGraveyard.")?;
            ("Condition::SourceCastFromZone(ZoneRef::Graveyard)", r)
        };
        let (hi_raw, lo_raw) = rest.split_once('.')?;
        let hi = hi_raw.parse::<usize>().ok()?;
        let lo = lo_raw.parse::<usize>().ok()?;
        if !(1..=4).contains(&hi) || !(1..=4).contains(&lo) {
            return None;
        }
        let filter = self.filter_expr_in_zone(&p.take("ChangeType")?, true)?;
        if reveal_requested
            && filter == "Filter::Any"
            && !matches!(destination, "Battlefield" | "Exile" | "Graveyard")
        {
            return self.deny("`ChangeZone` revealing an unrestricted hidden search".into());
        }
        let optional_hi = self.resolve_search_optional(hi, &filter, optional, mandatory, up_to)?;
        let optional_lo = self.resolve_search_optional(lo, &filter, optional, mandatory, up_to)?;
        let name = self.body.filter_static("SEARCH", &filter);
        let finds_hi = vec![find; hi].join(", ");
        let finds_lo = vec![find; lo].join(", ");
        Some(vec![format!(
            "Effect::IfCondition {{ condition: {condition}, then: &[Effect::SearchLibrary {{ filter: &{name}, finds: &[{finds_hi}], optional: {optional_hi} }}], otherwise: &[Effect::SearchLibrary {{ filter: &{name}, finds: &[{finds_lo}], optional: {optional_lo} }}] }}"
        )])
    }

    /// Lowers `ChangeZone | Defined$ Targeted | Destination$ Library | LibraryPosition$ <N/X>`
    /// when `Origin$` is omitted on an already-chosen permanent target
    /// (*Underdark Rift* after `RollDice | ResultSVar$ X`).
    pub(super) fn put_targeted_into_library_beneath_top(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        let target = target?;
        if targets_here
            || !target.starts_with("TargetSpec::Object(")
            || p.has("Origin")
            || p.peek("Defined") != Some("Targeted")
            || p.peek("Destination") != Some("Library")
            || p.has("Shuffle")
            || p.has("DestinationAlternative")
        {
            return None;
        }
        let mut probe = p.clone();
        probe.take("Defined");
        probe.take("Destination");
        probe.take("IsCurse");
        let pos = probe.take("LibraryPosition")?;
        let count = self.beneath_top_library_position(&pos)?;
        if !probe.exhausted() {
            return None;
        }
        *p = probe;
        Some(vec![format!(
            "Effect::PutTargetIntoLibraryBeneathTop {{ count: {count} }}"
        )])
    }

    /// Handles `DB$ Token | ImprintTokens$ True` followed immediately by
    /// `DB$ PutCounter | Defined$ Imprinted` inside
    /// `wave24a_cz_remembered_followup` via `Effect::RepeatEachObject { only_found: true }`
    /// (*Summoner's Sending*).
    pub(super) fn try_cz_remembered_token_imprint_counter(
        &mut self,
        sp: &mut Params,
        next: &mut Option<String>,
        target: Option<&str>,
        this_way_variant: (&'static str, bool, &'static str),
        flags: LineFlags,
    ) -> Option<(Vec<String>, bool)> {
        if sp.peek("ImprintTokens") != Some("True") {
            return None;
        }
        let mut token_p = sp.clone();
        token_p.take("ImprintTokens");
        let mut effs = self.effect_of_with_prior("Token", &mut token_p, target, flags)?;
        if !token_p.exhausted() {
            return None;
        }
        let next_name = next.as_deref()?;
        let next_body = self.svars.get(next_name.trim())?.clone();
        let (n_api, mut np) = Params::parse(&next_body)?;
        np.drop_prose();
        if n_api != "PutCounter" || np.take("Defined").as_deref() != Some("Imprinted") {
            return None;
        }
        let kind = super::counter_kind(&np.take("CounterType")?)?;
        let amount = self.amount_or_count_ext(np.take("CounterNum").as_deref().unwrap_or("1"))?;
        let inner_cond = if np.has("ConditionDefined") {
            if np.take("ConditionDefined").as_deref() != Some("Remembered") {
                return None;
            }
            if let Some(cz) = np.take("ConditionZone")
                && cz != "Exile"
            {
                return None;
            }
            if !matches!(
                np.take("ConditionCompare").as_deref(),
                None | Some("GE1" | "GT0")
            ) {
                return None;
            }
            let present = np.take("ConditionPresent")?;
            np.take("ConditionDescription");
            let (variant, card_zone, prefix) = this_way_variant;
            let filter_ref = if matches!(present.trim(), "Card" | "Permanent") {
                "&Filter::Any".to_string()
            } else {
                let expr = self.filter_expr_in_zone(present.trim(), card_zone)?;
                format!("&{}", self.body.filter_static(prefix, &expr))
            };
            Some(format!(
                "Condition::AmountAtLeast(Amount::ThisWay(ThisWay::{variant}({filter_ref})), 1)"
            ))
        } else {
            None
        };
        let after_counter = np.take("SubAbility");
        if !np.exhausted() {
            return None;
        }
        let counter_loop = format!(
            "Effect::RepeatEachObject {{ zone: ZoneSel::Battlefield, filter: &Filter::Any, only_found: true, then: &[Effect::AddCounterFilter {{ filter: &Filter::Reference, kind: {kind}, amount: {amount} }}] }}"
        );
        if let Some(cond) = inner_cond {
            effs.push(format!(
                "Effect::IfCondition {{ condition: {cond}, then: &[{counter_loop}], otherwise: &[] }}"
            ));
        } else {
            effs.push(counter_loop);
        }
        *sp = token_p;
        *next = after_counter;
        Some((effs, true))
    }
}
