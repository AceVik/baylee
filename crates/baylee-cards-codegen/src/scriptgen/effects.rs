//! Reading the effects that are more than one line: tokens, animation,
//! pumps, zone changes and library searches.

use super::{
    CLUE_TOKEN_SCRIPT, Chain, Params, Tx, card_type_const, color_word, keyword_const, pump_amount,
};
use baylee_core::types::SubtypeKind;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EffectSubject {
    Unbound,
    Targets,
    Source,
    Event,
}

impl Tx<'_> {
    /// `DB$ Sacrifice`: "sacrifice it".
    ///
    /// **Bare** is "sacrifice this" — 111 of the corpus's 895 sacrifice
    /// lines, and [`Effect::SacrificeSelf`] says it exactly. With an
    /// `UnlessCost$` it is the Karoo sentence, "sacrifice it unless you
    /// <pay>", 131 more — read by [`Tx::unless`] for every API, since the
    /// price belongs to the line and not to the sacrifice. What it is
    /// **not** is `Defined$`/`SacValid$`: those name somebody else's
    /// permanent ("each player sacrifices a creature"), a player choice this
    /// DSL has no effect for, and reading them as the source would be a
    /// card that sacrifices the wrong permanent under a
    /// `Coverage::Implemented`.
    pub(super) fn sacrifice_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.take("RememberSacrificed").is_some() {
            return self.deny("a sacrifice naming `RememberSacrificed`".to_string());
        }
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return self.deny("a sacrifice naming `Optional`".to_string()),
        };
        let raw_amount = p.take("Amount");
        if let Some(num) = p.take("ChangeNum")
            && num.trim() != "1"
        {
            return self.deny(format!("a sacrifice with `ChangeNum$ {num}`"));
        }
        p.claim_label("SacMessage", "SacValid");
        let Some(sac_valid) = p.take("SacValid") else {
            if raw_amount.is_some() {
                return self.deny("a sacrifice naming `Amount`".to_string());
            }
            let base = match p.take("Defined").as_deref() {
                None => "Effect::SacrificeSelf",
                Some("Self") if !self.on_a_spell => "Effect::SacrificeSelf",
                Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
                    if self.in_delayed && target.is_none() =>
                {
                    "Effect::SacrificeEvent"
                }
                Some(_) => return self.deny("a sacrifice naming `Defined`".to_string()),
            };
            return Some(vec![if optional {
                format!("Effect::MayDo {{ effects: &[{base}] }}")
            } else {
                base.to_string()
            }]);
        };
        let who = self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let filter = if sac_valid == "Permanent" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr(&sac_valid)?;
            format!("&{}", self.body.filter_static("SAC", &expr))
        };
        let effect = match raw_amount.as_deref().map(str::trim) {
            None | Some("1") => {
                format!("Effect::SacrificeFilter {{ who: {who}, filter: {filter} }}")
            }
            Some(raw) => {
                let amount = self.amount_or_count_ext(raw)?;
                format!(
                    "Effect::SacrificeAmountForPlayers {{ who: {who}, filter: {filter}, amount: \
                     {amount} }}"
                )
            }
        };
        if !optional {
            return Some(vec![effect]);
        }
        if who == "PlayerRel::You" {
            return Some(vec![format!("Effect::MayDo {{ effects: &[{effect}] }}")]);
        }
        if matches!(
            who,
            "PlayerRel::Chosen"
                | "PlayerRel::ControllerOfTarget"
                | "PlayerRel::ControllerOfEvent"
                | "PlayerRel::TargetPlayerOrController"
                | "PlayerRel::EventPlayer"
                | "PlayerRel::DefendingPlayer"
                | "PlayerRel::ActivePlayer"
                | "PlayerRel::OwnerOfTarget"
                | "PlayerRel::ControllerOfAttached"
        ) {
            return Some(vec![format!(
                "Effect::MayDoFor {{ player: {who}, effects: &[{effect}] }}"
            )]);
        }
        self.deny("a sacrifice naming `Optional`".to_string())
    }

    /// `Token`: "create a 1/1 white Soldier creature token".
    ///
    /// The effect names a `&'static TokenDef`, so what this writes is the
    /// constant the **ledger** filed that token under and never a definition
    /// of its own: the id a client keys token art off is a token's place in
    /// `generated_tokens::ALL`, and a literal written into a card file would
    /// have no place there at all. Which is also why this is one of the two
    /// rules that can be refused by something other than the script — a run
    /// with no token corpus has no constant to name.
    ///
    /// Everything the token *is* — its colours, its size, its keywords — is
    /// read by [`crate::tokengen`] from the token script and never from this
    /// line, which carries none of it.
    #[allow(clippy::too_many_lines)]
    pub(super) fn token_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        // Who gets it. The corpus writes `TokenOwner$ You` on 2220 of its
        // 3610 token lines and leaves the key off on 1154 — and absence is
        // **not** a synonym for you: Rootcast Apprenticeship says "target
        // player creates a 1/1 green Squirrel creature token" with no
        // `TokenOwner$` at all, leaning on its own line's target instead.
        // That is the trap [`Self::player_rel_of`] was written for, so the
        // absent key is read the same way it reads one there: as the target
        // where this line targets a player, and as you where it does not.
        let raw_owner = p.take("TokenOwner");
        let is_plain_target_ctrl = raw_owner.as_deref() == Some("TargetedController")
            && target.is_some_and(|t| {
                t.starts_with("TargetSpec::Object(")
                    || t.starts_with("TargetSpec::CardInGraveyard(")
            })
            && matches!(p.peek("TokenAmount"), None | Some("1"))
            && !p.has("TokenTapped")
            && !p.has("TokenAttacking")
            && !p.has("PumpKeywords")
            && !p.has("PumpDuration");
        if is_plain_target_ctrl {
            p.take("TokenAmount");
            let Some(stem) = p.take("TokenScript") else {
                return self.deny("a `Token` effect naming no `TokenScript$`".to_string());
            };
            let Some(tokens) = self.tokens else {
                return self.deny("`Token` with no token scripts to read it against".to_string());
            };
            let Some(body) = tokens.body(&stem, self.cats) else {
                return self.deny(format!("token script `{stem}`"));
            };
            return Some(vec![format!(
                "Effect::CreateTokenForTargetController {{ token: &generated_tokens::{} }}",
                body.constant
            )]);
        }
        let owner = match raw_owner.as_deref() {
            Some("You") => "PlayerRel::You",
            Some("TriggeredCardController") if self.trigger_is_self => "PlayerRel::You",
            other => match self.player_of_line(other, target, targets_a_player) {
                Some(rel) => rel,
                None => {
                    return match other {
                        Some(who) => self.deny(format!("token owner `{who}`")),
                        None => {
                            self.deny("a token created under another player's control".to_string())
                        }
                    };
                }
            },
        };
        let tapped = match p.take("TokenTapped").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("token `TokenTapped$ {other}`")),
        };
        let attacking = match p.take("TokenAttacking").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("token `TokenAttacking$ {other}`")),
        };
        let haste = if p.peek("PumpKeywords") == Some("Haste")
            && matches!(p.peek("PumpDuration"), Some("EOT" | "EndOfTurn"))
        {
            p.take("PumpKeywords");
            p.take("PumpDuration");
            true
        } else {
            false
        };
        let sacrifice_at_eot = if attacking
            && tapped
            && !haste
            && owner == "PlayerRel::You"
            && p.peek("AtEOT") == Some("Sacrifice")
        {
            p.take("AtEOT");
            true
        } else {
            false
        };
        if attacking && (!tapped || haste) {
            return self.deny("an attacking token with unsupported entry modifiers".to_string());
        }
        if owner != "PlayerRel::You" && (tapped || attacking || haste) {
            return self
                .deny("a modified token created under another player's control".to_string());
        }
        let Some(stem) = p.take("TokenScript") else {
            return self.deny("a `Token` effect naming no `TokenScript$`".to_string());
        };
        let raw_amount = p.take("TokenAmount");
        if owner == "PlayerRel::You" && !tapped && !attacking && !haste {
            return self.create_tokens(&stem, raw_amount.as_deref());
        }
        let Some(tokens) = self.tokens else {
            return self.deny("`Token` with no token scripts to read it against".to_string());
        };
        let Some(body) = tokens.body(&stem, self.cats) else {
            return self.deny(format!("token script `{stem}`"));
        };
        let token = format!("&generated_tokens::{}", body.constant);
        let amount = match raw_amount.as_deref() {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => match self.amount_or_count_ext(raw) {
                None => {
                    return match self.svars.get(raw) {
                        Some(how) => self.deny(format!("token amount `{raw}` = `{how}`")),
                        None => self.deny(format!("token amount `{raw}`")),
                    };
                }
                Some(a) => a,
            },
        };
        if owner != "PlayerRel::You" {
            return Some(vec![format!(
                "Effect::CreateTokensFor {{ player: {owner}, token: {token}, amount: {amount} }}"
            )]);
        }
        if attacking {
            return Some(vec![format!(
                "Effect::CreateTokenAttacking {{ token: {token}, amount: {amount}, \
                 sacrifice_at_next_end_step: {sacrifice_at_eot} }}"
            )]);
        }
        Some(vec![format!(
            "Effect::CreateTokenModified {{ token: {token}, amount: {amount}, tapped: {tapped}, \
             haste: {haste} }}"
        )])
    }

    /// The half of a token effect that is about the token rather than about
    /// the line that asked for it: which script, and how many.
    ///
    /// Its own function because [`Self::investigate_effect`] is the same
    /// question asked in different words, and a second copy of it is a
    /// second chance to answer "how many" differently — which is the whole
    /// argument `effects::applies_to` already makes about a predicate with
    /// three readers. The keys are read by the caller, because the corpus
    /// spells them differently (`TokenAmount$` against `Num$`) and that is
    /// the only difference between the two.
    pub(super) fn create_tokens(
        &mut self,
        stem: &str,
        raw_amount: Option<&str>,
    ) -> Option<Vec<String>> {
        let Some(tokens) = self.tokens else {
            return self.deny("`Token` with no token scripts to read it against".to_string());
        };
        let Some(body) = tokens.body(stem, self.cats) else {
            return self.deny(format!("token script `{stem}`"));
        };
        // `generated_tokens` and not `tokens`: the ledger is the one door,
        // and which half of it a constant is written in is the generator's
        // business — a hand-written token is re-exported from there under
        // the same name.
        let token = format!("&generated_tokens::{}", body.constant);
        let amount = match raw_amount {
            None => None,
            Some(raw) => match self.amount_or_count_ext(raw) {
                // A refusal names the `SVar` the amount resolves *through*
                // and not merely the letter, because `X` is what 356 scripts
                // write and each of them means it by a different count —
                // the letter alone ranks one entry that is really thirty.
                None => {
                    return match self.svars.get(raw) {
                        Some(how) => self.deny(format!("token amount `{raw}` = `{how}`")),
                        None => self.deny(format!("token amount `{raw}`")),
                    };
                }
                Some(a) => Some(a),
            },
        };
        Some(match amount.as_deref() {
            // One is the number `CreateToken` already means, and writing it
            // as `CreateTokenN { amount: Amount::Fixed(1) }` would give the
            // commonest token effect there is a second spelling.
            None | Some("Amount::Fixed(1)") => {
                vec![format!("Effect::CreateToken {{ token: {token} }}")]
            }
            Some(n) => vec![format!(
                "Effect::CreateTokenN {{ token: {token}, amount: {n} }}"
            )],
        })
    }

    /// CR 701.16a: "'Investigate' means 'Create a Clue token.'"
    ///
    /// One sentence in the rules, and nothing in the DSL was missing —
    /// `Effect::CreateToken` has been able to say it since tokens existed,
    /// and `tokens::CLUE` is already in the ledger. What was missing is the
    /// *word*: the corpus writes every other token as `Token` with a
    /// `TokenScript$` and writes this one as its own API, because Magic
    /// gives the action a keyword name. That is the fifth time an entry on
    /// the report turned out to be a sentence the DSL could already spell.
    ///
    /// Who investigates is read from **two** keys, because the corpus writes
    /// both: `Defined$ You` on five lines and `ValidPlayer$ You` on two.
    /// Every other value there names somebody else — `Opponent`,
    /// `TargetedController`, `Player.withMostTypeCreature` — and is refused
    /// for the reason [`Self::token_effect`] refuses a token under another
    /// player's control: `Effect::CreateToken` has no room for an owner.
    ///
    /// `Optional$ True` is on two lines and is claimed by nothing here, so
    /// those two refuse themselves. That is the honest-stub rule paying for
    /// itself rather than a case being handled: "you may investigate" is a
    /// question this DSL cannot ask, and reading the key as its absence
    /// would be an inference wearing a reading's clothes.
    pub(super) fn investigate_effect(
        &mut self,
        p: &mut Params,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let who = match (p.take("Defined"), p.take("ValidPlayer")) {
            // Both keys on one line would be two answers to one question.
            // No line in the corpus writes both, which is what makes this a
            // refusal rather than a precedence rule nobody can check.
            (Some(_), Some(_)) => {
                return self.deny("`Investigate` naming its player twice".to_string());
            }
            (Some(d), None) | (None, Some(d)) => self.player_rel(Some(&d)),
            (None, None) => self.player_rel_of(None, targets_a_player),
        };
        if who != Some("PlayerRel::You") {
            return self.deny("somebody other than you investigating".to_string());
        }
        self.create_tokens(CLUE_TOKEN_SCRIPT, p.take("Num").as_deref())
    }

    /// The revealed-name cast-history repeat, including its following shuffle
    /// and the memory cleanup. Each qualifying opponent loses life only once.
    fn revealed_name_life_loss(&mut self, mut p: Params, sub: Option<&str>) -> Option<Vec<String>> {
        if p.take("RepeatPlayers").as_deref() != Some("Opponent") {
            return None;
        }
        let repeat_sub = p.take("RepeatSubAbility")?;
        let shuffle = sub?;
        if !p.exhausted() {
            return None;
        }
        let (api, mut loss) = Params::parse(self.svars.get(repeat_sub.trim())?)?;
        loss.drop_prose();
        if api != "LoseLife"
            || loss.take("Defined").as_deref() != Some("Player.IsRemembered")
            || loss.take("LifeAmount").as_deref() != Some("6")
            || loss.take("ConditionCheckSVar").as_deref() != Some("LoseLifeCheck")
            || loss.take("ConditionSVarCompare").as_deref() != Some("GE1")
            || !loss.exhausted()
        {
            return None;
        }
        let (shuffle_api, mut sh) = Params::parse(self.svars.get(shuffle.trim())?)?;
        sh.drop_prose();
        let cleanup = sh.take("SubAbility");
        if shuffle_api != "Shuffle" || sh.take("Defined").as_deref() != Some("You") {
            return None;
        }
        if let Some(name) = cleanup {
            let (api, mut c) = Params::parse(self.svars.get(name.trim())?)?;
            if api != "Cleanup"
                || c.take("ClearImprinted").as_deref() != Some("True")
                || !c.exhausted()
            {
                return None;
            }
        }
        if !sh.exhausted() {
            return None;
        }
        Some(vec![
            "Effect::PlayersWhoCastRevealedNameThisTurnLoseLife { who: PlayerRel::EachOpponent, amount: Amount::Fixed(6) }"
                .to_string(),
            "Effect::ShuffleLibrary { who: PlayerRel::You }".to_string(),
        ])
    }

    /// `RepeatEach` over opponents where each opponent sacrifices a matching
    /// permanent unless they pay a generic mana cost (Rishadan Cutpurse,
    /// Rishadan Footpad, Rishadan Brigand).
    pub(super) fn repeat_each_effect(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &super::Chain,
    ) -> Option<Vec<String>> {
        if self.svars.get("LoseLifeCheck").map(String::as_str)
            == Some("Count$ThisTurnCast_Card.RememberedPlayerCtrl+sharesNameWith Imprinted")
        {
            return self.revealed_name_life_loss(p, sub);
        }
        if sub.is_some() || chain.target.is_some() || chain.second_target.is_some() {
            return None;
        }
        let players = match p.take("RepeatPlayers").as_deref() {
            Some("Player") => "PlayerRel::EachPlayer",
            Some("Player.Opponent") => "PlayerRel::EachOpponent",
            _ => return None,
        };
        let repeat_sub = p.take("RepeatSubAbility")?;
        if !p.exhausted() {
            return None;
        }
        let body = self.svars.get(repeat_sub.trim())?.clone();
        let (sub_api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if sub_api == "LoseLife"
            && matches!(
                sp.take("Defined").as_deref(),
                Some("Player.IsRemembered" | "Remembered")
            )
        {
            let amount_var = sp.take("LifeAmount")?;
            if let Some(sub_name) = sp.take("SubAbility")
                && self.svars.get(sub_name.trim()).map(String::as_str)
                    != Some("DB$ Cleanup | ClearRemembered$ True")
            {
                return None;
            }
            if !sp.exhausted() {
                return None;
            }
            let count = self.svars.get(amount_var.trim())?.trim();
            let valid = count
                .strip_prefix("Count$Valid ")?
                .strip_suffix(".RememberedPlayerCtrl")?;
            let expr = self.filter_expr(valid)?;
            let filter = self.body.filter_static("COUNT", &expr);
            return Some(vec![format!(
                "Effect::LoseLife {{ amount: Amount::ControlledCount {{ who: PlayerRel::Recipient, \
                 filter: &{filter} }}, target: {players} }}"
            )]);
        }
        if players != "PlayerRel::EachOpponent"
            || sub_api != "Sacrifice"
            || sp.take("Defined").as_deref() != Some("Remembered")
            || sp.take("UnlessPayer").as_deref() != Some("Remembered")
        {
            return None;
        }
        let sac_valid = sp.take("SacValid")?;
        let cost = sp.take("UnlessCost")?;
        if !sp.exhausted() {
            return None;
        }
        let filter = if sac_valid == "Permanent" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr(&sac_valid)?;
            format!("&{}", self.body.filter_static("SAC", &expr))
        };
        let super::Price::Generic(mana) = self.unless_price(cost.trim())? else {
            return None;
        };
        Some(vec![format!(
            "Effect::PlayerMayPayOr {{ player: PlayerRel::EachOpponent, mana: {mana}, \
             effect: &Effect::SacrificeFilter {{ who: PlayerRel::EventPlayer, filter: {filter} }} }}"
        )])
    }

    /// One side of a pump, refused by what its value resolves *through*.
    ///
    /// The same rule as the token amount's: the letter is what 356 scripts
    /// write and each means it by a different count, so an entry naming `X`
    /// would rank thirty questions as one.
    pub(super) fn pump_side(&mut self, raw: &str) -> Option<String> {
        if let Some(kicked) = self.kicked_branch {
            let trimmed = raw.trim();
            let neg = trimmed.starts_with('-');
            let var = trimmed.trim_start_matches(['+', '-']);
            if let Some((a, b)) = self.kicked_pair(var) {
                let value = if kicked { a } else { b };
                self.kicked_branch_used.set(true);
                return Some(if neg && value > 0 {
                    format!("Amount::NegXFixed({value})")
                } else {
                    format!("Amount::Fixed({value})")
                });
            }
        }
        if let Some(a) = pump_amount(raw, self.svars, self.has_x) {
            return Some(a);
        }
        let trimmed = raw.trim();
        let Some(how) = self
            .svars
            .get(trimmed.trim_start_matches(['+', '-']))
            .cloned()
        else {
            return self.deny(format!("pump amount `{raw}`"));
        };
        // A pump that **counts**. "Add {B} for each Swamp you control" and
        // "gets -1/-1 for each artifact you control" are one reading with a
        // sign in front of it, and [`Self::counted_amount`] has been able to
        // say the first since `Amount::CountOf`. Nothing read the second,
        // because the letter is not the number and the sign had nowhere to
        // live: `Amount::Fixed` holds a `u32`, and the two negatives the DSL
        // had are each a variant of their own magnitude.
        //
        // `Amount::Negated` is that sign as a wrapper rather than a negative
        // twin of every count there is, so the magnitude is said once. The
        // positive side comes with it and is the larger half by a long way —
        // 148 reference scripts against 26 — because a pump counting upwards
        // is what most of them print.
        match self.count_expr(&how) {
            Some(inner) if trimmed.starts_with('-') => Some(format!("Amount::Negated(&{inner})")),
            Some(inner) => Some(inner),
            None => self.deny(format!("pump amount `{raw}` = `{how}`")),
        }
    }

    fn pump_keyword_bits(kw: &str) -> Option<String> {
        let mut bits = Vec::new();
        for part in kw.split('&').map(str::trim) {
            let bit = match part {
                "HIDDEN CARDNAME can't attack or block." => {
                    "KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)"
                }
                "HIDDEN CARDNAME can't block." => "KeywordSet::CANT_BLOCK",
                "HIDDEN CARDNAME can't attack." => "KeywordSet::CANT_ATTACK",
                "HIDDEN CARDNAME can't be blocked." => "KeywordSet::UNBLOCKABLE",
                "Provoke" => "KeywordSet::PROVOKE",
                other => keyword_const(other)?,
            };
            bits.push(bit);
        }
        if bits.is_empty() {
            return None;
        }
        let mut out = bits[0].to_string();
        for b in &bits[1..] {
            out = format!("{out}.union({b})");
        }
        Some(out)
    }

    /// `Pump`: `NumAtt$ +2 | NumDef$ +2 | KW$ Trample`, the commonest
    /// effect in the whole script corpus.
    ///
    /// `Defined$ Self` and `Defined$ Targeted` are two different effects
    /// here, not one with a flag: `PumpFilter` binds `Filter::This` to the
    /// source, `PumpTarget` to what the spell targeted, and an ability can
    /// have both a target and a pump on itself.
    #[allow(clippy::too_many_lines)]
    pub(super) fn pump_effect(
        &mut self,
        p: &mut Params,
        target: &str,
        second: bool,
    ) -> Option<Vec<String>> {
        if p.peek("KW").map(str::trim) == Some("HIDDEN CARDNAME's power and toughness are switched")
            && !p.has("NumAtt")
            && !p.has("NumDef")
            && !second
        {
            p.take("KW");
            p.take("IsCurse");
            let duration = match p.take("Duration").as_deref() {
                None => "Duration::UntilEndOfTurn",
                Some(_) => return None,
            };
            return match p.take("Defined").as_deref() {
                Some("Self") => Some(vec![format!(
                    "Effect::continuous(&Filter::This, Modifier::SwitchPT, {duration})"
                )]),
                None if target == "TargetSpec::AnyPlayer" && !self.on_a_spell => {
                    Some(vec![format!(
                        "Effect::continuous(&Filter::This, Modifier::SwitchPT, {duration})"
                    )])
                }
                None | Some("Targeted") if target != "TargetSpec::AnyPlayer" => {
                    Some(vec![format!(
                        "Effect::ModifyTargets {{ modifier: Modifier::SwitchPT, duration: {duration} }}"
                    )])
                }
                _ => None,
            };
        }
        if p.peek("KW").map(str::trim)
            == Some("HIDDEN This card doesn't untap during your next untap step.")
            && !p.has("NumAtt")
            && !p.has("NumDef")
            && !second
        {
            if p.take("Duration").as_deref() != Some("Permanent") {
                return None;
            }
            p.take("KW");
            p.take("IsCurse");
            let to = match p.take("Defined").as_deref() {
                Some("Self") => "TargetSpec::ThisObject",
                None if target == "TargetSpec::AnyPlayer" && !self.on_a_spell => {
                    "TargetSpec::ThisObject"
                }
                None | Some("Targeted" | "ParentTarget") if target != "TargetSpec::AnyPlayer" => {
                    target
                }
                Some("TriggeredCardLKICopy" | "TriggeredCard")
                    if self.trigger_mode.as_deref() == Some("ChangesZone") =>
                {
                    "TargetSpec::EventObject"
                }
                Some("TriggeredAttackerLKICopy" | "TriggeredAttacker")
                    if self.trigger_mode.as_deref() == Some("Attacks") =>
                {
                    "TargetSpec::EventObject"
                }
                Some("TriggeredBlockerLKICopy" | "TriggeredBlocker")
                    if self.trigger_mode.as_deref() == Some("Blocks") =>
                {
                    "TargetSpec::EventObject"
                }
                Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
                    if self.in_delayed && target == "TargetSpec::AnyPlayer" =>
                {
                    "TargetSpec::EventObject"
                }
                _ => return None,
            };
            return Some(vec![format!(
                "Effect::SuppressUntap {{ target: {to}, steps: 1 }}"
            )]);
        }
        if p.peek("KW").map(str::trim) == Some("HIDDEN All creatures able to block CARDNAME do so.")
            && !p.has("NumAtt")
            && !p.has("NumDef")
            && !second
        {
            if p.take("Duration").is_some() {
                return None;
            }
            p.take("KW");
            p.take("IsCurse");
            return match p.take("Defined").as_deref() {
                Some("Self") => Some(vec![
                    "Effect::continuous(&Filter::This, Modifier::MustBeBlockedByAllAble, Duration::UntilEndOfTurn)"
                        .to_string(),
                ]),
                None if target == "TargetSpec::AnyPlayer" && !self.on_a_spell => Some(vec![
                    "Effect::continuous(&Filter::This, Modifier::MustBeBlockedByAllAble, Duration::UntilEndOfTurn)"
                        .to_string(),
                ]),
                None | Some("Targeted" | "ParentTarget") if target != "TargetSpec::AnyPlayer" => {
                    Some(vec![
                        "Effect::ModifyTargets { modifier: Modifier::MustBeBlockedByAllAble, duration: Duration::UntilEndOfTurn }"
                            .to_string(),
                    ])
                }
                _ => None,
            };
        }
        if !second
            && target == "TargetSpec::AnyPlayer"
            && p.peek("Defined") == Some("You")
            && p.peek("KW").map(str::trim) == Some("Hexproof")
            && !p.has("NumAtt")
            && !p.has("NumDef")
        {
            let duration = match p.take("Duration").as_deref() {
                None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
                Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
                Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
                Some(_) => return None,
            };
            p.take("Defined");
            p.take("KW");
            p.take("IsCurse");
            return Some(vec![format!(
                "Effect::continuous(&Filter::Any, Modifier::PlayerHexproof {{ who: PlayerRel::You }}, {duration})"
            )]);
        }
        let power = self.pump_side_ext(p.take("NumAtt").as_deref().unwrap_or("0"))?;
        let toughness = self.pump_side_ext(p.take("NumDef").as_deref().unwrap_or("0"))?;
        let keywords = match p.take("KW") {
            None => "KeywordSet::EMPTY".to_string(),
            Some(kw) => Self::pump_keyword_bits(&kw)?,
        };
        // Every duration but the default is a lifetime the DSL spells
        // differently; none of them is "until end of turn" with a longer
        // name.
        let duration = match p.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some("UntilUntaps") if !self.on_a_spell => {
                "Duration::WhileSourceTapped { control: false, power_limited: false }"
            }
            Some("AsLongAsControl") if !self.on_a_spell => "Duration::WhileYouControlSource",
            Some(_) => return None,
        };
        // Purely an AI targeting hint (don't curse your own team); it moves
        // no rule, so reading it changes nothing.
        p.take("IsCurse");
        // A pump that names nobody and targets nothing is the source's own:
        // the reference defaults an absent `Defined$` to the card itself,
        // and 175 of its `AB$ Pump` lines are written that way — Shivan
        // Dragon's "{R}: This creature gets +1/+0". A pump that moves
        // nothing at all is a placeholder some other line does the work
        // for (`SP$ Pump | StackDescription$ None`), and pumping the source
        // by nought would claim a card that does nothing. A spell has no
        // self to pump once it resolves, so its bare pump stays refused.
        let empty = power == "Amount::Fixed(0)"
            && toughness == "Amount::Fixed(0)"
            && keywords == "KeywordSet::EMPTY";
        let defined = match p.take("Defined") {
            None if target == "TargetSpec::AnyPlayer" && !self.on_a_spell && !empty => {
                Some("Self".to_string())
            }
            other => other,
        };
        if !second
            && ((matches!(
                defined.as_deref(),
                Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
            ) && self.in_delayed
                && target == "TargetSpec::AnyPlayer")
                || (matches!(
                    defined.as_deref(),
                    Some(
                        "TriggeredCardLKICopy"
                            | "TriggeredCard"
                            | "TriggeredNewCardLKICopy"
                            | "TriggeredNewCard"
                    )
                ) && self.trigger_mode.as_deref() == Some("ChangesZone"))
                || (matches!(
                    defined.as_deref(),
                    Some("TriggeredAttackerLKICopy" | "TriggeredAttacker")
                ) && (matches!(
                    self.trigger_mode.as_deref(),
                    Some("AttackerUnblocked" | "AttackerBlocked")
                ) || (self.trigger_mode.as_deref()
                    == Some("AttackerBlockedByCreature")
                    && self
                        .block_line
                        .as_ref()
                        .is_some_and(|(role, _, _)| *role == super::BlockRole::Blocks))))
                || (matches!(
                    defined.as_deref(),
                    Some("TriggeredBlockerLKICopy" | "TriggeredBlocker")
                ) && (self.trigger_mode.as_deref() == Some("Blocks")
                    || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                        && self
                            .block_line
                            .as_ref()
                            .is_some_and(|(role, _, _)| *role == super::BlockRole::Blocked)))))
        {
            return Some(vec![format!(
                "Effect::PumpEventObject {{ power: {power}, toughness: {toughness}, \
                 keywords: {keywords}, duration: {duration} }}"
            )]);
        }
        // "Enchanted creature gets +1/+0 until end of turn" on an Aura's own
        // activated ability (Firebreathing): the object the source is
        // attached to, which `Filter::AttachedToBySource` binds to the
        // resolving ability's source.
        let whom = match defined.as_deref() {
            Some("Self") => Some("Filter::This"),
            Some("Enchanted" | "Equipped") => Some("Filter::AttachedToBySource"),
            Some("TriggeredAttackerLKICopy" | "TriggeredAttacker")
                if self.trigger_mode.as_deref() == Some("Attacks") =>
            {
                Some("Filter::Reference")
            }
            _ => None,
        };
        if let Some(whom) = whom {
            return Some(vec![format!(
                "Effect::PumpFilter {{ filter: &{whom}, controlled_by: None, \
                 power: {power}, toughness: {toughness}, keywords: {keywords}, \
                 duration: {duration} }}"
            )]);
        }
        Some(match defined.as_deref() {
            None | Some("Targeted") => {
                // Without a target this would pump nothing at all.
                if target == "TargetSpec::AnyPlayer" {
                    return None;
                }
                let kind = if second {
                    "PumpTargetSlot { slot: TargetSlot::Second,"
                } else {
                    "PumpTarget {"
                };
                vec![format!(
                    "Effect::{kind} power: {power}, toughness: {toughness}, \
                     keywords: {keywords}, duration: {duration} }}"
                )]
            }
            Some("ParentTarget") if !second && target != "TargetSpec::AnyPlayer" => {
                vec![format!(
                    "Effect::PumpTarget {{ power: {power}, toughness: {toughness}, \
                     keywords: {keywords}, duration: {duration} }}"
                )]
            }
            Some(_) => return None,
        })
    }

    pub(super) fn pump_all_side(&mut self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        let var = trimmed.trim_start_matches(['+', '-']);
        if self.svars.get(var).map(|d| d.trim()) == Some("Count$Domain") {
            let inner = "Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)";
            return Some(if trimmed.starts_with('-') {
                format!("Amount::Negated(&{inner})")
            } else {
                inner.to_string()
            });
        }
        self.pump_side(raw)
    }

    /// `PumpAll`: `ValidCards$ Creature.blocking | NumAtt$ +1 | NumDef$ +1`,
    /// pumping every permanent on the battlefield that matches `ValidCards$`.
    pub(super) fn pump_all_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("KW").map(str::trim)
            == Some("HIDDEN This card doesn't untap during your next untap step.")
            && p.peek("Duration") == Some("Permanent")
            && !p.has("NumAtt")
            && !p.has("NumDef")
            && target.is_none()
            && p.peek("Defined").is_none()
        {
            p.take("KW");
            p.take("Duration");
            p.take("IsCurse");
            let valid = p.take("ValidCards")?;
            let expr = self.filter_expr(&valid)?;
            let name = self.body.filter_static("FREEZE_ALL", &expr);
            return Some(vec![format!(
                "Effect::SuppressUntap {{ target: TargetSpec::Object(&{name}), steps: 1 }}"
            )]);
        }
        let defined = p.take("Defined");
        let controlled_by = if target == Some("TargetSpec::Player(PlayerRel::Chosen)")
            && ((targets_a_player && matches!(defined.as_deref(), None | Some("Targeted")))
                || (!targets_here && defined.as_deref() == Some("Targeted")))
        {
            "Some(PlayerRel::Chosen)"
        } else if target.is_none() && defined.is_none() {
            "None"
        } else {
            return None;
        };
        let valid = p.take("ValidCards")?;
        if controlled_by != "None" && (valid.contains("Ctrl") || valid.contains("Own")) {
            return None;
        }
        let power = self.pump_all_side(p.take("NumAtt").as_deref().unwrap_or("0"))?;
        let toughness = self.pump_all_side(p.take("NumDef").as_deref().unwrap_or("0"))?;
        let keywords = match p.take("KW") {
            None => "KeywordSet::EMPTY".to_string(),
            Some(kw) => Self::pump_keyword_bits(&kw)?,
        };
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some(_) => return None,
        };
        p.take("IsCurse");
        if power == "Amount::Fixed(0)"
            && toughness == "Amount::Fixed(0)"
            && keywords == "KeywordSet::EMPTY"
        {
            return None;
        }
        let mut expr = self.filter_expr(&valid)?;
        if !valid.contains(['.', ',', '+'])
            && expr.starts_with("Filter::HasSubtype(subtypes::creature::")
            && self
                .oracle
                .to_ascii_lowercase()
                .contains(&format!("{} creatures", valid.trim().to_ascii_lowercase()))
        {
            expr = format!("Filter::And(&[Filter::CREATURE, {expr}])");
        }
        let filter = self.body.filter_static("PUMP_ALL", &expr);
        Some(vec![format!(
            "Effect::PumpFilter {{ filter: &{filter}, controlled_by: {controlled_by}, \
             power: {power}, toughness: {toughness}, keywords: {keywords}, \
             duration: {duration} }}"
        )])
    }

    /// `ChangeZoneAll` into a library with a shuffle, from hands and
    /// graveyards: Timetwister's "each player shuffles their hand and
    /// graveyard into their library", and "target player shuffles their
    /// graveyard into their library".
    ///
    /// Whose cards: `ChangeType$ Card` with no player named is every
    /// player's, each into their own library; `Card.YouOwn` is yours; a
    /// `Defined$` or a player target names them. Every other change —
    /// battlefield, exile, a library position, a random pick, a type other
    /// than any card — is refused.
    #[allow(clippy::too_many_lines)]
    pub(super) fn shuffle_into_library(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.peek("Origin") == Some("Library") && p.peek("Destination") == Some("Exile") {
            if p.take("ChangeType").as_deref() != Some("Card.YouOwn") {
                return None;
            }
            p.take("Origin");
            p.take("Destination");
            return Some(vec![
                "Effect::ExileTop { who: PlayerRel::You, count: Amount::LibrarySize(PlayerRel::You) }"
                    .to_string(),
            ]);
        }
        if p.peek("Destination") != Some("Library") {
            if p.peek("Origin") == Some("Graveyard") && p.peek("Destination") == Some("Exile") {
                p.take("Origin");
                p.take("Destination");
                p.take("IsCurse");
                let defined = p.take("Defined");
                let change_type = p.take("ChangeType");
                let player = match change_type.as_deref() {
                    Some("Card.YouOwn") if defined.is_none() && target.is_none() => {
                        "PlayerRel::You"
                    }
                    Some("Card.OppOwn" | "Card.OppCtrl" | "Card.YouDontOwn")
                        if defined.is_none() && target.is_none() =>
                    {
                        "PlayerRel::EachOpponent"
                    }
                    Some("Card") | None if defined.is_some() || targets_a_player => {
                        let def = match defined.as_deref() {
                            Some("ParentTarget")
                                if target == Some("TargetSpec::Player(PlayerRel::Chosen)") =>
                            {
                                Some("Targeted")
                            }
                            other => other,
                        };
                        self.player_of_line(def, target, targets_a_player)?
                    }
                    Some("Card") if target.is_none() => "PlayerRel::EachPlayer",
                    _ => return None,
                };
                return Some(vec![format!(
                    "Effect::ExileGraveyard {{ player: {player} }}"
                )]);
            }
            if target.is_some() || targets_a_player || p.take("Defined").is_some() {
                return None;
            }
            return match (
                p.take("Origin").as_deref(),
                p.take("Destination").as_deref(),
            ) {
                (Some("Exile"), Some("Battlefield"))
                    if p.take("ChangeType").as_deref() == Some("Card.ExiledWithSource") =>
                {
                    Some(vec!["Effect::ReturnLinkedToBattlefield".to_string()])
                }
                (Some("Exile"), Some(dest @ ("Hand" | "Graveyard")))
                    if p.take("ChangeType").as_deref() == Some("Card.ExiledWithSource") =>
                {
                    if let Some(hidden) = p.take("Hidden")
                        && hidden != "True"
                    {
                        return None;
                    }
                    Some(vec![format!(
                        "Effect::ReturnLinkedExile {{ to: ZoneRef::{dest} }}"
                    )])
                }
                (Some("Battlefield"), Some("Hand")) => {
                    p.take("IsCurse");
                    let valid = p.take("ChangeType")?;
                    let expr = self.filter_expr(&valid)?;
                    let name = self.body.filter_static("BOUNCE_ALL", &expr);
                    Some(vec![format!(
                        "Effect::ReturnAllToHand {{ filter: &{name}, opponents_only: false }}"
                    )])
                }
                (Some("Battlefield"), Some("Exile")) => {
                    p.take("IsCurse");
                    let valid = p.take("ChangeType")?;
                    let expr = self.filter_expr(&valid)?;
                    let name = self.body.filter_static("EXILE_ALL", &expr);
                    let linked = self.svars.values().any(|s| {
                        s.contains("Defined$ ExiledWith") || s.contains("Card.ExiledWithSource")
                    });
                    Some(vec![format!(
                        "Effect::ExileAll {{ filter: &{name}, linked: {linked} }}"
                    )])
                }
                (Some("Graveyard"), Some("Hand")) => {
                    p.take("IsCurse");
                    let valid = p.take("ChangeType")?;
                    let stripped = Self::strip_your_graveyard_owner(&valid)?;
                    let expr = self.filter_expr_in_zone(&stripped, true)?;
                    let name = self.body.filter_static("GRAVE_TO_HAND", &expr);
                    Some(vec![format!(
                        "Effect::GraveyardAllToHand {{ filter: &{name} }}"
                    )])
                }
                (Some("Graveyard"), Some("Battlefield")) => {
                    p.take("IsCurse");
                    let valid = p.take("ChangeType")?;
                    let tapped = match p.take("Tapped").as_deref() {
                        None | Some("False") => false,
                        Some("True") => true,
                        Some(_) => return None,
                    };
                    let gain = match p.take("GainControl").as_deref() {
                        None | Some("False") => false,
                        Some("True") => true,
                        Some(_) => return None,
                    };
                    if !gain && let Some(stripped) = Self::strip_your_graveyard_owner(&valid) {
                        let expr = self.filter_expr_in_zone(&stripped, true)?;
                        let name = self.body.filter_static("REANIMATE_ALL", &expr);
                        return Some(vec![format!(
                            "Effect::YourGraveyardToBattlefield {{ filter: &{name}, tapped: {tapped} }}"
                        )]);
                    }
                    if valid == "Creature" && gain && !tapped {
                        return Some(vec![
                            "Effect::AllGraveyardCreaturesToBattlefield".to_string(),
                        ]);
                    }
                    if valid.contains("Ctrl") || valid.contains("Own") || tapped || gain {
                        return None;
                    }
                    let expr = self.filter_expr_in_zone(&valid, true)?;
                    let name = self.body.filter_static("REANIMATE_ALL", &expr);
                    Some(vec![format!(
                        "Effect::EachPlayerGraveyardToBattlefield {{ filter: &{name} }}"
                    )])
                }
                _ => None,
            };
        }
        if p.peek("Origin") == Some("Hand")
            && p.peek("Destination") == Some("Library")
            && p.peek("LibraryPosition") == Some("0")
            && !p.has("Shuffle")
        {
            p.take("Origin");
            p.take("Destination");
            p.take("LibraryPosition");
            let defined = p.take("Defined");
            let change_type = p.take("ChangeType");
            let player = match change_type.as_deref() {
                Some("Card.YouOwn") if defined.is_none() && target.is_none() => "PlayerRel::You",
                Some("Card") | None if defined.is_some() || targets_a_player => {
                    self.player_of_line(defined.as_deref(), target, targets_a_player)?
                }
                _ => return None,
            };
            return Some(vec![format!("Effect::PutHandOnTop {{ player: {player} }}")]);
        }
        if p.peek("Origin") == Some("Graveyard")
            && p.peek("Destination") == Some("Library")
            && p.peek("LibraryPosition") == Some("-1")
            && p.peek("RandomOrder") == Some("True")
            && !p.has("Shuffle")
        {
            p.take("Origin");
            p.take("Destination");
            p.take("LibraryPosition");
            p.take("RandomOrder");
            let defined = p.take("Defined");
            let change_type = p.take("ChangeType");
            let player = match change_type.as_deref() {
                Some("Card.YouOwn") if defined.is_none() && target.is_none() => "PlayerRel::You",
                Some("Card") | None if defined.is_some() || targets_a_player => {
                    self.player_of_line(defined.as_deref(), target, targets_a_player)?
                }
                Some("Card") if target.is_none() => "PlayerRel::EachPlayer",
                _ => return None,
            };
            return Some(vec![format!(
                "Effect::GraveyardToLibraryBottom {{ player: {player} }}"
            )]);
        }
        if p.take("Destination").as_deref() != Some("Library")
            || p.take("Shuffle").as_deref() != Some("True")
        {
            return None;
        }
        let origin = p.take("Origin")?;
        let (hand, graveyard) = match origin.as_str() {
            "Hand" => (true, false),
            "Graveyard" => (false, true),
            "Hand,Graveyard" | "Graveyard,Hand" => (true, true),
            _ => return None,
        };
        // The reference's "from every zone listed" rather than one of them:
        // what two origins mean on a card.
        if hand && graveyard {
            p.take("UseAllOriginZones");
        }
        p.take("IsCurse");
        let defined = p.take("Defined");
        let change_type = p.take("ChangeType");
        if let Some(effects) = self.shuffle_matching_graveyard(
            !hand && graveyard && defined.is_none() && target.is_none(),
            change_type.as_deref(),
        ) {
            return Some(effects);
        }
        let who = match change_type.as_deref() {
            Some("Card.YouOwn" | "Card.YouCtrl") if defined.is_none() && target.is_none() => {
                "PlayerRel::You".to_string()
            }
            Some("Card") | None if defined.is_some() || targets_a_player => self
                .player_of_line(defined.as_deref(), target, targets_a_player)?
                .to_string(),
            Some("Card") if target.is_none() => "PlayerRel::EachPlayer".to_string(),
            _ => return None,
        };
        Some(vec![format!(
            "Effect::ShuffleIntoLibrary {{ who: {who}, hand: {hand}, graveyard: {graveyard} }}"
        )])
    }

    fn shuffle_matching_graveyard(
        &mut self,
        eligible: bool,
        change_type: Option<&str>,
    ) -> Option<Vec<String>> {
        if !eligible {
            return None;
        }
        let raw = change_type?;
        if matches!(raw, "Card.YouOwn" | "Card.YouCtrl") {
            return None;
        }
        let stripped = Self::strip_your_graveyard_owner(raw)?;
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let name = self.body.filter_static("SHUFFLE_GRAVEYARD", &expr);
        Some(vec![format!(
            "Effect::ShuffleGraveyardMatchingIntoLibrary {{ who: PlayerRel::You, filter: &{name} }}"
        )])
    }

    /// "Target creature can't be blocked this turn" (Dwarven Warriors,
    /// Rogue's Passage, Infiltrate): an `Effect` whose one static says
    /// nothing may block what it remembers, and which ends when that leaves
    /// the battlefield. The engine's word for it is the keyword, granted for
    /// the rest of the turn — to the targets, or to the source itself.
    /// Every other `Effect` is its own sentence and stays refused.
    #[allow(clippy::unused_self)]
    pub(super) fn debuff_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target == Some("TargetSpec::Object(&Filter::CREATURE)")
            && p.peek("AllSuffixKeywords") == Some("walk")
        {
            p.take("AllSuffixKeywords");
            return Some(vec![
                "Effect::ModifyTargets { modifier: Modifier::LoseAllLandwalk, \
                 duration: Duration::UntilEndOfTurn }"
                    .to_string(),
            ]);
        }
        if target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            && p.peek("Defined").is_none()
            && let Some(kw) = p.take("Keywords")
        {
            p.take("IsCurse");
            let mut out = Vec::new();
            for part in kw.split('&') {
                let bit = keyword_const(part.trim())?;
                out.push(format!(
                    "Effect::ModifyTargets {{ modifier: Modifier::RemoveKeyword({bit}), \
                     duration: Duration::UntilEndOfTurn }}"
                ));
            }
            return (!out.is_empty()).then_some(out);
        }
        if target.is_none() && p.take("Defined").as_deref() == Some("Self") {
            let kw = p.take("Keywords")?;
            let mut out = Vec::new();
            for part in kw.split('&') {
                let bit = keyword_const(part.trim())?;
                out.push(format!(
                    "Effect::continuous(&Filter::This, Modifier::RemoveKeyword({bit}), \
                     Duration::UntilEndOfTurn)"
                ));
            }
            return (!out.is_empty()).then_some(out);
        }
        None
    }

    pub(super) fn unblockable_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let statics = p.take("StaticAbilities")?;
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CantBlockBy"
            || st.take("ValidAttacker").as_deref() != Some("Card.IsRemembered")
            || !st.exhausted()
        {
            return None;
        }
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if !matches!(ends.as_deref(), None | Some("Battlefield")) || p.take("Duration").is_some() {
            return None;
        }
        p.take("IsCurse");
        let keywords = "KeywordSet::UNBLOCKABLE";
        Some(match p.take("RememberObjects").as_deref() {
            Some("Targeted") if target.is_some_and(|t| t != "TargetSpec::AnyPlayer") => {
                vec![format!(
                    "Effect::PumpTarget {{ power: Amount::Fixed(0), toughness: \
                     Amount::Fixed(0), keywords: {keywords}, duration: \
                     Duration::UntilEndOfTurn }}"
                )]
            }
            Some("Self") if !self.on_a_spell => vec![format!(
                "Effect::PumpFilter {{ filter: &Filter::This, controlled_by: None, \
                 power: Amount::Fixed(0), toughness: Amount::Fixed(0), \
                 keywords: {keywords}, duration: Duration::UntilEndOfTurn }}"
            )],
            _ => return None,
        })
    }

    /// Wave-3 extensions for `SP$/AB$/DB$ Effect` (called from `chains.rs`
    /// only after `static_effect` returns `None`).
    pub(super) fn wave3_effect_api(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.peek("ReplacementEffects").is_some() {
            return self.dont_untap_replacement_effect(p, target);
        }
        if p.peek("Triggers").is_some() || p.peek("Abilities").is_some() {
            return None;
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some("UntilYourNextUpkeep") => "Duration::UntilYourNextUpkeep",
            Some("Permanent") => "Duration::Indefinitely",
            Some("AsLongAsControl") if !self.on_a_spell => "Duration::WhileYouControlSource",
            Some("UntilUntaps") if !self.on_a_spell => {
                "Duration::WhileSourceTapped { control: false, power_limited: false }"
            }
            _ => return None,
        };
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if !matches!(ends.as_deref(), None | Some("Battlefield")) {
            return None;
        }
        p.take("IsCurse");
        let subject = self.effect_remember_subject(p, target)?;
        match (p.peek("Defined"), subject) {
            (Some("Targeted"), EffectSubject::Targets) | (Some("Self"), EffectSubject::Source) => {
                p.take("Defined");
            }
            (Some("Self"), EffectSubject::Unbound) if !self.on_a_spell => {
                p.take("Defined");
            }
            _ => {}
        }
        let statics = p.take("StaticAbilities")?;
        let mut out = Vec::new();
        for svar_name in statics.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let body = self.svars.get(svar_name)?.clone();
            let (mode, mut st) = Params::parse(&body)?;
            st.drop_prose();
            out.extend(self.wave3_static_effect_line(&mode, &mut st, subject, duration)?);
        }
        (!out.is_empty()).then_some(out)
    }

    fn wave3_static_effect_line(
        &mut self,
        mode: &str,
        st: &mut Params,
        subject: EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
        match mode {
            "CantBlockBy" => self.effect_cant_block_by(st, subject, duration),
            "MustAttack" => {
                let valid = st.take("ValidCreature")?;
                st.exhausted().then_some(())?;
                self.emit_effect_modifiers(
                    subject,
                    &valid,
                    &["Modifier::AttacksEachCombat".to_string()],
                    duration,
                )
            }
            "CantBlock"
            | "CantAttack"
            | "AssignNoCombatDamage"
            | "CantRegenerate"
            | "CantPhaseOut" => {
                let modifier = match mode {
                    "CantBlock" => "Modifier::AddKeyword(KeywordSet::CANT_BLOCK)",
                    "CantAttack" => "Modifier::CantAttack",
                    "AssignNoCombatDamage" => "Modifier::AssignsNoCombatDamage",
                    "CantPhaseOut" => "Modifier::CantPhaseOut",
                    _ => "Modifier::CantBeRegenerated",
                };
                let valid = st.take("ValidCard")?;
                st.exhausted().then_some(())?;
                self.emit_effect_modifiers(subject, &valid, &[modifier.to_string()], duration)
            }
            "CantBeCast" if subject == EffectSubject::Unbound => {
                let valid = st.take("ValidCard").unwrap_or_else(|| "Card".to_string());
                if valid.contains("IsRemembered")
                    || valid.contains("EffectSource")
                    || valid.contains("RememberedPlayer")
                {
                    return None;
                }
                let filter_ref = if valid == "Card" {
                    "&Filter::Any".to_string()
                } else {
                    let expr = self.filter_expr(&valid)?;
                    let name = self.body.filter_static("EFFECT", &expr);
                    format!("&{name}")
                };
                let modifier = match st.take("Caster").as_deref() {
                    Some("Opponent" | "Player.Opponent") => {
                        format!("Modifier::OpponentsCantCast({filter_ref})")
                    }
                    None | Some("Player") => format!(
                        "Modifier::PlayersCantCast {{ who: PlayerRel::EachPlayer, filter: {filter_ref} }}"
                    ),
                    Some("You") => format!(
                        "Modifier::PlayersCantCast {{ who: PlayerRel::You, filter: {filter_ref} }}"
                    ),
                    _ => return None,
                };
                st.exhausted().then_some(())?;
                Some(vec![format!(
                    "Effect::continuous(&Filter::Any, {modifier}, {duration})"
                )])
            }
            "CastWithFlash" if subject == EffectSubject::Unbound => {
                Self::wave3_cast_with_flash(st, duration)
            }
            "CantBeActivated" => {
                if st.take("ValidSA").as_deref() != Some("Activated.!ManaAbility") {
                    return None;
                }
                let valid = st.take("ValidCard")?;
                st.exhausted().then_some(())?;
                self.emit_effect_modifiers(
                    subject,
                    &valid,
                    &["Modifier::CantActivateNonMana".to_string()],
                    duration,
                )
            }
            "Continuous" => self.effect_continuous_static(st, subject, duration),
            _ => None,
        }
    }

    fn wave3_cast_with_flash(st: &mut Params, duration: &str) -> Option<Vec<String>> {
        if !matches!(st.take("ValidSA").as_deref(), None | Some("Spell"))
            || st.take("Caster").as_deref() != Some("You")
        {
            return None;
        }
        let valid = st.take("ValidCard")?;
        let modifier = match valid.as_str() {
            "Sorcery" => "Modifier::SorceriesHaveFlash".to_string(),
            "Card" | "Card.nonLand" => "Modifier::SpellsHaveFlash(None)".to_string(),
            other => {
                let mut types = Vec::new();
                for part in other.split(',').map(str::trim) {
                    if part == "Land" {
                        return None;
                    }
                    types.push(card_type_const(part)?.to_string());
                }
                let mask = types
                    .into_iter()
                    .reduce(|acc, t| format!("{acc}.union({t})"))?;
                format!("Modifier::SpellsHaveFlash(Some({mask}))")
            }
        };
        st.exhausted().then_some(())?;
        Some(vec![format!(
            "Effect::continuous(&Filter::Any, {modifier}, {duration})"
        )])
    }

    /// `DB$ Effect | ReplacementEffects$ DontUntap` lowering to
    /// `Modifier::DoesNotUntap` with exact untap-step or source-linked duration.
    fn dont_untap_replacement_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.peek("StaticAbilities").is_some() || p.peek("Triggers").is_some() {
            return None;
        }
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if !matches!(ends.as_deref(), None | Some("Battlefield")) {
            return None;
        }
        p.take("IsCurse");
        let dur_raw = p.take("Duration")?;
        let subject = self.effect_remember_subject(p, target)?;
        let name = p.take("ReplacementEffects")?;
        let body = self.svars.get(name.trim())?.clone();
        let (event, mut r) = Params::parse(&body)?;
        r.drop_prose();
        if event != "Untap" || r.take("Layer").as_deref() != Some("CantHappen") {
            return None;
        }
        let step_turn = r.take("ValidStepTurnToController")?;
        let valid = r.take("ValidCard")?;
        if !r.exhausted() {
            return None;
        }
        let duration = match (dur_raw.as_str(), step_turn.as_str()) {
            ("UntilTheEndOfYourNextUntap", "Player.Activator") => {
                "Duration::UntilYourNextUntapStep"
            }
            ("AsLongAsControl", "You") if !self.on_a_spell && subject == EffectSubject::Targets => {
                "Duration::WhileYouControlSource"
            }
            ("UntilUntaps", "You") if !self.on_a_spell && subject == EffectSubject::Targets => {
                "Duration::WhileSourceTapped { control: false, power_limited: false }"
            }
            _ => return None,
        };
        self.emit_effect_modifiers(
            subject,
            &valid,
            &["Modifier::DoesNotUntap".to_string()],
            duration,
        )
    }

    fn effect_remember_subject(
        &self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<EffectSubject> {
        let obj_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        Some(match p.take("RememberObjects").as_deref() {
            None => EffectSubject::Unbound,
            Some("Targeted" | "ParentTarget" | "ThisTargetedCard") if obj_target => {
                EffectSubject::Targets
            }
            Some("Self") if !self.on_a_spell => EffectSubject::Source,
            Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
                if self.in_delayed && target.is_none() =>
            {
                EffectSubject::Event
            }
            Some("TriggeredCard" | "TriggeredCardLKICopy")
                if self.trigger_mode.as_deref() == Some("ChangesZone") =>
            {
                EffectSubject::Event
            }
            Some("TriggeredAttacker" | "TriggeredAttackerLKICopy")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerUnblocked")
                ) =>
            {
                EffectSubject::Event
            }
            Some("TriggeredBlocker" | "TriggeredBlockerLKICopy")
                if self.trigger_mode.as_deref() == Some("Blocks") =>
            {
                EffectSubject::Event
            }
            Some("TriggeredTarget" | "TriggeredTargetLKICopy")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce")
                ) =>
            {
                EffectSubject::Event
            }
            Some(_) => return None,
        })
    }

    fn emit_effect_modifiers(
        &mut self,
        subject: EffectSubject,
        valid: &str,
        modifiers: &[String],
        duration: &str,
    ) -> Option<Vec<String>> {
        if modifiers.is_empty() {
            return None;
        }
        let target_kind = match (subject, valid) {
            (EffectSubject::Targets, "Card.IsRemembered" | "Creature.IsRemembered") => 0u8,
            (EffectSubject::Source, "Card.IsRemembered" | "Creature.IsRemembered") => 1u8,
            (EffectSubject::Event, "Card.IsRemembered" | "Creature.IsRemembered") => 2u8,
            (
                EffectSubject::Unbound,
                "Card.Self" | "Creature.Self" | "Card.EffectSource" | "Creature.EffectSource",
            ) if !self.on_a_spell => 1u8,
            (EffectSubject::Unbound, other)
                if !other.contains("IsRemembered")
                    && !other.contains("EffectSource")
                    && !other.contains("RememberedPlayer") =>
            {
                let expr = self.filter_expr(other)?;
                let name = self.body.filter_static("EFFECT", &expr);
                return Some(
                    modifiers
                        .iter()
                        .map(|m| format!("Effect::continuous(&{name}, {m}, {duration})"))
                        .collect(),
                );
            }
            _ => return None,
        };
        let ctor = match target_kind {
            0 => "ModifyTargets",
            1 => "ModifySource",
            _ => "ModifyEvent",
        };
        Some(
            modifiers
                .iter()
                .map(|m| format!("Effect::{ctor} {{ modifier: {m}, duration: {duration} }}"))
                .collect(),
        )
    }

    fn effect_cant_block_by(
        &mut self,
        st: &mut Params,
        subject: EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
        let attacker = st.take("ValidAttacker")?;
        let modifier = match st.take("ValidBlocker") {
            None => "Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)".to_string(),
            Some(blocker)
                if !blocker.contains("IsRemembered") && !blocker.contains("EffectSource") =>
            {
                let expr = self.filter_expr(&blocker)?;
                let name = self.body.filter_static("BLOCKER", &expr);
                format!("Modifier::CantBeBlockedBy(&{name})")
            }
            Some(_) => return None,
        };
        if !st.exhausted() {
            return None;
        }
        self.emit_effect_modifiers(subject, &attacker, &[modifier], duration)
    }

    fn effect_continuous_static(
        &mut self,
        st: &mut Params,
        subject: EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
        if !matches!(
            st.take("AffectedZone").as_deref(),
            None | Some("Battlefield")
        ) {
            return None;
        }
        let affected = st.take("Affected")?;
        if affected == "You" && subject == EffectSubject::Unbound {
            let modifier = if st.take("NoHandSize").as_deref() == Some("True") {
                "Modifier::NoMaxHandSize"
            } else if st.take("AddKeyword").as_deref() == Some("Hexproof") {
                "Modifier::PlayerHexproof { who: PlayerRel::You }"
            } else {
                return None;
            };
            if !st.exhausted() {
                return None;
            }
            return Some(vec![format!(
                "Effect::continuous(&Filter::Any, {modifier}, {duration})"
            )]);
        }
        let mut mods = Vec::new();
        let add_p = st.take("AddPower");
        let add_t = st.take("AddToughness");
        if add_p.is_some() || add_t.is_some() {
            let p: i16 = add_p
                .as_deref()
                .map_or(Some(0), |s| s.trim().parse().ok())?;
            let t: i16 = add_t
                .as_deref()
                .map_or(Some(0), |s| s.trim().parse().ok())?;
            mods.push(format!("Modifier::ModifyPT({p}, {t})"));
        }
        let set_p = st.take("SetPower");
        let set_t = st.take("SetToughness");
        if set_p.is_some() || set_t.is_some() {
            match (
                set_p.as_deref().map(|s| s.trim().parse::<i16>().ok()),
                set_t.as_deref().map(|s| s.trim().parse::<i16>().ok()),
            ) {
                (Some(Some(p)), Some(Some(t))) => mods.push(format!("Modifier::SetPT({p}, {t})")),
                (Some(Some(p)), None) => mods.push(format!("Modifier::SetPower({p})")),
                (None, Some(Some(t))) => mods.push(format!("Modifier::SetToughness({t})")),
                _ => return None,
            }
        }
        if let Some(raw_kw) = st.take("AddKeyword") {
            let kw = Self::pump_keyword_bits(&raw_kw)?;
            mods.push(format!("Modifier::AddKeyword({kw})"));
        }
        if let Some(hidden) = st.take("AddHiddenKeyword") {
            let m = match hidden.trim() {
                "CARDNAME can't block." => "Modifier::AddKeyword(KeywordSet::CANT_BLOCK)",
                "CARDNAME can't attack." => "Modifier::CantAttack",
                "CARDNAME can't attack or block." => {
                    "Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK))"
                }
                "CARDNAME attacks each combat if able." => "Modifier::AttacksEachCombat",
                "All creatures able to block CARDNAME do so." => "Modifier::MustBeBlockedByAllAble",
                "CARDNAME can block any number of creatures." => "Modifier::CanBlockAnyNumber",
                _ => return None,
            };
            mods.push(m.to_string());
        }
        match st.take("RemoveAllAbilities").as_deref() {
            None => {}
            Some("True") => mods.push("Modifier::LoseAllAbilities".to_string()),
            Some(_) => return None,
        }
        if let Some(types) = st.take("AddType") {
            for word in types.split('&').map(str::trim) {
                let tc = super::card_type_const(word)?;
                mods.push(format!("Modifier::AddType({tc})"));
            }
        }
        if !st.exhausted() {
            return None;
        }
        self.emit_effect_modifiers(subject, &affected, &mods, duration)
    }

    /// A target opponent guesses whether the chosen name is in your hand.
    /// Read both reveal branches as one decision, then an optional reveal and
    /// a draw only when the answer was wrong. Other generic choices refuse.
    pub(super) fn guess_named_card_effect(
        &mut self,
        mut p: Params,
        _sub: Option<&str>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if !matches!(
            target,
            Some("TargetSpec::AnyOpponent" | "TargetSpec::Player(PlayerRel::Chosen)")
        ) || p.take("Choices").as_deref() != Some("GuessInHand,GuessNotInHand")
            || p.take("Guess").as_deref() != Some("True")
            || p.take("ShowChoice").as_deref() != Some("True")
        {
            return None;
        }
        p.take("AILogic");
        if !p.exhausted() {
            return None;
        }
        for (reveal_name, draw_name, compare) in [
            ("GuessInHand", "DBInHandDraw", Some("EQ0")),
            ("GuessNotInHand", "DBNotInHandDraw", None),
        ] {
            let (api, mut reveal) = Params::parse(self.svars.get(reveal_name)?)?;
            reveal.drop_prose();
            let sub = reveal.take("SubAbility");
            if api != "RevealHand"
                || reveal.take("Defined").as_deref() != Some("You")
                || reveal.take("Optional").as_deref() != Some("True")
                || reveal.take("RememberRevealedPlayer").as_deref() != Some("True")
                || sub.as_deref() != Some(draw_name)
                || !reveal.exhausted()
            {
                return None;
            }
            let (draw_api, mut draw) = Params::parse(self.svars.get(draw_name)?)?;
            draw.drop_prose();
            if draw_api != "Draw"
                || draw.take("Defined").as_deref() != Some("You")
                || draw.take("ConditionPlayerDefined").as_deref() != Some("Remembered")
                || draw.take("ConditionPlayerContains").as_deref() != Some("You")
                || draw.take("ConditionCheckSVar").as_deref() != Some("X")
                || draw.take("ConditionSVarCompare").as_deref() != compare
                || !matches!(draw.take("NumCards").as_deref(), None | Some("1"))
            {
                return None;
            }
            let cleanup = draw.take("SubAbility")?;
            if self.svars.get(cleanup.trim()).map(String::as_str)
                != Some("DB$ Cleanup | ClearRemembered$ True")
                || !draw.exhausted()
                || self.svars.get("X").map(String::as_str) != Some("Count$ValidHand Card.NamedCard")
            {
                return None;
            }
        }
        Some(vec![
            "Effect::GuessChosenNameInHand { who: PlayerRel::You, guesser: PlayerRel::Chosen }"
                .to_string(),
            "Effect::MayDo { effects: &[Effect::RevealHandForGuess, Effect::DrawIfGuessWrong] }"
                .to_string(),
        ])
    }

    /// Non-targeted `GenericChoice | Choices$ A,B,...` chosen on resolution
    /// (CR 608.2d, e.g. `Catastrophe`), lowered to `Effect::ChooseOne`.
    pub(super) fn choose_one_effect(
        &mut self,
        mut p: Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some()
            || !matches!(p.take("Defined").as_deref(), None | Some("You"))
            || p.has("Guess")
            || p.has("ShowChoice")
            || p.has("ChoiceTitle")
            || p.has("Amount")
            || p.has("Mandatory")
            || p.has("Optional")
        {
            return None;
        }
        let raw_choices = p.take("Choices")?;
        p.take("AILogic");
        if !p.exhausted() {
            return None;
        }
        let mut choices = Vec::new();
        for name in raw_choices
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let body = self.svars.get(name)?.clone();
            let mut branch = super::Chain::default();
            self.chain(&body, &mut branch)?;
            if branch.target.is_some()
                || branch.second_target.is_some()
                || branch.effects.is_empty()
            {
                return None;
            }
            let one = match branch.effects.as_slice() {
                [only] => only.clone(),
                many => format!("Effect::Sequence(&[{}])", many.join(", ")),
            };
            choices.push(one);
        }
        if choices.len() < 2 {
            return None;
        }
        Some(vec![format!(
            "Effect::ChooseOne {{ choices: &[{}] }}",
            choices.join(", ")
        )])
    }

    /// Choose among every player tied for least life, then give that player
    /// control of the source. `ChangeController` already asks only on a tie.
    pub(super) fn choose_player_effect(
        &mut self,
        p: &mut Params,
        sub: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.take("Defined").as_deref() != Some("You")
            || p.take("Choices").as_deref() != Some("Player.withLowestLife")
        {
            return None;
        }
        p.take("ChoiceTitle");
        p.take("AILogic");
        if !p.exhausted() {
            return None;
        }
        let (api, mut child) = Params::parse(self.svars.get(sub?.trim())?)?;
        child.drop_prose();
        if api != "GainControl"
            || child.take("Defined").as_deref() != Some("Self")
            || child.take("NewController").as_deref() != Some("Player.Chosen")
            || !child.exhausted()
        {
            return None;
        }
        Some(vec![
            "Effect::ChangeController { new_controller: PlayerRel::LeastLifeOrTied }".to_string(),
        ])
    }

    pub(super) fn gain_control_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if targets_a_player
            && target == Some("TargetSpec::Player(PlayerRel::Chosen)")
            && p.take("Defined").as_deref() == Some("Self")
            && matches!(
                p.take("NewController").as_deref(),
                None | Some("Player.Chosen")
            )
        {
            return Some(vec![
                "Effect::ChangeController { new_controller: PlayerRel::Chosen }".to_string(),
            ]);
        }
        if target.is_none() && !targets_a_player {
            return self.gain_control_untargeted(p);
        }
        if target.is_none() || targets_a_player {
            return None;
        }
        if p.peek("SVar:X:Count") == Some("xPaid") {
            p.take("SVar:X:Count");
        }
        match p.take("Defined").as_deref() {
            None => {}
            Some("Targeted") if !targets_here => {}
            Some(_) => return None,
        }
        match p.take("NewController").as_deref() {
            None | Some("You") => {}
            Some(_) => return None,
        }
        let duration = match p.take("LoseControl").as_deref() {
            Some("EOT") => "Duration::UntilEndOfTurn",
            Some("LeavesPlay") if !self.on_a_spell => "Duration::WhileSourceOnBattlefield",
            Some("LeavesPlay,LoseControl" | "LoseControl,LeavesPlay") if !self.on_a_spell => {
                "Duration::WhileYouControlSource"
            }
            Some("Untap,LeavesPlay" | "LeavesPlay,Untap") if !self.on_a_spell => {
                "Duration::WhileSourceTapped { control: false, power_limited: false }"
            }
            Some(raw) if !self.on_a_spell && Self::is_untap_leaves_lose_control(raw) => {
                "Duration::WhileSourceTapped { control: true, power_limited: false }"
            }
            None => "Duration::Indefinitely",
            Some(_) => return None,
        };
        let untap = match p.take("Untap").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let mut out = Vec::new();
        if untap {
            out.push("Effect::UntapTarget".to_string());
        }
        out.push(format!(
            "Effect::continuous(&Filter::This, Modifier::GainControl, {duration})"
        ));
        if let Some(kw_raw) = p.take("AddKWs") {
            if duration != "Duration::UntilEndOfTurn" {
                return None;
            }
            let each: Option<Vec<&str>> =
                kw_raw.split('&').map(|k| keyword_const(k.trim())).collect();
            let kw =
                each?.join(".union(") + &")".repeat(kw_raw.split('&').count().saturating_sub(1));
            out.push(format!(
                "Effect::PumpTarget {{ power: Amount::Fixed(0), toughness: Amount::Fixed(0), \
                 keywords: {kw}, duration: Duration::UntilEndOfTurn }}"
            ));
        }
        Some(out)
    }

    fn gain_control_untargeted(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.peek("Defined") == Some("Self")
            && p.peek("NewController").is_some()
            && !p.has("LoseControl")
            && !p.has("AddKWs")
            && !p.has("AllValid")
        {
            let who_raw = p.take("NewController")?;
            let who = self.player_of_line(Some(&who_raw), None, false)?;
            if !Self::is_single_player_rel(who) {
                return None;
            }
            p.take("Defined");
            let untap = match p.take("Untap").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(_) => return None,
            };
            let mut out = Vec::new();
            if untap {
                out.push("Effect::UntapSelf".to_string());
            }
            out.push(format!(
                "Effect::ChangeController {{ new_controller: {who} }}"
            ));
            return Some(out);
        }
        if p.peek("AllValid").is_some() && p.peek("ConditionCheckSVar").is_some() {
            return self.gain_control_if_more_than_each_other(p);
        }
        if p.peek("AllValid").is_some()
            && !p.has("LoseControl")
            && !p.has("Untap")
            && !p.has("AddKWs")
        {
            let valid = p.take("AllValid")?;
            let who = match p.take("NewController").as_deref() {
                None | Some("You") => "PlayerRel::You",
                Some(raw) => self.player_of_line(Some(raw), None, false)?,
            };
            if !Self::is_single_player_rel(who) {
                return None;
            }
            let expr = self.filter_expr(&valid)?;
            let name = self.body.filter_static("GAIN_CONTROL", &expr);
            return Some(vec![format!(
                "Effect::GainControlAll {{ filter: &{name}, to: {who} }}"
            )]);
        }
        None
    }

    /// "If you control more [valid] than each other player, you gain control
    /// of all [valid]" (Peer Pressure).
    fn gain_control_if_more_than_each_other(&mut self, p: &mut Params) -> Option<Vec<String>> {
        let valid = p.take("AllValid")?;
        match p.take("NewController").as_deref() {
            None | Some("You") => {}
            Some(_) => return None,
        }
        let lhs = p.take("ConditionCheckSVar")?;
        let cmp = p.take("ConditionSVarCompare")?;
        let rhs = cmp.strip_prefix("GT")?;
        let expected_lhs = format!("Count$Valid {valid}+YouCtrl");
        let expected_rhs = format!("PlayerCountOther$HighestValid {valid}+YouCtrl");
        if self.svars.get(lhs.trim()).map(|s| s.trim()) != Some(expected_lhs.as_str())
            || self.svars.get(rhs.trim()).map(|s| s.trim()) != Some(expected_rhs.as_str())
        {
            return None;
        }
        let expr = self.filter_expr(&valid)?;
        let name = self.body.filter_static("GAIN_CONTROL", &expr);
        Some(vec![format!(
            "Effect::IfCondition {{ condition: Condition::ControlMoreThanEachOtherPlayer(&{name}), then: &[Effect::continuous(&{name}, Modifier::GainControl, Duration::Indefinitely)], otherwise: &[] }}"
        )])
    }

    /// `DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True`
    /// paired with a `DelayedTrigger` at the end step that returns `DelayTriggerRememberedLKI`
    /// under its owner's control (`Effect::ExileAndReturnAtEndStep` /
    /// `Effect::ExileAndReturnTappedAtEndStep`, Astral Slide, CR 603.7).
    /// `Defined$ TopOfLibrary | Origin$ Library | Destination$ Exile | RememberChanged$ True`
    /// chained into `DB$ Effect` granting `MayPlay$ True` from `Exile`
    /// (with optional `MayPlayWithoutManaCost$ True`) and optional `Cleanup`:
    /// lowers to `Effect::ExileTopToPlay`.
    pub(super) fn exile_top_to_play_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if p.take("Defined").as_deref() != Some("TopOfLibrary")
            || p.take("Origin").as_deref() != Some("Library")
            || p.take("Destination").as_deref() != Some("Exile")
            || p.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        let who = self.player_rel(p.take("DefinedPlayer").as_deref())?;
        let count = self.plain_num(p.take("ChangeNum").as_deref().unwrap_or("1"))?;
        let count = u8::try_from(count).ok().filter(|&n| n >= 1)?;
        self.exile_top_play_follow_up(who, count, sub)
    }

    /// `Dig | DigNum$ N | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True`
    /// chained into `DB$ Effect` granting `MayPlay$ True` from `Exile`
    /// (impulse draw: `Effect::ExileTopToPlay` / `Effect::ExileTopMayCast`).
    pub(super) fn dig_exile_top_to_play_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if targets_here && !targets_a_player {
            return self.deny("a `Dig` exile on a line that targets an object".into());
        }
        if p.take("ChangeNum").as_deref() != Some("All")
            || p.take("DestinationZone").as_deref() != Some("Exile")
            || p.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        if !matches!(p.take("ChangeValid").as_deref(), None | Some("Card")) {
            return None;
        }
        let who = self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let count = self.plain_num(&p.take("DigNum")?)?;
        let count = u8::try_from(count).ok().filter(|&n| n >= 1)?;
        self.exile_top_play_follow_up(who, count, sub)
    }

    fn exile_top_play_follow_up(
        &mut self,
        who: &str,
        count: u8,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let Some(eff_name) = sub.take() else {
            return self.deny("top-of-library exile without `SubAbility$`".into());
        };
        let Some(eff_body) = self.svars.get(&eff_name).cloned() else {
            return self.deny(format!("`SubAbility$ {eff_name}` names no SVar"));
        };
        let Some((eff_api, mut ep)) = Params::parse(&eff_body) else {
            return self.deny("top-of-library exile follow-up with no `$`".into());
        };
        ep.drop_prose();
        if eff_api != "Effect"
            || !matches!(
                ep.take("RememberObjects").as_deref(),
                Some("Remembered" | "RememberedCard")
            )
            || !matches!(ep.take("EffectOwner").as_deref(), None | Some("You"))
            || !matches!(
                ep.take("ExileOnMoved")
                    .or_else(|| ep.take("ForgetOnMoved"))
                    .as_deref(),
                None | Some("Exile")
            )
        {
            return self.deny(
                "a top-of-library exile follow-up other than an exiled-card play permission".into(),
            );
        }
        let window = match ep.take("Duration").as_deref() {
            None | Some("EndOfTurn" | "UntilEndOfTurn") => "PlayWindow::ThisTurn",
            Some("UntilYourNextTurn") => "PlayWindow::UntilYourNextTurn",
            Some("UntilYourNextUpkeep") => "PlayWindow::UntilYourNextUpkeep",
            Some("UntilTheEndOfYourNextTurn") => "PlayWindow::ThroughYourNextTurn",
            Some(other) => {
                return self.deny(format!(
                    "top-of-library exile permission `Duration$ {other}`"
                ));
            }
        };
        let Some(static_name) = ep.take("StaticAbilities") else {
            return self.deny("top-of-library exile `Effect` without `StaticAbilities$`".into());
        };
        let next_sub = ep.take("SubAbility");
        if let Some(key) = ep.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        let (free, mana_any_type, nonland_cast_only, spell_types) =
            self.exile_play_permission(&static_name)?;
        if let Some(cname) = next_sub {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        if nonland_cast_only {
            if count == 1 && window == "PlayWindow::ThisTurn" && !free && !mana_any_type {
                return Some(vec![format!(
                    "Effect::ExileTopMayCast {{ who: {who}, nonland_only: true }}"
                )]);
            }
            return self
                .deny("nonland-only exile cast permission outside `ExileTopMayCast`".into());
        }
        Some(vec![format!(
            "Effect::ExileTopToPlay {{ who: {who}, play: ExilePlay {{ count: {count}, \
             window: {window}, free: {free}, mana_any_type: {mana_any_type}, \
             spell_types: {spell_types} }} }}"
        )])
    }

    fn exile_play_permission(&mut self, static_name: &str) -> Option<(bool, bool, bool, String)> {
        let Some(static_body) = self.svars.get(static_name).cloned() else {
            return self.deny(format!("`StaticAbilities$ {static_name}` names no SVar"));
        };
        let Some((mode, mut sp)) = Params::parse(&static_body) else {
            return self.deny("top-of-library exile static ability without `Mode$`".into());
        };
        sp.drop_prose();
        if mode != "Continuous"
            || sp.take("MayPlay").as_deref() != Some("True")
            || sp.take("AffectedZone").as_deref() != Some("Exile")
        {
            return self.deny(
                "top-of-library exile static ability other than playing the exiled card".into(),
            );
        }
        let affected = sp.take("Affected")?;
        let (nonland_cast_only, spell_types) = match affected.as_str() {
            "Card.IsRemembered" => (false, "None".to_string()),
            "Card.IsRemembered+nonLand" | "Card.nonLand+IsRemembered" => (true, "None".to_string()),
            other => {
                let mut type_bits = Vec::new();
                for alt in other.split(',') {
                    let mut parts = alt.split(['.', '+']).map(str::trim);
                    let base = parts.next()?;
                    let rest: Vec<&str> = parts.collect();
                    if rest != ["IsRemembered"] {
                        return self.deny(
                            "top-of-library exile static ability other than playing the exiled card"
                                .into(),
                        );
                    }
                    let tc = super::card_type_const(base)?;
                    if tc == "TypeSet::LAND" {
                        return self.deny("top-of-library land-only exile permission".into());
                    }
                    type_bits.push(tc);
                }
                if type_bits.is_empty() {
                    return None;
                }
                let mut expr = type_bits[0].to_string();
                for bit in &type_bits[1..] {
                    expr = format!("{expr}.union({bit})");
                }
                (false, format!("Some({expr})"))
            }
        };
        let free = match sp.take("MayPlayWithoutManaCost").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!(
                    "top-of-library exile `MayPlayWithoutManaCost$ {other}`"
                ));
            }
        };
        let mana_any_type = match sp.take("MayPlayIgnoreType").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("top-of-library exile `MayPlayIgnoreType$ {other}`"));
            }
        };
        if let Some(key) = sp.first_key() {
            return self.deny(format!("unclaimed parameter `StaticAbility.{key}`"));
        }
        Some((free, mana_any_type, nonland_cast_only, spell_types))
    }

    fn consume_optional_remember_cleanup(
        &mut self,
        cname: String,
        sub: &mut Option<String>,
    ) -> Option<()> {
        let Some(c_body) = self.svars.get(&cname).cloned() else {
            return self.deny(format!("`SubAbility$ {cname}` names no SVar"));
        };
        let Some((c_api, mut cp)) = Params::parse(&c_body) else {
            return self.deny("top-of-library exile cleanup with no `$`".into());
        };
        cp.drop_prose();
        if c_api == "Cleanup" {
            if cp.take("ClearRemembered").as_deref() != Some("True") {
                return self.deny("cleanup other than `ClearRemembered$ True`".into());
            }
            *sub = cp.take("SubAbility");
            if let Some(key) = cp.first_key() {
                return self.deny(format!("unclaimed parameter `Cleanup.{key}`"));
            }
        } else {
            *sub = Some(cname);
        }
        Some(())
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn exile_until_end_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        single_target: bool,
    ) -> Option<Vec<String>> {
        if !target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) {
            return self.deny("an end-step blink that does not target a battlefield object".into());
        }
        let remembered = p.take("RememberChanged").as_deref() == Some("True")
            || p.take("RememberTargets").as_deref() == Some("True");
        if p.take("Origin").as_deref() != Some("Battlefield")
            || p.take("Destination").as_deref() != Some("Exile")
            || !remembered
        {
            return self.deny("an end-step blink missing `RememberChanged$ True`".into());
        }
        for key in ["ForgetOtherTargets", "ForgetOtherRemembered"] {
            if let Some(val) = p.take(key)
                && val != "True"
            {
                return self.deny(format!("a blink with `{key}$ {val}`"));
            }
        }
        p.take("IsCurse");
        let Some(del_name) = sub.take() else {
            return self.deny("an end-step blink with no delayed trigger".into());
        };
        let Some(del_body) = self.svars.get(&del_name).cloned() else {
            return self.deny(format!("`SubAbility$ {del_name}` names no SVar"));
        };
        let Some((del_api, mut dp)) = Params::parse(&del_body) else {
            return self.deny("a blink delayed trigger with no `$` in it".into());
        };
        dp.drop_prose();
        if del_api == "ChangeZone" {
            let _ = single_target;
            if !matches!(
                dp.take("Defined").as_deref(),
                Some("Remembered" | "RememberedLKI")
            ) || !matches!(dp.take("Origin").as_deref(), Some("Exile" | "All"))
                || dp.take("Destination").as_deref() != Some("Battlefield")
            {
                return self
                    .deny("a `ChangeZone` exile follow-up other than an immediate blink".into());
            }
            if let Some(forget) = dp.take("ForgetOtherRemembered")
                && forget != "True"
            {
                return self.deny(format!(
                    "a blink return with `ForgetOtherRemembered$ {forget}`"
                ));
            }
            let owner_control = match dp.take("GainControl").as_deref() {
                None | Some("False") => true,
                Some("True") => false,
                Some(other) => {
                    return self.deny(format!("a blink return with `GainControl$ {other}`"));
                }
            };
            let cleanup_name = dp.take("SubAbility");
            if let Some(key) = dp.first_key() {
                return self.deny(format!("unclaimed parameter `BlinkReturn.{key}`"));
            }
            if let Some(cname) = cleanup_name {
                self.consume_optional_remember_cleanup(cname, sub)?;
            }
            let tgt = target.expect("validated battlefield object target");
            return Some(vec![format!(
                "Effect::Blink {{ target: {tgt}, owner_control: {owner_control} }}"
            )]);
        }
        if del_api != "DelayedTrigger"
            || dp.take("Mode").as_deref() != Some("Phase")
            || !dp
                .take("Phase")
                .is_some_and(|ph| ph.eq_ignore_ascii_case("End of Turn"))
            || !matches!(dp.take("ValidPlayer").as_deref(), None | Some("Player"))
            || !matches!(
                dp.take("RememberObjects").as_deref(),
                Some("RememberedLKI" | "Remembered")
            )
        {
            return self
                .deny("a `ChangeZone` delayed trigger that is not an end-step LKI return".into());
        }
        if dp.peek("ConditionDefined") == Some("Remembered")
            && dp.peek("ConditionPresent") == Some("Card")
        {
            dp.take("ConditionDefined");
            dp.take("ConditionPresent");
        }
        let Some(ret_name) = dp.take("Execute") else {
            return self.deny("a blink delayed trigger with no `Execute$`".into());
        };
        let cleanup_name = dp.take("SubAbility");
        if let Some(key) = dp.first_key() {
            return self.deny(format!("unclaimed parameter `DelayedTrigger.{key}`"));
        }
        let Some(ret_body) = self.svars.get(&ret_name).cloned() else {
            return self.deny(format!("`Execute$ {ret_name}` names no SVar"));
        };
        let Some((ret_api, mut rp)) = Params::parse(&ret_body) else {
            return self.deny("a blink return ability with no `$` in it".into());
        };
        rp.drop_prose();
        if ret_api != "ChangeZone"
            || !matches!(
                rp.take("Defined").as_deref(),
                Some("DelayTriggerRememberedLKI" | "DelayTriggerRemembered")
            )
            || !matches!(rp.take("Origin").as_deref(), Some("Exile" | "All"))
            || rp.take("Destination").as_deref() != Some("Battlefield")
        {
            return self.deny(
                "a blink delayed return that does not return `DelayTriggerRememberedLKI` to the battlefield"
                    .into(),
            );
        }
        let tapped = match rp.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("a blink delayed return with `Tapped$ {other}`"));
            }
        };
        if let Some(key) = rp.first_key() {
            return self.deny(format!("unclaimed parameter `DelayedReturn.{key}`"));
        }
        if let Some(cname) = cleanup_name {
            let Some(c_body) = self.svars.get(&cname).cloned() else {
                return self.deny(format!("`SubAbility$ {cname}` names no SVar"));
            };
            let Some((c_api, mut cp)) = Params::parse(&c_body) else {
                return self.deny("a blink cleanup ability with no `$` in it".into());
            };
            cp.drop_prose();
            if c_api != "Cleanup" || cp.take("ClearRemembered").as_deref() != Some("True") {
                return self.deny("a blink sub-ability other than `ClearRemembered$ True`".into());
            }
            *sub = cp.take("SubAbility");
            if let Some(key) = cp.first_key() {
                return self.deny(format!("unclaimed parameter `Cleanup.{key}`"));
            }
        }
        Some(vec![if tapped {
            "Effect::ExileAndReturnTappedAtEndStep".to_string()
        } else {
            "Effect::ExileAndReturnAtEndStep".to_string()
        }])
    }

    /// `ChangeZone` for the zone pairs the engine has an effect for.
    ///
    /// The corpus writes every zone change with one API and two zone names; the
    /// engine has a named effect per movement, because the movements differ
    /// in rules and not only in destination. So this is a table of pairs,
    /// not a translation of `Destination$` — and a pair with no effect
    /// refuses rather than reaching for the nearest one. Battlefield →
    /// Graveyard is the pair that makes the point: it is *not* `Destroy`,
    /// which checks indestructible (CR 702.12b), and generating one for the
    /// other would quietly kill creatures that survive.
    #[allow(clippy::too_many_lines)]
    pub(super) fn change_zone(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let origin = p.take("Origin")?;
        let destination = p.take("Destination")?;
        p.claim_label("ChangeTypeDesc", "ChangeType");
        // A library search is a different effect, not a zone change with a
        // hidden target: a card in a library cannot be targeted at all
        // (CR 115.2 needs a visible object), so `Effect::SearchLibrary`
        // *finds* rather than moves, and carries the shuffle with it.
        if origin == "Library" {
            return self.search_library(p, &destination, targets_here);
        }
        let target = target.unwrap_or("TargetSpec::AnyPlayer");
        let itself = match p.take("Defined").as_deref() {
            // A sub-ability `DB$ ChangeZone` that declares neither its own
            // `ValidTgts$` nor `Defined$` nor `Hidden$` defaults in the corpus
            // to `Self` (Cyclopean Snare's `SVar:DBBounce:DB$ ChangeZone |
            // Origin$ Battlefield | Destination$ Hand`), never the parent
            // ability's target; and an untargeted `Origin$ Battlefield |
            // Destination$ Hand` with no `ChangeType$` on a permanent's own
            // ability is "return CARDNAME to its owner's hand" (Darting
            // Merfolk).
            None => {
                self.defaults_to_self_zone_change(p, &origin, &destination, target, targets_here)
            }
            Some("Self") => true,
            Some("Targeted") if !targets_here && target != "TargetSpec::AnyPlayer" => false,
            Some("ExiledWith")
                if origin == "Exile"
                    && destination == "Battlefield"
                    && target == "TargetSpec::AnyPlayer" =>
            {
                return Some(vec!["Effect::ReturnLinkedToBattlefield".to_string()]);
            }
            Some("Enchanted" | "Equipped")
                if !self.on_a_spell
                    && !self.cost_sacrifices_self
                    && target == "TargetSpec::AnyPlayer"
                    && origin == "Battlefield" =>
            {
                return match destination.as_str() {
                    "Exile" => Some(vec![
                        "Effect::ExileAll { filter: &Filter::AttachedToBySource, linked: false }"
                            .to_string(),
                    ]),
                    "Hand" => Some(vec![
                        "Effect::ReturnAllToHand { filter: &Filter::AttachedToBySource, opponents_only: false }"
                            .to_string(),
                    ]),
                    _ => self.deny(format!("`ChangeZone` of `Defined$ Enchanted` to `{destination}`")),
                };
            }
            Some("Enchanted & Self" | "Self & Enchanted")
                if !self.on_a_spell
                    && !self.cost_sacrifices_self
                    && target == "TargetSpec::AnyPlayer"
                    && origin == "Battlefield"
                    && destination == "Hand" =>
            {
                return Some(vec![
                    "Effect::ReturnAllToHand { filter: &Filter::AttachedToBySource, opponents_only: false }"
                        .to_string(),
                    "Effect::bounce(TargetSpec::ThisObject)".to_string(),
                ]);
            }
            Some(def) if target == "TargetSpec::AnyPlayer" && self.is_event_object_defined(def) => {
                return self.change_zone_event_object(p, def, &origin, &destination);
            }
            Some(other) => return self.deny(format!("`ChangeZone` of `Defined$ {other}`")),
        };
        // "Return a land you control to its owner's hand": no target, no
        // `Defined$`, and a player picking one of their own permanents while
        // the ability resolves.
        if !itself
            && target == "TargetSpec::AnyPlayer"
            && origin == "Battlefield"
            && p.has("Hidden")
        {
            return self.return_chosen(p, &destination);
        }
        // "You may put a basic land card from your hand onto the battlefield"
        // (Firebrand Ranger, Skyshroud Ranger): no target, no `Defined$`, and
        // the controller optionally puts one matching card from their hand
        // onto the battlefield.
        if !itself
            && target == "TargetSpec::AnyPlayer"
            && origin == "Hand"
            && destination == "Battlefield"
            && p.has("ChangeType")
            && (p.has("Optional")
                || p.peek("Mandatory") == Some("True")
                || (!p.has("Mandatory")
                    && self.oracle.to_ascii_lowercase().contains("you may put")))
        {
            return self.put_from_hand_onto_battlefield(p);
        }
        if !itself
            && target == "TargetSpec::AnyPlayer"
            && origin == "Hand"
            && destination == "Exile"
            && p.take("Imprint").as_deref() == Some("True")
        {
            if p.take("ChangeNum").as_deref() != Some("1") {
                return None;
            }
            let valid = p.take("ChangeType")?;
            let expr = self.filter_expr_in_zone(&valid, true)?;
            let filter = self.body.filter_static("IMPRINT", &expr);
            return Some(vec![format!(
                "Effect::ExileChosenFromHandWithSource(&{filter})"
            )]);
        }
        if !itself
            && origin == "Hand"
            && destination == "Library"
            && matches!(
                target,
                "TargetSpec::AnyPlayer" | "TargetSpec::Player(PlayerRel::Chosen)"
            )
            && matches!(p.peek("LibraryPosition"), Some("0" | "-1"))
            && (p.peek("Mandatory") == Some("True") || p.peek("Optional") == Some("True"))
        {
            return self.hand_to_library_change_zone(p, target, targets_here);
        }
        if !itself
            && target == "TargetSpec::AnyPlayer"
            && origin == "Graveyard"
            && destination == "Hand"
            && p.peek("Hidden") == Some("True")
            && p.peek("AtRandom") == Some("True")
            && p.has("ChangeType")
        {
            return self.random_graveyard_to_hand(p);
        }
        if !itself
            && target == "TargetSpec::AnyPlayer"
            && origin == "Graveyard"
            && matches!(
                destination.as_str(),
                "Battlefield" | "Hand" | "Exile" | "Library"
            )
            && p.peek("Hidden") == Some("True")
            && p.has("ChangeType")
            && (p.peek("Mandatory") == Some("True")
                || p.peek("Optional") == Some("True")
                || (!p.has("Mandatory")
                    && !p.has("Optional")
                    && (p
                        .peek("SelectPrompt")
                        .is_some_and(|s| s.to_ascii_lowercase().starts_with("you may"))
                        || self.oracle.to_ascii_lowercase().contains("you may "))))
        {
            return self.choose_from_your_graveyard(p, &destination);
        }
        // Without a target this would move nothing at all.
        if !itself && target == "TargetSpec::AnyPlayer" {
            return self.deny("`ChangeZone` with neither a target nor `Defined$`".to_string());
        }
        Some(match (origin.as_str(), destination.as_str(), itself) {
            ("Battlefield" | "Graveyard", "Hand", true) => {
                vec!["Effect::bounce(TargetSpec::ThisObject)".to_string()]
            }
            ("Graveyard", "Battlefield", true) => {
                let tapped = match p.take("Tapped").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let face = match p.take("Transformed").as_deref() {
                    None | Some("False") => "None",
                    Some("True") => "Some(1)",
                    Some(_) => return None,
                };
                let counters = self.take_with_counters(p)?;
                vec![format!(
                    "Effect::ReturnSourceFromGraveyard {{ face: {face}, tapped: {tapped}, \
                     owner_control: false, counters: {counters} }}"
                )]
            }
            ("Battlefield", "Hand", false) => vec![format!("Effect::bounce({target})")],
            ("Battlefield", "Library", true)
                if p.take("LibraryPosition").as_deref() == Some("0") =>
            {
                vec!["Effect::PutSourceOnTopOfLibrary".to_string()]
            }
            ("Battlefield", "Library", false) if target.starts_with("TargetSpec::Object(") => {
                match (
                    p.take("Shuffle").as_deref(),
                    p.take("LibraryPosition").as_deref(),
                ) {
                    (Some("True"), None) => {
                        vec!["Effect::ShuffleTargetsIntoOwnersLibraries".to_string()]
                    }
                    (None, Some("0")) => {
                        vec!["Effect::PutTargetOnTopOfLibrary".to_string()]
                    }
                    (None, Some("-1")) => {
                        p.take("IsCurse");
                        vec!["Effect::PutTargetOnBottomOfLibrary".to_string()]
                    }
                    _ => {
                        return self
                            .deny("`ChangeZone` from `Battlefield` to `Library`".to_string());
                    }
                }
            }
            ("Battlefield", "Exile", false) => {
                p.take("IsCurse");
                if p.peek("Duration") == Some("UntilHostLeavesPlay") {
                    if self.on_a_spell
                        || !single_target
                        || !target.starts_with("TargetSpec::Object(")
                        || p.peek("ThisDefinedAndTgts").is_some()
                    {
                        return None;
                    }
                    p.take("Duration");
                    return Some(vec![format!(
                        "Effect::exile_until({target}, ExileUntil::SourceLeavesBattlefield)"
                    )]);
                }
                let verb = if self.svars.values().any(|s| {
                    s.contains("Defined$ ExiledWith") || s.contains("Card.ExiledWithSource")
                }) {
                    "exile_linked"
                } else {
                    "exile"
                };
                match p.take("ThisDefinedAndTgts").as_deref() {
                    None => vec![format!("Effect::{verb}({target})")],
                    Some("Self") if verb == "exile" => vec![
                        "Effect::ExileSource".to_string(),
                        format!("Effect::{verb}({target})"),
                    ],
                    Some(other) => {
                        return self
                            .deny(format!("`ChangeZone` with `ThisDefinedAndTgts$ {other}`"));
                    }
                }
            }
            ("Graveyard", "Exile", false) if target.starts_with("TargetSpec::CardInGraveyard") => {
                match p.take("Imprint").as_deref() {
                    Some("True") => vec!["Effect::ExileTargetsWithSource".to_string()],
                    None => vec![format!("Effect::exile({target})")],
                    Some(other) => {
                        return self.deny(format!("`ChangeZone` with `Imprint$ {other}`"));
                    }
                }
            }
            ("Battlefield" | "Stack", "Exile", true) => vec!["Effect::ExileSource".to_string()],
            ("Exile", "Graveyard", true) => {
                vec!["Effect::MoveSourceToOwnerGraveyard".to_string()]
            }
            // "Return target card from your graveyard to your hand"
            // (Regrowth): the target is a `CardInGraveyard`, which the chain
            // read off `Origin$` before it read the valid-string.
            ("Graveyard", "Hand", false) if target.starts_with("TargetSpec::CardInGraveyard") => {
                if let Some(num) = p.take("ChangeNum")
                    && num != "1"
                {
                    return self.deny(format!("`ChangeZone` with `ChangeNum$ {num}`"));
                }
                vec![format!("Effect::GraveyardToHand {{ target: {target} }}")]
            }
            ("Graveyard", "Library", false)
                if target.starts_with("TargetSpec::CardInGraveyard") =>
            {
                match p.take("LibraryPosition").as_deref() {
                    None | Some("0") => {
                        vec![format!("Effect::GraveyardToTop {{ target: {target} }}")]
                    }
                    Some("-1") => vec![format!(
                        "Effect::PutOnBottomOfLibraryFromGraveyard {{ target: {target} }}"
                    )],
                    Some(other) => {
                        return self.deny(format!("`ChangeZone` with `LibraryPosition$ {other}`"));
                    }
                }
            }
            // "Return target creature card from your graveyard to the
            // battlefield" (Resurrection): it enters under the control of
            // the player whose effect put it there (CR 110.2a), which is
            // `owner_control: false`. A line that says otherwise carries a
            // key (`GainControl$`, `WithCountersType$`) this does not claim.
            ("Graveyard", "Battlefield", false)
                if target.starts_with("TargetSpec::CardInGraveyard") =>
            {
                if let Some(gain) = p.take("GainControl")
                    && gain != "True"
                {
                    return self.deny(format!("`ChangeZone` with `GainControl$ {gain}`"));
                }
                if let Some(num) = p.take("ChangeNum")
                    && num != "1"
                {
                    return self.deny(format!("`ChangeZone` with `ChangeNum$ {num}`"));
                }
                let tapped = match p.take("Tapped").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(other) => {
                        return self.deny(format!("`ChangeZone` with `Tapped$ {other}`"));
                    }
                };
                let counters = self.take_with_counters(p)?;
                if tapped {
                    vec![format!(
                        "Effect::GraveyardToBattlefieldModified {{ target: {target}, \
                         owner_control: false, tapped: true, counters: {counters}, \
                         modifiers: &[] }}"
                    )]
                } else {
                    vec![format!(
                        "Effect::GraveyardToBattlefield {{ target: {target}, \
                         owner_control: false, counters: {counters} }}"
                    )]
                }
            }
            _ => return self.deny(format!("`ChangeZone` {origin} to {destination}")),
        })
    }

    fn take_with_counters(&self, p: &mut Params) -> Option<String> {
        if let Some(code) = p.take("WithCountersType") {
            let Some(kind) = super::counter_kind(&code) else {
                return self.deny(format!("counter `{code}`"));
            };
            let n: u16 = self
                .plain_num(p.take("WithCountersAmount").as_deref().unwrap_or("1"))?
                .try_into()
                .ok()?;
            if n == 0 {
                return None;
            }
            Some(format!("Some(({kind}, {n}))"))
        } else {
            Some("None".to_string())
        }
    }

    fn defaults_to_self_zone_change(
        &self,
        p: &Params,
        origin: &str,
        destination: &str,
        target: &str,
        targets_here: bool,
    ) -> bool {
        !targets_here
            && !p.has("Hidden")
            && !p.has("DefinedPlayer")
            && (target != "TargetSpec::AnyPlayer"
                || (self.on_a_spell
                    && origin == "Stack"
                    && destination == "Exile"
                    && !p.has("ChangeType"))
                || (!self.on_a_spell
                    && ((origin == "Battlefield"
                        && (matches!(destination, "Hand" | "Exile")
                            || (destination == "Library"
                                && p.peek("LibraryPosition") == Some("0"))))
                        || (origin == "Graveyard"
                            && matches!(destination, "Battlefield" | "Hand")))
                    && !p.has("ChangeType")))
    }

    fn hand_to_library_change_zone(
        &mut self,
        p: &mut Params,
        target: &str,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if let Some(hidden) = p.take("Hidden")
            && hidden != "True"
        {
            return self.deny(format!("`ChangeZone` with `Hidden$ {hidden}`"));
        }
        let pos = p.take("LibraryPosition")?;
        let targets_a_player = targets_here && target == "TargetSpec::Player(PlayerRel::Chosen)";
        let defined_player = p.take("DefinedPlayer");
        let player = if let Some(who) = defined_player.as_deref() {
            self.player_of_line(Some(who), Some(target), targets_a_player)?
        } else if targets_a_player {
            "PlayerRel::Chosen"
        } else {
            "PlayerRel::You"
        };
        if pos == "0" {
            if p.take("Mandatory").as_deref() != Some("True") {
                return None;
            }
            if let Some(ct) = p.take("ChangeType")
                && ct != "Card"
            {
                return None;
            }
            let count: u8 = self
                .plain_num(p.take("ChangeNum").as_deref().unwrap_or("1"))?
                .try_into()
                .ok()?;
            if count == 0 || count > 3 {
                return None;
            }
            return Some(vec![if player == "PlayerRel::You" {
                format!("Effect::PutFromHandOnTop {{ count: {count} }}")
            } else {
                format!("Effect::PutFromHandOnTopFor {{ player: {player}, count: {count} }}")
            }]);
        }
        if pos == "-1" && p.take("Optional").as_deref() == Some("True") {
            if !matches!(p.take("ChangeNum").as_deref(), None | Some("1")) {
                return None;
            }
            let filter = match p.take("ChangeType").as_deref() {
                None | Some("Card") => "Filter::Any".to_string(),
                Some(valid) => {
                    let expr = self.filter_expr_in_zone(valid, true)?;
                    self.body.filter_static("BOTTOM", &expr)
                }
            };
            return Some(vec![format!(
                "Effect::BottomCardFromHand {{ player: {player}, filter: &{filter}, then: &[] }}"
            )]);
        }
        None
    }

    fn choose_from_your_graveyard(
        &mut self,
        p: &mut Params,
        destination: &str,
    ) -> Option<Vec<String>> {
        if p.take("Hidden").as_deref() != Some("True") {
            return None;
        }
        let prompt_may = p
            .take("SelectPrompt")
            .is_some_and(|s| s.to_ascii_lowercase().starts_with("you may"));
        let optional = match (
            p.take("Mandatory").as_deref(),
            p.take("Optional").as_deref(),
        ) {
            (Some("True"), None) => false,
            (None, Some("True")) => true,
            (None, None) if prompt_may || self.oracle.to_ascii_lowercase().contains("you may ") => {
                true
            }
            _ => return None,
        };
        if !matches!(p.take("ChangeNum").as_deref(), None | Some("1")) {
            return None;
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") if destination == "Battlefield" => true,
            Some(_) => return None,
        };
        let dest = match destination {
            "Battlefield" => "SearchDest::Battlefield",
            "Hand" => "SearchDest::Hand",
            "Exile" => "SearchDest::Exile",
            "Library" if p.take("LibraryPosition").as_deref() == Some("0") => {
                "SearchDest::TopOfLibrary"
            }
            _ => return None,
        };
        let valid = p.take("ChangeType")?;
        let stripped = Self::strip_your_graveyard_owner(&valid)?;
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let filter = self.body.filter_static("GRAVE_CHOICE", &expr);
        Some(vec![format!(
            "Effect::ChooseFromYourGraveyard {{ filter: &{filter}, destination: {dest}, \
             tapped: {tapped}, optional: {optional} }}"
        )])
    }

    /// `Origin$ Battlefield` with a chooser rather than a target: the bounce
    /// land's "return a land you control to its owner's hand".
    ///
    /// **`Hidden$ True` is the discriminator, not decoration**, and that is
    /// a measurement rather than a reading of the word. Of the reference's
    /// 173 untargeted `Battlefield` → `Hand` lines, 96 carry a
    /// `ChangeType$` and **all 96** of those carry `Hidden$ True`; of the 77
    /// that name no filter — "return this land to its owner's hand", a
    /// different card — exactly one does. So the key separates "somebody
    /// chooses from the battlefield" from "this moves itself" cleanly, and
    /// the branch above is guarded on its presence so a line without it
    /// still gets the old, correct report.
    ///
    /// `Mandatory$ True` is required for the reason [`Self::search_library`]
    /// requires a word about its count: [`Effect::ReturnChosenToHand`] is an
    /// instruction and "you may return" is a different card, so a script
    /// that does not say which is refused rather than read as either. It
    /// separates the corpus 81 to 15 and every one of the twelve lands this
    /// was written for is on the mandatory side.
    ///
    /// Everything else is refused by not being claimed — `DefinedPlayer$`
    /// and `Chooser$` (14 and 9 lines that hand the choice to somebody
    /// else), `Optional$`, `UnlessCost$`, `RememberLKI$`. Each is a rule
    /// this does not have, and the generic unclaimed-parameter report names
    /// it better than a guess would.
    ///
    /// [`Effect::ReturnChosenToHand`]: baylee_cards_dsl::Effect::ReturnChosenToHand
    pub(super) fn return_chosen(
        &mut self,
        p: &mut Params,
        destination: &str,
    ) -> Option<Vec<String>> {
        if destination != "Hand" {
            self.note(format!(
                "`ChangeZone` chosen off the battlefield into {destination}"
            ));
            return None;
        }
        for (key, expected) in [("Hidden", "True"), ("Mandatory", "True")] {
            match p.take(key).as_deref() {
                Some(value) if value == expected => {}
                Some(other) => {
                    self.note(format!("`ChangeZone` with `{key}$ {other}`"));
                    return None;
                }
                None => {
                    self.note(format!("`ChangeZone` chosen without `{key}$`"));
                    return None;
                }
            }
        }
        // One permanent, because that is the only count the effect states.
        // A sentence returning two is a different rule and says so here
        // rather than writing a card that returns one of them.
        let count = self.plain_num(p.take("ChangeNum").as_deref().unwrap_or("1"))?;
        if count != 1 {
            self.note(format!("`ChangeZone` returning {count} chosen permanents"));
            return None;
        }
        let who = self.player_rel(p.take("DefinedPlayer").as_deref())?;
        let change_type = p.take("ChangeType")?;
        if who != "PlayerRel::You" && change_type.contains("YouCtrl") {
            return None;
        }
        let filter = self.filter_expr(&change_type)?;
        let name = self.body.filter_static("RETURN", &filter);
        Some(vec![format!(
            "Effect::ReturnChosenToHand {{ who: {who}, filter: &{name} }}"
        )])
    }

    /// `Origin$ Hand` to `Destination$ Battlefield` with an explicit `Optional$`:
    /// "you may put a [matching] card from your hand onto the battlefield"
    /// (Firebrand Ranger, Skyshroud Ranger).
    pub(super) fn put_from_hand_onto_battlefield(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if let Some(hidden) = p.take("Hidden")
            && hidden != "True"
        {
            return self.deny(format!("`ChangeZone` found `Hidden$ {hidden}`"));
        }
        p.take("SelectPrompt");
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Tapped$ {other}`"));
                return None;
            }
        };
        let mandatory = match p.take("Mandatory").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Mandatory$ {other}`"));
                return None;
            }
        };
        let optional = match p.take("Optional").as_deref() {
            Some("You" | "True") if !mandatory => true,
            Some("False") => false,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Optional$ {other}`"));
                return None;
            }
            None if mandatory => false,
            None if self.oracle.to_ascii_lowercase().contains("you may put") => true,
            None => {
                self.note("`ChangeZone` from hand without `Optional$`".to_string());
                return None;
            }
        };
        let count = self.plain_num(p.take("ChangeNum").as_deref().unwrap_or("1"))?;
        if count != 1 {
            self.note(format!("`ChangeZone` putting {count} cards from hand"));
            return None;
        }
        let raw_valid = p.take("ChangeType")?;
        let cleaned = Self::strip_your_graveyard_owner(&raw_valid).unwrap_or(raw_valid);
        let filter = self.filter_expr_in_zone(&cleaned, true)?;
        let name = self.body.filter_static("PUT_FROM_HAND", &filter);
        Some(vec![format!(
            "Effect::PutFromHandOntoBattlefield {{ tapped: {tapped}, filter: &{name}, \
             mana_value: None, optional: {optional} }}"
        )])
    }

    /// `Origin$ Library`: the fetchland sentence — "search your library for
    /// an Island or Swamp card, put it onto the battlefield, then shuffle".
    ///
    /// `finds` is positional and its length is how many cards may be found,
    /// so `ChangeNum$ 2` is the same `Find` twice rather than a count beside
    /// it. Nothing here targets, and a line that says it does is a different
    /// card.
    pub(super) fn search_library(
        &mut self,
        p: &mut Params,
        destination: &str,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here {
            self.note("`ChangeZone` searching a library *and* targeting".to_string());
            return None;
        }
        if destination == "Library"
            && p.peek("Shuffle") == Some("False")
            && p.peek("Imprint") == Some("True")
            && p.peek("Reveal") == Some("True")
        {
            return self.reveal_library_search(p);
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Tapped$ {other}`"));
                return None;
            }
        };
        let any_number = self.any_number_library_search(p, destination)?;
        let base_find = match (destination, tapped) {
            ("Battlefield", false) => "Find::BATTLEFIELD",
            ("Battlefield", true) => "Find::BATTLEFIELD_TAPPED",
            ("Hand", false) => "Find::HAND",
            ("Exile", false) => "Find::EXILE",
            ("Graveyard", false) => "Find::GRAVEYARD",
            ("Library", false)
                if !any_number
                    && matches!(p.take("LibraryPosition").as_deref(), None | Some("0")) =>
            {
                "Find::TOP_OF_LIBRARY"
            }
            _ => {
                self.note(format!("`ChangeZone` searching into {destination}"));
                return None;
            }
        };
        let find = if any_number {
            format!("{base_find}.any_number()")
        } else {
            base_find.to_string()
        };
        // "You may search" and "search for up to two" are both this flag:
        // the player may end up with fewer cards than `finds` allows.
        let optional = match p.take("Optional").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Optional$ {other}`"));
                return None;
            }
        };
        let shuffle_non_mandatory = match p.take("ShuffleNonMandatory").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` found `ShuffleNonMandatory$ {other}`"));
                return None;
            }
        };
        let optional = optional || shuffle_non_mandatory || any_number;
        // Claimed rather than read: `Mandatory$ True` is the default, every
        // search this emitter writes shuffles afterwards, and `Hidden$ True`
        // only says the library is a hidden zone, which it is.
        //
        // The shuffle is *this emitter's* convention and not a rule — no
        // sub-rule of CR 701.23 makes a search shuffle; the card's own "then
        // shuffle" is what does, as the separate action CR 701.24 names. So
        // `Shuffle$ False` has to be refused here rather than read, because
        // nothing downstream can express a search that leaves the library in
        // order.
        // Asked before the loop below consumes it, because it is the only
        // word that says a count above one is a requirement.
        let mandatory = p.has("Mandatory");
        for (key, expected) in [
            ("Mandatory", "True"),
            ("Shuffle", "True"),
            ("Hidden", "True"),
        ] {
            if let Some(value) = p.take(key)
                && value != expected
            {
                self.note(format!("`ChangeZone` with `{key}$ {value}`"));
                return None;
            }
        }
        let count = self.plain_num(p.take("ChangeNum").as_deref().unwrap_or("1"))?;
        let count = usize::try_from(count).ok()?;
        if count == 0 || count > 4 {
            self.note(format!("`ChangeZone` finding {count} cards"));
            return None;
        }
        // "Search your library for **up to** two basic land cards" and
        // "search your library for three cards and reveal them" are two
        // different cards, and above a count of one the difference is what
        // the player is allowed to find — `optional` is exactly that flag.
        //
        // The script does not always say which it is. Measured over the
        // reference: 137 lines search a library for more than one card, and
        // of the 135 that carry no `Optional$`, 70 print "up to" and 65 do
        // not — identical fields, opposite cards. `Mandatory$ True` does
        // separate one side cleanly (36 lines, **none** of them "up to"),
        // so a script that says either word is read and a script that says
        // neither is refused. Reading the absence of a field as "up to"
        // would be an inference rather than a reading, and it would have
        // written Blighted Woodland — "up to two" — as a card that must
        // find both.
        let optional = optional
            || (count > 1
                && !mandatory
                && self
                    .oracle
                    .to_ascii_lowercase()
                    .contains("search your library for up to "));
        if count > 1 && !optional && !mandatory {
            self.note(
                "`ChangeZone` finding several cards without saying whether that is a maximum"
                    .to_string(),
            );
            return None;
        }
        let filter = self.filter_expr_in_zone(&p.take("ChangeType")?, true)?;
        let name = self.body.filter_static("SEARCH", &filter);
        let finds = vec![find.as_str(); count].join(", ");
        Some(vec![format!(
            "Effect::SearchLibrary {{ filter: &{name}, finds: &[{finds}], optional: {optional} }}"
        )])
    }

    fn reveal_library_search(&mut self, p: &mut Params) -> Option<Vec<String>> {
        p.take("Shuffle");
        p.take("Imprint");
        p.take("Reveal");
        if p.take("ChangeNum").as_deref() != Some("1") {
            return None;
        }
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let valid = p.take("ChangeType")?;
        let expr = self.filter_expr_in_zone(&valid, true)?;
        let filter = self.body.filter_static("REVEAL_SEARCH", &expr);
        Some(vec![format!(
            "Effect::RevealLibrarySearch {{ filter: &{filter}, optional: {optional} }}"
        )])
    }

    fn any_number_library_search(&mut self, p: &mut Params, destination: &str) -> Option<bool> {
        if destination == "Exile"
            && p.peek("ChangeNum") == Some("X")
            && self.svars.get("X").map(String::as_str) == Some("Count$ValidLibrary Artifact.YouOwn")
        {
            p.take("ChangeNum");
            for key in ["RememberChanged", "ForgetOtherRemembered"] {
                if p.take(key).as_deref() != Some("True") {
                    return self.deny(format!("an any-number exile search without `{key}$ True`"));
                }
            }
            return Some(true);
        }
        if matches!(destination, "Hand" | "Battlefield" | "Graveyard")
            && let Some(num_var) = p.peek("ChangeNum").map(str::trim)
            && let Some(def) = self.svars.get(num_var).map(|s| s.trim())
            && let Some(valid) = def.strip_prefix("Count$ValidLibrary ")
            && let Some(stripped) = Self::strip_your_graveyard_owner(valid)
            && let Some(change_type) = p.peek("ChangeType").map(str::trim)
            && (stripped == change_type
                || stripped.strip_prefix("Card.") == Some(change_type)
                || change_type.strip_prefix("Card.") == Some(stripped.as_str()))
        {
            p.take("ChangeNum");
            return Some(true);
        }
        Some(false)
    }

    pub(super) fn exile_self_return_as_face_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        p.take("Defined");
        if p.take("Origin").as_deref() != Some("Battlefield")
            || p.take("Destination").as_deref() != Some("Exile")
            || p.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        let next_name = sub.take()?;
        let next_body = self.svars.get(&next_name)?.clone();
        let (next_api, mut np) = Params::parse(&next_body)?;
        np.drop_prose();
        if next_api != "ChangeZone"
            || !matches!(
                np.take("Defined").as_deref(),
                Some("Remembered" | "RememberedLKI")
            )
            || !matches!(np.take("Origin").as_deref(), Some("Exile" | "All"))
            || np.take("Destination").as_deref() != Some("Battlefield")
        {
            return None;
        }
        if let Some(forget) = np.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        let face = match np.take("Transformed").as_deref() {
            None | Some("False") => 0u8,
            Some("True") => 1u8,
            Some(_) => return None,
        };
        let owner_control = match np.take("GainControl").as_deref() {
            None | Some("False") => true,
            Some("True") => false,
            Some(_) => return None,
        };
        let cleanup_name = np.take("SubAbility");
        if let Some(key) = np.first_key() {
            return self.deny(format!("unclaimed parameter `SelfBlinkReturn.{key}`"));
        }
        if let Some(cname) = cleanup_name {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        Some(vec![format!(
            "Effect::ExileSelfReturnAsFace {{ face: {face}, owner_control: {owner_control} }}"
        )])
    }

    pub(super) fn exile_source_with_counters_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        p.take("Defined");
        if p.take("Origin").as_deref() != Some("Stack")
            || p.take("Destination").as_deref() != Some("Exile")
            || p.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        let next_name = sub.take()?;
        let next_body = self.svars.get(&next_name)?.clone();
        let (next_api, mut np) = Params::parse(&next_body)?;
        np.drop_prose();
        if next_api != "PutCounter"
            || !matches!(
                np.take("Defined").as_deref(),
                Some("Remembered" | "RememberedLKI")
            )
        {
            return None;
        }
        let code = np.take("CounterType")?;
        let Some(kind) = super::counter_kind(&code) else {
            return self.deny(format!("counter `{code}`"));
        };
        let n: u16 = self
            .plain_num(np.take("CounterNum").as_deref().unwrap_or("1"))?
            .try_into()
            .ok()?;
        let cleanup_name = np.take("SubAbility");
        if let Some(key) = np.first_key() {
            return self.deny(format!("unclaimed parameter `ExileCounters.{key}`"));
        }
        if let Some(cname) = cleanup_name {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        Some(vec![format!(
            "Effect::ExileSourceWithCounters {{ kind: {kind}, n: {n} }}"
        )])
    }

    pub(super) fn remembered_token_follow_up(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if target.is_some()
            || targets_a_player
            || p.take("RememberTokens").as_deref() != Some("True")
            || !matches!(p.take("TokenAmount").as_deref(), None | Some("1"))
            || !matches!(p.take("TokenOwner").as_deref(), None | Some("You"))
            || p.has("TokenTapped")
            || p.has("TokenAttacking")
            || p.has("PumpKeywords")
            || p.has("PumpDuration")
        {
            return None;
        }
        let Some(stem) = p.take("TokenScript") else {
            return self.deny("a `Token` effect naming no `TokenScript$`".to_string());
        };
        let Some(tokens) = self.tokens else {
            return self.deny("`Token` with no token scripts to read it against".to_string());
        };
        let Some(body) = tokens.body(&stem, self.cats) else {
            return self.deny(format!("token script `{stem}`"));
        };
        let token = format!("&generated_tokens::{}", body.constant);
        let next_name = sub.take()?;
        let next_body = self.svars.get(&next_name)?.clone();
        let (next_api, mut np) = Params::parse(&next_body)?;
        np.drop_prose();
        if !matches!(
            np.take("Defined").as_deref(),
            Some("Remembered" | "RememberedLKI")
        ) {
            return None;
        }
        let effect = if next_api == "Attach" {
            if self.on_a_spell || !matches!(np.take("Object").as_deref(), None | Some("Self")) {
                return None;
            }
            format!("Effect::CreateTokenThenAttachSelf {{ token: {token} }}")
        } else if next_api == "PutCounter" {
            let code = np.take("CounterType")?;
            let Some(kind) = super::counter_kind(&code) else {
                return self.deny(format!("counter `{code}`"));
            };
            let amount =
                self.amount_or_count_ext(np.take("CounterNum").as_deref().unwrap_or("1"))?;
            format!(
                "Effect::CreateTokenThenCounters {{ token: {token}, kind: {kind}, amount: {amount} }}"
            )
        } else {
            return None;
        };
        let cleanup_name = np.take("SubAbility");
        if let Some(key) = np.first_key() {
            return self.deny(format!("unclaimed parameter `TokenFollowUp.{key}`"));
        }
        if let Some(cname) = cleanup_name {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        Some(vec![effect])
    }

    pub(super) fn is_event_object_defined(&self, def: &str) -> bool {
        let block_role = self.block_line.as_ref().map(|(r, _, _)| *r);
        match def {
            "DelayTriggerRemembered" | "DelayTriggerRememberedLKI" => self.in_delayed,
            "TriggeredNewCardLKICopy"
            | "TriggeredNewCard"
            | "TriggeredCardLKICopy"
            | "TriggeredCard" => {
                matches!(
                    self.trigger_mode.as_deref(),
                    Some("ChangesZone" | "Discarded" | "Cycled")
                )
            }
            "TriggeredAttackerLKICopy" | "TriggeredAttacker" => {
                matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
                ) || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                    && block_role == Some(super::BlockRole::Blocks))
            }
            "TriggeredBlockerLKICopy" | "TriggeredBlocker" => {
                self.trigger_mode.as_deref() == Some("Blocks")
                    || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                        && block_role == Some(super::BlockRole::Blocked))
            }
            "TriggeredTargetLKICopy" | "TriggeredTarget" => {
                matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce")
                )
            }
            _ => false,
        }
    }

    fn change_zone_event_object(
        &mut self,
        p: &mut Params,
        defined: &str,
        origin: &str,
        destination: &str,
    ) -> Option<Vec<String>> {
        p.take("IsCurse");
        match (origin, destination) {
            ("Battlefield", "Hand") => {
                Some(vec!["Effect::bounce(TargetSpec::EventObject)".to_string()])
            }
            ("Battlefield", "Exile") => {
                Some(vec!["Effect::exile(TargetSpec::EventObject)".to_string()])
            }
            ("Graveyard", "Exile")
                if !self.in_delayed
                    && self.trigger_is_self
                    && self.trigger_mode.as_deref() == Some("ChangesZone")
                    && matches!(
                        defined,
                        "TriggeredNewCardLKICopy"
                            | "TriggeredNewCard"
                            | "TriggeredCardLKICopy"
                            | "TriggeredCard"
                    ) =>
            {
                Some(vec!["Effect::ExileSource".to_string()])
            }
            ("Graveyard", "Hand") => Some(vec![
                "Effect::GraveyardToHand { target: TargetSpec::EventObject }".to_string(),
            ]),
            ("Graveyard", "Library") if !p.has("Shuffle") => {
                match p.take("LibraryPosition").as_deref() {
                    None | Some("0") => Some(vec![
                        "Effect::GraveyardToTop { target: TargetSpec::EventObject }".to_string(),
                    ]),
                    Some("-1") => Some(vec![
                    "Effect::PutOnBottomOfLibraryFromGraveyard { target: TargetSpec::EventObject }"
                        .to_string(),
                ]),
                    _ => None,
                }
            }
            ("Graveyard", "Battlefield") => {
                let owner_control = match p.take("GainControl").as_deref() {
                    None | Some("False") => true,
                    Some("True") => false,
                    Some(_) => return None,
                };
                let tapped = match p.take("Tapped").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let counters = self.take_with_counters(p)?;
                if tapped {
                    Some(vec![format!(
                        "Effect::GraveyardToBattlefieldModified {{ target: TargetSpec::EventObject, \
                         owner_control: {owner_control}, tapped: true, counters: {counters}, \
                         modifiers: &[] }}"
                    )])
                } else if !owner_control && counters == "None" {
                    Some(vec![
                        "Effect::reanimate(TargetSpec::EventObject)".to_string(),
                    ])
                } else {
                    Some(vec![format!(
                        "Effect::GraveyardToBattlefield {{ target: TargetSpec::EventObject, \
                         owner_control: {owner_control}, counters: {counters} }}"
                    )])
                }
            }
            _ => None,
        }
    }

    pub(super) fn amount_or_count_ext(&mut self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        let resolved = self
            .svars
            .get(trimmed)
            .map_or(trimmed, String::as_str)
            .trim();
        if let Some(expr) = self.wave6_count_expr(resolved) {
            return Some(expr);
        }
        self.amount_or_count(raw)
    }

    pub(super) fn pump_side_ext(&mut self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        let positive = trimmed.strip_prefix('+').unwrap_or(trimmed).trim();
        let resolved = self
            .svars
            .get(positive)
            .map_or(positive, String::as_str)
            .trim();
        if let Some(expr) = self.wave6_count_expr(resolved) {
            return Some(expr);
        }
        self.pump_side(raw)
    }

    fn wave6_count_expr(&self, def: &str) -> Option<String> {
        if def == "TriggerCount$Result" && self.trigger_mode.as_deref() == Some("RolledDie") {
            return Some("Amount::EventAmount".to_string());
        }
        if matches!(
            def,
            "TriggeredCard$Toughness" | "TriggeredCardLKICopy$Toughness"
        ) && self.trigger_mode.as_deref() == Some("ChangesZone")
        {
            return Some("Amount::EventLastToughness".to_string());
        }
        let (base_raw, op_raw) = def
            .split_once('/')
            .map_or((def, None), |(l, r)| (l.trim(), Some(r.trim())));
        let base = match base_raw {
            "Count$LastDieRoll" => "Amount::LastDieRoll",
            "Count$CoinsWonThisResolution" => "Amount::CoinsWonThisResolution",
            _ => return None,
        };
        let Some(op) = op_raw else {
            return Some(base.to_string());
        };
        if let Some(n) = op
            .strip_prefix("Minus.")
            .and_then(|s| s.parse::<u32>().ok())
        {
            return Some(format!(
                "Amount::SaturatingSub {{ base: &{base}, subtract: {n} }}"
            ));
        }
        if let Some(n) = op.strip_prefix("Plus.").and_then(|s| s.parse::<u32>().ok()) {
            return Some(format!("Amount::Plus {{ base: &{base}, offset: {n} }}"));
        }
        if let Some(n) = op
            .strip_prefix("Times.")
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&v| v >= 1)
        {
            return Some(format!("Amount::Times {{ base: &{base}, factor: {n} }}"));
        }
        None
    }

    pub(super) fn copy_mods(
        &mut self,
        p: &mut Params,
        allow_keep_self_name: bool,
    ) -> Option<Vec<String>> {
        let mut mods = Vec::new();
        let remove_card_types = match p.take("RemoveCardTypes").as_deref() {
            None | Some("False") => false,
            Some("True") if !p.has("RemoveSubTypes") => true,
            Some(_) => return None,
        };
        if let Some(types) = p.take("AddTypes") {
            let mut card_types = Vec::new();
            for word in types
                .split([',', '&'])
                .map(str::trim)
                .filter(|w| !w.is_empty())
            {
                if let Some(ct) = card_type_const(word) {
                    card_types.push(ct);
                } else if let Some(st) = Self::copy_supertype_const(word) {
                    mods.push(format!("CopyMod::AddSupertype({st})"));
                } else {
                    let path = self.cats.const_path(word)?;
                    mods.push(format!("CopyMod::AddSubtype({path})"));
                }
            }
            if remove_card_types {
                let mask = card_types
                    .into_iter()
                    .map(str::to_string)
                    .reduce(|acc, t| format!("{acc}.union({t})"))?;
                mods.push(format!("CopyMod::SetTypes({mask})"));
            } else {
                for ct in card_types {
                    mods.push(format!("CopyMod::AddType({ct})"));
                }
            }
        } else if remove_card_types {
            return None;
        }
        if let Some(types) = p.take("RemoveTypes") {
            for word in types
                .split([',', '&'])
                .map(str::trim)
                .filter(|w| !w.is_empty())
            {
                let ct = card_type_const(word)?;
                mods.push(format!("CopyMod::RemoveType({ct})"));
            }
        }
        if let Some(flag) = p.take("NonLegendary") {
            if flag != "True" {
                return None;
            }
            mods.push("CopyMod::RemoveSupertype(SupertypeSet::LEGENDARY)".to_string());
        }
        if let Some(raw) = p
            .take("RemoveSuperTypes")
            .or_else(|| p.take("RemoveSupertypes"))
        {
            for word in raw
                .split([',', '&'])
                .map(str::trim)
                .filter(|w| !w.is_empty())
            {
                let st = Self::copy_supertype_const(word)?;
                mods.push(format!("CopyMod::RemoveSupertype({st})"));
            }
        }
        if let Some(raw) = p.take("SetCreatureTypes") {
            let mut paths = Vec::new();
            for word in raw
                .split([',', '&'])
                .map(str::trim)
                .filter(|w| !w.is_empty())
            {
                paths.push(self.cats.const_path_of(SubtypeKind::Creature, word)?);
            }
            if paths.is_empty() {
                return None;
            }
            mods.push(format!(
                "CopyMod::SetCreatureTypes(&[{}])",
                paths.join(", ")
            ));
        }
        Self::copy_mods_extra(p, allow_keep_self_name, &mut mods)?;
        Some(mods)
    }

    fn copy_mods_extra(
        p: &mut Params,
        allow_keep_self_name: bool,
        mods: &mut Vec<String>,
    ) -> Option<()> {
        if let Some(raw) = p.take("AddKeywords") {
            for word in raw.split('&').map(str::trim).filter(|w| !w.is_empty()) {
                let kw = keyword_const(word)?;
                mods.push(format!("CopyMod::AddKeyword({kw})"));
            }
        }
        match (p.take("SetPower"), p.take("SetToughness")) {
            (None, None) => {}
            (Some(pow), Some(tou)) => {
                let pow = pow.trim().parse::<i16>().ok()?;
                let tou = tou.trim().parse::<i16>().ok()?;
                mods.push(format!("CopyMod::SetPT({pow}, {tou})"));
            }
            _ => return None,
        }
        if let Some(raw) = p
            .take("SetColor")
            .or_else(|| p.take("SetColors"))
            .or_else(|| p.take("Colors"))
        {
            mods.push(format!(
                "CopyMod::SetColor({})",
                Self::parse_copy_colors(&raw)?
            ));
        }
        if let Some(raw) = p.take("AddColors") {
            mods.push(format!(
                "CopyMod::AddColor({})",
                Self::parse_copy_colors(&raw)?
            ));
        }
        if let Some(name) = p.take("NewName") {
            let name = name.trim();
            if name == "CARDNAME" && allow_keep_self_name {
                mods.push("CopyMod::KeepName".to_string());
            } else if !name.is_empty() && !name.contains("CARDNAME") && !name.contains("NICKNAME") {
                mods.push(format!("CopyMod::SetName({name:?})"));
            } else {
                return None;
            }
        }
        if let Some(flag) = p.take("KeepName") {
            if flag != "True" {
                return None;
            }
            mods.push("CopyMod::KeepName".to_string());
        }
        if let Some(flag) = p.take("NoManaCost").or_else(|| p.take("RemoveManaCost")) {
            if flag != "True" {
                return None;
            }
            mods.push("CopyMod::NoManaCost".to_string());
        }
        if let Some(flag) = p.take("KeepManaCost") {
            if flag != "True" {
                return None;
            }
            mods.push("CopyMod::KeepManaCost".to_string());
        }
        if let Some(raw) = p.take("SetManaCost") {
            let mana = Self::format_mana_cost_braced(&raw)?;
            mods.push(format!("CopyMod::SetManaCost(mana!(\"{mana}\"))"));
        }
        if let Some(flag) = p.take("GainThisAbility") {
            if flag != "True" {
                return None;
            }
            mods.push("CopyMod::KeepResolvingAbility".to_string());
        }
        Some(())
    }

    fn copy_supertype_const(word: &str) -> Option<&'static str> {
        Some(match word {
            "Legendary" => "SupertypeSet::LEGENDARY",
            "Basic" => "SupertypeSet::BASIC",
            "Snow" => "SupertypeSet::SNOW",
            _ => return None,
        })
    }

    fn parse_copy_colors(raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("Colorless") {
            return Some("ColorSet::EMPTY".to_string());
        }
        let mut colors = Vec::new();
        for part in trimmed
            .split([',', '&'])
            .map(str::trim)
            .filter(|w| !w.is_empty())
        {
            let name = color_word(part)?.strip_prefix("ManaColor::")?;
            colors.push(format!("Color::{name}"));
        }
        (!colors.is_empty()).then(|| format!("ColorSet::from_slice(&[{}])", colors.join(", ")))
    }

    fn format_mana_cost_braced(raw: &str) -> Option<String> {
        let mut mana = String::new();
        for tok in raw.split_whitespace() {
            if tok.starts_with('{') && tok.ends_with('}') {
                mana.push_str(tok);
            } else {
                mana.push('{');
                mana.push_str(tok);
                mana.push('}');
            }
        }
        if mana.is_empty() || baylee_core::mana::ManaCost::try_parse(&mana).is_err() {
            return None;
        }
        Some(mana)
    }

    fn take_copy_permanent_riders(p: &mut Params) -> Option<(bool, bool, bool, &'static str)> {
        let tapped = match p
            .take("TokenTapped")
            .or_else(|| p.take("Tapped"))
            .as_deref()
        {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let attacking = match p.take("TokenAttacking").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let haste_until_eot = match (
            p.take("PumpKeywords").as_deref(),
            p.take("PumpDuration").as_deref(),
        ) {
            (None, None) => false,
            (Some("Haste"), None | Some("EOT" | "UntilEndOfTurn")) => true,
            _ => return None,
        };
        let cleanup = match p.take("AtEOT").or_else(|| p.take("AtEOTTrig")).as_deref() {
            None => "CopyCleanup::None",
            Some("Sacrifice") => "CopyCleanup::SacrificeAtNextEndStep",
            Some("Exile") => "CopyCleanup::ExileAtNextEndStep",
            Some("SacrificeCombat") => "CopyCleanup::SacrificeAtEndOfCombat",
            Some("ExileCombat") => "CopyCleanup::ExileAtEndOfCombat",
            Some(_) => return None,
        };
        Some((tapped, attacking, haste_until_eot, cleanup))
    }

    pub(super) fn copy_permanent_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        p.take("AITgts");
        let kicked_copies = p.peek("NumCopies").and_then(|raw| {
            if !self.has_single_kicker {
                return None;
            }
            let def = self.svars.get(raw.trim())?.trim();
            let (k_str, u_str) = def.strip_prefix("Count$Kicked.")?.split_once('.')?;
            let k = k_str.trim().parse::<u32>().ok()?;
            let u = u_str.trim().parse::<u32>().ok()?;
            Some((format!("Amount::Fixed({k})"), format!("Amount::Fixed({u})")))
        });
        let amount = if kicked_copies.is_some() {
            p.take("NumCopies");
            String::new()
        } else {
            match p.take("NumCopies") {
                None => "Amount::Fixed(1)".to_string(),
                Some(raw) => self.amount_or_count_ext(&raw)?,
            }
        };
        let (tapped, attacking, haste_until_eot, cleanup) = Self::take_copy_permanent_riders(p)?;
        let mods = self.copy_mods(p, false)?;
        let mods_list = mods.join(", ");
        let populate = match p.take("Populate").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let wrap = |mk: &dyn Fn(&str) -> String| -> Vec<String> {
            match &kicked_copies {
                Some((k, u)) => vec![format!(
                    "Effect::IfKicked {{ then: &[{}], otherwise: &[{}] }}",
                    mk(k),
                    mk(u)
                )],
                None => vec![mk(&amount)],
            }
        };
        let choose_filter = if populate {
            if target.is_some()
                || targets_here
                || p.has("Defined")
                || p.has("Choices")
                || !matches!(p.take("Controller").as_deref(), None | Some("You"))
            {
                return None;
            }
            Some(self.body.filter_static(
                "POPULATE",
                "Filter::And(&[Filter::CREATURE, Filter::IsToken, Filter::ControlledByYou])",
            ))
        } else if let Some(choices) = p.take("Choices") {
            if target.is_some()
                || targets_here
                || p.has("Defined")
                || p.has("RandomCopied")
                || !matches!(p.take("Chooser").as_deref(), None | Some("You"))
                || !matches!(p.take("Controller").as_deref(), None | Some("You"))
            {
                return None;
            }
            let expr = self.filter_expr(&choices)?;
            Some(self.body.filter_static("COPY_CHOICE", &expr))
        } else {
            None
        };
        if let Some(filter) = choose_filter {
            return Some(wrap(&|amt| {
                format!(
                    "Effect::ChooseAndCreateTokenCopy {{ filter: &{filter}, amount: {amt}, \
                     mods: &[{mods_list}], tapped: {tapped}, attacking: {attacking}, \
                     haste_until_eot: {haste_until_eot}, cleanup: {cleanup} }}"
                )
            }));
        }
        let controller =
            self.player_of_line(p.take("Controller").as_deref(), target, targets_a_player)?;
        let obj_target = target.filter(|t| {
            t.starts_with("TargetSpec::Object(") || t.starts_with("TargetSpec::CardInGraveyard(")
        });
        let copy_target = match p.take("Defined").as_deref() {
            None if targets_here => obj_target?.to_string(),
            Some("Targeted" | "ParentTarget") => obj_target?.to_string(),
            Some("Self") if !self.on_a_spell && target.is_none() => {
                "TargetSpec::ThisObject".to_string()
            }
            Some(def) if target.is_none() && self.is_event_object_defined(def) => {
                "TargetSpec::EventObject".to_string()
            }
            _ => return None,
        };
        Some(wrap(&|amt| {
            format!(
                "Effect::CreateTokenCopy {{ target: {copy_target}, controller: {controller}, \
                 amount: {amt}, mods: &[{mods_list}], tapped: {tapped}, attacking: {attacking}, \
                 haste_until_eot: {haste_until_eot}, cleanup: {cleanup} }}"
            )
        }))
    }

    pub(super) fn become_copy_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        second_target_here: bool,
    ) -> Option<Vec<String>> {
        if !matches!(p.take("CloneZone").as_deref(), None | Some("Battlefield")) {
            return None;
        }
        let until_eot = match p.take("Duration").as_deref() {
            None | Some("Permanent") => false,
            Some("UntilEndOfTurn") => true,
            Some(_) => return None,
        };
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let mods = self.copy_mods(p, true)?;
        if let Some(extra_svars) = p.peek("AddSVars")
            && mods.iter().any(|m| m == "CopyMod::KeepResolvingAbility")
            && self
                .svars
                .get(extra_svars.trim())
                .and_then(|body| Params::parse(body))
                .is_some_and(|(api, _)| api == "Clone")
        {
            p.take("AddSVars");
        }
        let obj_target = target.filter(|t| {
            t.starts_with("TargetSpec::Object(") || t.starts_with("TargetSpec::CardInGraveyard(")
        });
        let clone_target = p.take("CloneTarget");
        let defined = p.take("Defined");
        let (subject, subject_filter, original, original_is_second) = if second_target_here {
            if clone_target.as_deref() != Some("ParentTarget") || defined.is_some() {
                return None;
            }
            (
                "TargetSlot::First",
                "None".to_string(),
                "TargetSpec::ThisObject".to_string(),
                true,
            )
        } else if let Some(valid) = clone_target
            .as_deref()
            .and_then(|s| s.strip_prefix("Valid "))
        {
            let orig = match defined.as_deref() {
                None if targets_here => obj_target?.to_string(),
                Some("Targeted" | "ParentTarget") => obj_target?.to_string(),
                Some("Self") if !self.on_a_spell && target.is_none() => {
                    "TargetSpec::ThisObject".to_string()
                }
                _ => return None,
            };
            let expr = self.filter_expr(valid.trim())?;
            let filter = self.body.filter_static("COPY_SUBJ", &expr);
            ("TargetSlot::This", format!("Some(&{filter})"), orig, false)
        } else if clone_target.as_deref() == Some("Targeted")
            && defined.as_deref() == Some("Self")
            && !self.on_a_spell
            && obj_target.is_some()
        {
            (
                "TargetSlot::First",
                "None".to_string(),
                "TargetSpec::ThisObject".to_string(),
                false,
            )
        } else if matches!(clone_target.as_deref(), None | Some("Self")) && !self.on_a_spell {
            let orig = match defined.as_deref() {
                None if targets_here => obj_target?.to_string(),
                Some("Targeted" | "ParentTarget") => obj_target?.to_string(),
                Some(def) if target.is_none() && self.is_event_object_defined(def) => {
                    "TargetSpec::EventObject".to_string()
                }
                _ => return None,
            };
            ("TargetSlot::This", "None".to_string(), orig, false)
        } else {
            return None;
        };
        let effect = format!(
            "Effect::BecomeCopy {{ subject: {subject}, subject_filter: {subject_filter}, \
             original: {original}, original_is_second_target: {original_is_second}, \
             until_eot: {until_eot}, mods: &[{}] }}",
            mods.join(", ")
        );
        Some(vec![if optional {
            format!("Effect::MayDo {{ effects: &[{effect}] }}")
        } else {
            effect
        }])
    }

    pub(super) fn flip_coin_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if p.has("NoCall")
            || p.has("HeadsSubAbility")
            || p.has("TailsSubAbility")
            || !matches!(p.take("Flipper").as_deref(), None | Some("You"))
        {
            return None;
        }
        let until_lose = match p.take("FlipUntilYouLose").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let win = match p.take("WinSubAbility") {
            None => Vec::new(),
            Some(name) => self.parse_branch_svar(&name, chain.target.as_deref(), None)?,
        };
        let lose = match p.take("LoseSubAbility") {
            None => Vec::new(),
            Some(name) => self.parse_branch_svar(&name, chain.target.as_deref(), None)?,
        };
        if let Some(sub_name) = sub.as_deref()
            && self.is_dummy_zero_mana_svar(sub_name)
        {
            *sub = None;
        }
        let rem_wins = match p.take("RememberNumber").as_deref() {
            None => false,
            Some("Wins") => true,
            Some(_) => return None,
        };
        let wins_follow = if rem_wins {
            let sub_name = sub.take()?;
            self.parse_branch_svar(
                &sub_name,
                chain.target.as_deref(),
                Some(("Wins", "Count$CoinsWonThisResolution")),
            )?
        } else {
            Vec::new()
        };
        if until_lose {
            if p.has("Amount") {
                return None;
            }
            let mut after = lose;
            after.extend(wins_follow);
            return Some(vec![format!(
                "Effect::FlipUntilLose {{ max_flips: None, per_win: &[{}], after: &[{}] }}",
                win.join(", "),
                after.join(", ")
            )]);
        }
        let raw_amount = p.take("Amount");
        if let Some(raw_amt) = raw_amount
            && raw_amt.trim() != "1"
        {
            if !lose.is_empty() {
                return None;
            }
            let count = self.amount_or_count_ext(&raw_amt)?;
            let mut out = vec![format!(
                "Effect::FlipCoins {{ count: {count}, stop_on_loss: false, \
                 per_win: &[{}], if_won_all: &[] }}",
                win.join(", ")
            )];
            out.extend(wins_follow);
            return Some(out);
        }
        if rem_wins {
            return None;
        }
        if win.is_empty() && lose.is_empty() {
            return Some(vec![
                "Effect::FlipCoins { count: Amount::Fixed(1), stop_on_loss: false, \
                 per_win: &[], if_won_all: &[] }"
                    .to_string(),
            ]);
        }
        Some(vec![format!(
            "Effect::FlipCoin {{ won: &[{}], lost: &[{}] }}",
            win.join(", "),
            lose.join(", ")
        )])
    }

    pub(super) fn roll_dice_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if !matches!(p.take("Defined").as_deref(), None | Some("You")) {
            return None;
        }
        let sides = match p.take("Sides") {
            None => 6u8,
            Some(raw) => raw.trim().parse::<u8>().ok().filter(|&n| n >= 2)?,
        };
        let count = match p.take("Amount") {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => self.amount_or_count_ext(&raw)?,
        };
        let modifier = match p.take("Modifier") {
            None => 0i8,
            Some(raw) => raw.trim().parse::<i8>().ok()?,
        };
        let result_svar = p.take("ResultSVar");
        let table = match p.take("ResultSubAbilities") {
            None => None,
            Some(raw) => Some(self.parse_die_result_table(&raw, chain.target.as_deref())?),
        };
        if let Some(sub_name) = sub.as_deref()
            && self.is_dummy_zero_mana_svar(sub_name)
        {
            *sub = None;
        }
        let mut out = vec![format!(
            "Effect::RollDice {{ count: {count}, sides: {sides}, modifier: {modifier} }}"
        )];
        if let Some(branch_effect) = table {
            out.push(branch_effect);
        }
        if let Some(var) = result_svar {
            let sub_name = sub.take()?;
            let body = self.svars.get(&sub_name)?.clone();
            let rewritten = self.rewrite_svar_placeholder(&body, var.trim(), "Count$LastDieRoll");
            let mut sub_chain = Chain {
                target: chain.target.clone(),
                ..Chain::default()
            };
            self.chain(&rewritten, &mut sub_chain)?;
            if sub_chain.target != chain.target {
                if chain.target.is_some() || sub_chain.second_target.is_some() {
                    return None;
                }
                chain.target = sub_chain.target;
                chain.target_up_to = sub_chain.target_up_to;
                chain.target_exact = sub_chain.target_exact;
                chain.target_is_x = sub_chain.target_is_x;
            }
            if sub_chain.effects.is_empty() {
                return None;
            }
            out.extend(sub_chain.effects);
        }
        Some(out)
    }

    fn parse_branch_svar(
        &mut self,
        name: &str,
        parent_target: Option<&str>,
        replace_var: Option<(&str, &str)>,
    ) -> Option<Vec<String>> {
        let body = self.svars.get(name.trim())?.clone();
        let body = match replace_var {
            Some((from, to)) => self.rewrite_svar_placeholder(&body, from, to),
            None => body,
        };
        let mut branch = Chain {
            target: parent_target.map(str::to_string),
            ..Chain::default()
        };
        self.chain(&body, &mut branch)?;
        if branch.target.as_deref() != parent_target
            || branch.second_target.is_some()
            || branch.effects.is_empty()
        {
            return None;
        }
        Some(branch.effects)
    }

    fn parse_die_result_table(&mut self, raw: &str, parent_target: Option<&str>) -> Option<String> {
        let mut ranges: Vec<(u8, u8, Vec<String>)> = Vec::new();
        let mut else_branch: Option<Vec<String>> = None;
        let parts: Vec<&str> = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            return None;
        }
        for (idx, part) in parts.iter().enumerate() {
            let (range_str, svar_name) = part.split_once(':')?;
            let range_str = range_str.trim();
            let effects = self.parse_branch_svar(svar_name, parent_target, None)?;
            if range_str == "Else" {
                if idx + 1 != parts.len() {
                    return None;
                }
                else_branch = Some(effects);
            } else if let Some(lo_str) = range_str.strip_suffix('+') {
                let lo = lo_str.trim().parse::<u8>().ok()?;
                ranges.push((lo, u8::MAX, effects));
            } else if let Some((lo_s, hi_s)) = range_str.split_once('-') {
                let lo = lo_s.trim().parse::<u8>().ok()?;
                let hi = hi_s.trim().parse::<u8>().ok()?;
                if lo > hi {
                    return None;
                }
                ranges.push((lo, hi, effects));
            } else {
                let n = range_str.parse::<u8>().ok()?;
                ranges.push((n, n, effects));
            }
        }
        let mut acc = else_branch.unwrap_or_default();
        for (min, max, then) in ranges.into_iter().rev() {
            let expr = format!(
                "Effect::IfDieRollInRange {{ min: {min}, max: {max}, \
                 then: &[{}], otherwise: &[{}] }}",
                then.join(", "),
                acc.join(", ")
            );
            acc = vec![expr];
        }
        (acc.len() == 1).then(|| acc.remove(0))
    }

    fn rewrite_svar_placeholder(&self, body: &str, var: &str, replacement: &str) -> String {
        let mut out_parts = Vec::new();
        for (idx, part) in body.split(" | ").enumerate() {
            if idx == 0 {
                out_parts.push(part.to_string());
                continue;
            }
            if let Some((k, v)) = part.split_once('$') {
                let val = v.trim();
                let new_val = if val == var {
                    Some(replacement.to_string())
                } else if val == format!("+{var}") {
                    Some(format!("+{replacement}"))
                } else {
                    let bare = val.strip_prefix('+').unwrap_or(val);
                    let plus = val.starts_with('+');
                    self.svars
                        .get(bare)
                        .and_then(|def| {
                            def.trim()
                                .strip_prefix(&format!("SVar${var}/"))
                                .map(|rest| format!("{replacement}/{rest}"))
                        })
                        .map(|expr| if plus { format!("+{expr}") } else { expr })
                };
                if let Some(nv) = new_val {
                    out_parts.push(format!("{}$ {nv}", k.trim()));
                    continue;
                }
            }
            out_parts.push(part.to_string());
        }
        out_parts.join(" | ")
    }

    fn is_dummy_zero_mana_svar(&self, name: &str) -> bool {
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return false;
        };
        p.drop_prose();
        api == "Mana"
            && p.take("Produced").is_some()
            && p.take("Amount").as_deref() == Some("0")
            && p.exhausted()
    }

    pub(super) fn phase_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        p.take("IsCurse");
        if p.has("AnyNumber")
            || p.has("WontPhaseInNormal")
            || p.has("PhaseInOrOut")
            || p.has("RememberAffected")
        {
            return None;
        }
        if let Some(valid) = p.take("AllValid") {
            if target.is_some() || p.has("Defined") {
                return None;
            }
            let expr = self.filter_expr(&valid)?;
            let filter = self.body.filter_static("PHASE", &expr);
            return Some(vec![format!("Effect::PhaseOutFilter(&{filter})")]);
        }
        let obj_target = target.filter(|t| t.starts_with("TargetSpec::Object("));
        if target.is_none() && p.peek("Defined") == Some("Valid Permanent.YouCtrl") {
            return self.phase_your_permanents(p);
        }
        match p.take("Defined").as_deref() {
            Some("Self") if !self.on_a_spell && target.is_none() => {
                Some(vec!["Effect::PhaseOut { target: None }".to_string()])
            }
            Some("Enchanted" | "Equipped") if !self.on_a_spell && target.is_none() => Some(vec![
                "Effect::PhaseOutFilter(&Filter::AttachedToBySource)".to_string(),
            ]),
            Some(def) if target.is_none() && self.is_event_object_defined(def) => Some(vec![
                "Effect::PhaseOut { target: Some(TargetSpec::EventObject) }".to_string(),
            ]),
            Some("Self & Targeted") if !self.on_a_spell => Some(vec![
                "Effect::PhaseOut { target: None }".to_string(),
                format!("Effect::PhaseOut {{ target: Some({}) }}", obj_target?),
            ]),
            None if targets_here => Some(vec![format!(
                "Effect::PhaseOut {{ target: Some({}) }}",
                obj_target?
            )]),
            Some("Targeted" | "ParentTarget") => Some(vec![format!(
                "Effect::PhaseOut {{ target: Some({}) }}",
                obj_target?
            )]),
            _ => None,
        }
    }

    pub(super) fn phase_out_until_source_leaves_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if p.take("WontPhaseInNormal").as_deref() != Some("True")
            || !matches!(
                p.take("ConditionPresent").as_deref(),
                None | Some("Card.Self")
            )
        {
            return None;
        }
        let eff_name = sub.take()?;
        let eff_body = self.svars.get(&eff_name)?.clone();
        let (eff_api, mut ep) = Params::parse(&eff_body)?;
        ep.drop_prose();
        if eff_api != "Effect"
            || ep.take("RememberObjects").as_deref() != Some("Targeted")
            || ep.take("ImprintCards").as_deref() != Some("Self")
            || ep.take("Duration").as_deref() != Some("Permanent")
            || ep.take("ForgetOnPhasedIn").as_deref() != Some("True")
            || !matches!(
                ep.take("ConditionPresent").as_deref(),
                None | Some("Card.Self")
            )
        {
            return None;
        }
        let trig_name = ep.take("Triggers")?;
        let next_sub = ep.take("SubAbility");
        if !ep.exhausted() {
            return None;
        }
        let trig_body = self.svars.get(trig_name.trim())?.clone();
        let (trig_mode, mut tp) = Params::parse(&trig_body)?;
        tp.drop_prose();
        if trig_mode != "ChangesZone"
            || tp.take("Origin").as_deref() != Some("Battlefield")
            || tp.take("Destination").as_deref() != Some("Any")
            || tp.take("ValidCard").as_deref() != Some("Card.IsImprinted")
            || !matches!(tp.take("TriggerZones").as_deref(), None | Some("Command"))
            || !matches!(
                tp.take("TriggerController").as_deref(),
                None | Some("TriggeredCardController")
            )
            || !matches!(tp.take("Static").as_deref(), None | Some("True"))
        {
            return None;
        }
        let exec_name = tp.take("Execute")?;
        if !tp.exhausted() {
            return None;
        }
        let exec_body = self.svars.get(exec_name.trim())?.clone();
        let (exec_api, mut xp) = Params::parse(&exec_body)?;
        xp.drop_prose();
        if exec_api != "Phases"
            || xp.take("Defined").as_deref() != Some("Remembered")
            || xp.take("PhaseInOrOut").as_deref() != Some("True")
        {
            return None;
        }
        let tap_on_phase_in = match xp.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if let Some(exile_name) = xp.take("SubAbility") {
            let eb = self.svars.get(exile_name.trim())?.clone();
            let (eapi, mut xep) = Params::parse(&eb)?;
            xep.drop_prose();
            if eapi != "ChangeZone"
                || xep.take("Origin").as_deref() != Some("Command")
                || xep.take("Destination").as_deref() != Some("Exile")
                || xep.take("Defined").as_deref() != Some("Self")
                || !xep.exhausted()
            {
                return None;
            }
        }
        if !xp.exhausted() {
            return None;
        }
        *sub = next_sub;
        Some(vec![format!(
            "Effect::PhaseOutUntilSourceLeaves {{ tap_on_phase_in: {tap_on_phase_in} }}"
        )])
    }

    /// `DB$/AB$/SP$ Amass` (CR 701.47a): amass `[subtype] N`.
    pub(super) fn amass_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let who = self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if who != "PlayerRel::You" {
            return self.deny("`Amass` for somebody other than you".to_string());
        }
        let raw_type = p.take("Type")?;
        let subtype = self
            .cats
            .const_path_of(SubtypeKind::Creature, raw_type.trim())?;
        let stem = format!("b_0_0_{}_army", raw_type.trim().to_ascii_lowercase());
        let Some(tokens) = self.tokens else {
            return self.deny("`Amass` with no token scripts to read it against".to_string());
        };
        let Some(body) = tokens.body(&stem, self.cats) else {
            return self.deny(format!("token script `{stem}`"));
        };
        let raw_num = p.take("Num")?;
        let amount = self.amount_or_count_ext(&raw_num)?;
        Some(vec![format!(
            "Effect::Amass {{ token: &generated_tokens::{}, subtype: {subtype}, amount: {amount} }}",
            body.constant
        )])
    }

    /// `DB$/AB$/SP$ BecomeMonarch` (CR 724.1): one player becomes the monarch.
    pub(super) fn become_monarch_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let who = self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(who) {
            return None;
        }
        Some(vec![format!("Effect::BecomeMonarch({who})")])
    }

    fn is_single_player_rel(who: &str) -> bool {
        matches!(
            who,
            "PlayerRel::You"
                | "PlayerRel::Chosen"
                | "PlayerRel::ControllerOfTarget"
                | "PlayerRel::ControllerOfEvent"
                | "PlayerRel::TargetPlayerOrController"
                | "PlayerRel::EventPlayer"
                | "PlayerRel::DefendingPlayer"
                | "PlayerRel::DamagedPlayer"
                | "PlayerRel::ActivePlayer"
                | "PlayerRel::OwnerOfTarget"
                | "PlayerRel::ControllerOfAttached"
                | "PlayerRel::OwnerOfSource"
        )
    }

    fn is_untap_leaves_lose_control(raw: &str) -> bool {
        let mut parts: Vec<&str> = raw.split(',').map(str::trim).collect();
        parts.sort_unstable();
        parts == ["LeavesPlay", "LoseControl", "Untap"]
    }

    /// `DB$/AB$/SP$ AlterAttribute | Attributes$ Prepared|Saddled` (CR 723.3 / 702.171).
    pub(super) fn alter_attribute_effect(
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let attr = p.take("Attributes")?;
        if let Some(activate) = p.take("Activate")
            && activate != "True"
        {
            return None;
        }
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let defined = p.take("Defined");
        let object_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        let effect = match (attr.as_str(), targets_here, defined.as_deref()) {
            ("Prepared", true, None | Some("Targeted")) | ("Prepared", false, Some("Targeted"))
                if single_target && object_target =>
            {
                "Effect::BecomeTargetPrepared"
            }
            ("Prepared", false, None | Some("Self")) => "Effect::BecomePrepared",
            ("Saddled", false, None | Some("Self")) => "Effect::BecomeSaddled",
            _ => return None,
        };
        Some(vec![if optional {
            format!("Effect::MayDo {{ effects: &[{effect}] }}")
        } else {
            effect.to_string()
        }])
    }

    /// `ChangeZone` from your graveyard to your hand at random.
    fn random_graveyard_to_hand(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if p.take("Hidden").as_deref() != Some("True")
            || p.take("AtRandom").as_deref() != Some("True")
        {
            return None;
        }
        if let Some(mand) = p.take("Mandatory")
            && mand != "True"
        {
            return None;
        }
        if !matches!(p.take("ChangeNum").as_deref(), None | Some("1")) {
            return None;
        }
        p.take("SelectPrompt");
        let valid = p.take("ChangeType")?;
        let stripped = Self::strip_your_graveyard_owner(&valid)?;
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let filter = self.body.filter_static("RANDOM_GRAVE", &expr);
        Some(vec![format!(
            "Effect::RandomGraveyardToHand {{ filter: &{filter} }}"
        )])
    }

    /// Strips required `IsRemembered` (and optional `YouOwn`/`YouCtrl`) from
    /// every comma-separated alternative in a post-mill `ChangeType$`.
    fn strip_remembered_filter(valid: &str) -> Option<String> {
        let mut out = Vec::new();
        for alt in valid.split(',') {
            let mut parts = alt
                .split('.')
                .flat_map(|s| s.split('+'))
                .map(str::trim)
                .filter(|a| !a.is_empty());
            let base = parts.next()?;
            let mut has_remembered = false;
            let mut kept = Vec::new();
            for atom in parts {
                match atom {
                    "IsRemembered" => has_remembered = true,
                    "YouOwn" | "YouCtrl" => {}
                    other if other.contains("Ctrl") || other.contains("Own") => return None,
                    other => kept.push(other),
                }
            }
            if !has_remembered {
                return None;
            }
            if kept.is_empty() {
                out.push(base.to_string());
            } else {
                out.push(format!("{base}.{}", kept.join("+")));
            }
        }
        (!out.is_empty()).then(|| out.join(","))
    }

    /// `Mill | RememberMilled$ True` followed by `ChangeZone` taking up to one
    /// matching milled card into your hand (`Effect::MillMayTakeOne`, CR 701.17c).
    pub(super) fn mill_may_take_one_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        if !matches!(p.take("Defined").as_deref(), None | Some("You"))
            || p.take("RememberMilled").as_deref() != Some("True")
        {
            return None;
        }
        let amount = self.amount_or_count_ext(&p.take("NumCards")?)?;
        let next_name = sub.take()?;
        let next_body = self.svars.get(&next_name)?.clone();
        let (next_api, mut dp) = Params::parse(&next_body)?;
        dp.drop_prose();
        if next_api != "ChangeZone"
            || !matches!(
                dp.take("Origin").as_deref(),
                Some("Graveyard,Exile" | "Exile,Graveyard" | "Graveyard")
            )
            || dp.take("Destination").as_deref() != Some("Hand")
            || !matches!(dp.take("Hidden").as_deref(), None | Some("True"))
            || !matches!(dp.take("ChangeNum").as_deref(), None | Some("1"))
        {
            return None;
        }
        if dp.has("Mandatory") {
            return None;
        }
        dp.claim_label("ChangeTypeDesc", "ChangeType");
        let prompt_may = dp
            .take("SelectPrompt")
            .is_some_and(|s| s.to_ascii_lowercase().starts_with("you may"));
        let optional = match dp.take("Optional").as_deref() {
            Some("True") => true,
            None => prompt_may || self.oracle.to_ascii_lowercase().contains("you may put "),
            Some(_) => return None,
        };
        if !optional {
            return None;
        }
        let valid = dp.take("ChangeType")?;
        let stripped = Self::strip_remembered_filter(&valid)?;
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let filter = self.body.filter_static("MILL_TAKE", &expr);
        let follow_up = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }
        if let Some(fname) = follow_up {
            if self.sub_is_api(Some(&fname), "Cleanup") {
                self.consume_optional_remember_cleanup(fname, sub)?;
            } else {
                *sub = Some(fname);
            }
        }
        Some(vec![format!(
            "Effect::MillMayTakeOne {{ amount: {amount}, filter: &{filter} }}"
        )])
    }

    /// Classifies a follow-up step's condition on `Remembered` after a
    /// `Sacrifice | RememberSacrificed$ True` line: `Some((true, check))` for
    /// "if you did / if a [check] was sacrificed", `Some((false, None))` for
    /// "if you didn't".
    fn take_remembered_sac_cond(&self, sp: &mut Params) -> Option<(bool, Option<String>)> {
        if sp.peek("ConditionDefined") == Some("Remembered") {
            let present = sp.take("ConditionPresent")?;
            let cmp = sp.take("ConditionCompare");
            sp.take("ConditionDefined");
            sp.take("ConditionDescription");
            return match cmp.as_deref() {
                None | Some("GE1" | "EQ1") => {
                    let check = (present != "Card" && present != "Permanent").then_some(present);
                    Some((true, check))
                }
                Some("EQ0" | "LT1") => Some((false, None)),
                Some(_) => None,
            };
        }
        if let Some(var) = sp.peek("ConditionCheckSVar")
            && self.svars.get(var.trim()).map(|s| s.trim()) == Some("Remembered$Amount")
        {
            let cmp = sp.take("ConditionSVarCompare");
            sp.take("ConditionCheckSVar");
            sp.take("ConditionDescription");
            return match cmp.as_deref() {
                None | Some("GE1" | "EQ1") => Some((true, None)),
                Some("EQ0" | "LT1") => Some((false, None)),
                Some(_) => None,
            };
        }
        None
    }

    /// Reads a chain of follow-up `DB$` lines all sharing the same
    /// `Remembered` sacrifice polarity (`on_sac`: true = paid, false = unpaid),
    /// stopping at an optional `Cleanup | ClearRemembered$ True`.
    fn collect_remembered_sac_body(
        &mut self,
        first_sub: String,
        target: Option<&str>,
        sub_out: &mut Option<String>,
    ) -> Option<(bool, Option<String>, Vec<String>)> {
        let mut cursor = Some(first_sub);
        let mut polarity: Option<bool> = None;
        let mut check_valid: Option<String> = None;
        let mut body_effects = Vec::new();
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 8 {
                return None;
            }
            let body = self.svars.get(name.trim())?.clone();
            let (api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if api == "Cleanup" && sp.take("ClearRemembered").as_deref() == Some("True") {
                let tail = sp.take("SubAbility");
                if !sp.exhausted() {
                    return None;
                }
                *sub_out = tail;
                break;
            }
            let (on_sac, check) = self.take_remembered_sac_cond(&mut sp)?;
            if let Some(prev) = polarity {
                if prev != on_sac || check_valid != check {
                    return None;
                }
            } else {
                polarity = Some(on_sac);
                check_valid = check;
            }
            if sp.has("ValidTgts") || sp.has("UnlessCost") || sp.has("AtEOT") {
                return None;
            }
            let next = sp.take("SubAbility");
            let effs = self.effect_of(&api, &mut sp, target, false, false)?;
            if !sp.exhausted() || effs.is_empty() {
                return None;
            }
            body_effects.extend(effs);
            cursor = next;
        }
        let on_sac = polarity?;
        (!body_effects.is_empty()).then_some((on_sac, check_valid, body_effects))
    }

    /// `Sacrifice | RememberSacrificed$ True` followed by `ConditionDefined$
    /// Remembered` (`PlayerMayPayCostThen`, `PlayerMayPayCostOr`,
    /// `SacrificeAndIf`, or `SacrificeSelfThen`).
    pub(super) fn sacrifice_remembered_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.take("RememberSacrificed").as_deref() != Some("True")
            || !matches!(p.take("Amount").as_deref(), None | Some("1"))
            || !matches!(p.take("ChangeNum").as_deref(), None | Some("1"))
        {
            return None;
        }
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        p.claim_label("SacMessage", "SacValid");
        let sac_valid = p.take("SacValid");
        let who = self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if !p.exhausted() {
            return None;
        }
        let first_sub = sub.take()?;
        let (on_sac, check_valid, body_effects) =
            self.collect_remembered_sac_body(first_sub, target, sub)?;
        let body_one = match body_effects.as_slice() {
            [one] => one.clone(),
            many => format!("Effect::Sequence(&[{}])", many.join(", ")),
        };
        if sac_valid.as_deref().is_none_or(|v| v == "Self") {
            if !optional
                && on_sac
                && check_valid.is_none()
                && who == "PlayerRel::You"
                && !self.on_a_spell
            {
                return Some(vec![format!(
                    "Effect::SacrificeSelfThen {{ then: &[{}] }}",
                    body_effects.join(", ")
                )]);
            }
            return None;
        }
        let valid = sac_valid?;
        let sac_filter = if valid == "Permanent" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            format!("&{}", self.body.filter_static("SAC", &expr))
        };
        if optional {
            if check_valid.as_deref().is_some_and(|c| c != valid) {
                return None;
            }
            if !matches!(
                who,
                "PlayerRel::You"
                    | "PlayerRel::Chosen"
                    | "PlayerRel::ControllerOfTarget"
                    | "PlayerRel::ControllerOfEvent"
                    | "PlayerRel::TargetPlayerOrController"
                    | "PlayerRel::EventPlayer"
                    | "PlayerRel::DefendingPlayer"
                    | "PlayerRel::ActivePlayer"
                    | "PlayerRel::OwnerOfTarget"
                    | "PlayerRel::ControllerOfAttached"
            ) {
                return None;
            }
            let variant = if on_sac {
                "PlayerMayPayCostThen"
            } else {
                "PlayerMayPayCostOr"
            };
            return Some(vec![format!(
                "Effect::{variant} {{ player: {who}, cost: &CostPart::Sacrifice({sac_filter}), \
                 effect: &{body_one} }}"
            )]);
        }
        if who == "PlayerRel::You" {
            if on_sac {
                let check_filter = match check_valid {
                    None => "&Filter::Any".to_string(),
                    Some(ref cv) => {
                        let expr = self.filter_expr(cv)?;
                        format!("&{}", self.body.filter_static("SAC_CHECK", &expr))
                    }
                };
                return Some(vec![format!(
                    "Effect::SacrificeAndIf {{ filter: {sac_filter}, check: {check_filter}, \
                     effect: &{body_one} }}"
                )]);
            }
            if check_valid.is_none() {
                return Some(vec![format!(
                    "Effect::IfCondition {{ condition: Condition::CanSacrifice({sac_filter}), \
                     then: &[Effect::SacrificeFilter {{ who: PlayerRel::You, filter: {sac_filter} }}], \
                     otherwise: &[{}] }}",
                    body_effects.join(", ")
                )]);
            }
        }
        None
    }

    /// Wave-7 fallback for `DigUntil`: handles `NoPutDesc$` prose and
    /// `RevealedDestination$ Library | RevealedLibraryPosition$ -1 | Shuffle$ True`
    /// (putting the rest into the library and immediately shuffling).
    pub(super) fn wave7_dig_until_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let player = self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if !matches!(p.take("Amount").as_deref(), None | Some("1")) {
            return None;
        }
        p.take("IsCurse");
        p.take("NoPutDesc");
        p.claim_label("ValidDescription", "Valid");
        let valid = p.take("Valid")?;
        let found_raw = p.take("FoundDestination");
        let revealed_raw = p.take("RevealedDestination");
        let pos = p.take("RevealedLibraryPosition");
        let shuffle = match p.take("Shuffle").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let random_order = match p.take("RevealRandomOrder").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if !p.entries.iter().all(|(k, _)| k.starts_with("Condition")) {
            return None;
        }
        let expr = self.filter_expr_in_zone(&valid, true)?;
        let name = self.body.filter_static("REVEAL_UNTIL", &expr);
        // Bottom + shuffle is identical to top + shuffle because the library is
        // shuffled immediately afterwards.
        if shuffle && revealed_raw.as_deref() == Some("Library") && pos.as_deref() == Some("-1") {
            let matched = match found_raw.as_deref() {
                Some("Battlefield") => "SearchDest::Battlefield",
                Some("Hand") => "SearchDest::Hand",
                Some("Graveyard") => "SearchDest::Graveyard",
                Some("Exile") => "SearchDest::Exile",
                Some("Library") => "SearchDest::TopOfLibrary",
                _ => return None,
            };
            return Some(vec![
                format!(
                    "Effect::RevealUntilMatching {{ player: {player}, filter: &{name}, \
                     matched: {matched}, otherwise: SearchDest::TopOfLibrary }}"
                ),
                format!("Effect::ShuffleLibrary {{ who: {player} }}"),
            ]);
        }
        if player == "PlayerRel::You"
            && !shuffle
            && random_order
            && revealed_raw.as_deref() == Some("Library")
            && pos.as_deref() == Some("-1")
        {
            let found = match found_raw.as_deref() {
                Some("Hand") => "SearchDest::Hand",
                Some("Battlefield") => "SearchDest::Battlefield",
                Some("Exile") => "SearchDest::Exile",
                Some("Graveyard") => "SearchDest::Graveyard",
                _ => return None,
            };
            return Some(vec![format!(
                "Effect::RevealUntil {{ filter: &{name}, found: {found} }}"
            )]);
        }
        if !shuffle && !random_order && pos.is_none() {
            let otherwise = match revealed_raw.as_deref() {
                Some("Library") => "SearchDest::TopOfLibrary",
                Some("Graveyard") => "SearchDest::Graveyard",
                Some("Exile") => "SearchDest::Exile",
                _ => return None,
            };
            let matched = match found_raw.as_deref() {
                Some("Battlefield") => "SearchDest::Battlefield",
                Some("Hand") => "SearchDest::Hand",
                Some("Graveyard") => "SearchDest::Graveyard",
                Some("Exile") => "SearchDest::Exile",
                Some("Library") => "SearchDest::TopOfLibrary",
                None if matches!(otherwise, "SearchDest::Graveyard" | "SearchDest::Exile") => {
                    otherwise
                }
                _ => return None,
            };
            return Some(vec![format!(
                "Effect::RevealUntilMatching {{ player: {player}, filter: &{name}, \
                 matched: {matched}, otherwise: {otherwise} }}"
            )]);
        }
        None
    }

    // -------------------------------------------------------------------------
    // Wave 8b: Goad, resolution-chosen color chains, RepeatEach, ChooseCard,
    // PeekAndReveal / RevealTop, and RevealFromHand.
    // -------------------------------------------------------------------------

    /// `AB$/SP$/DB$ Goad` -> `Effect::Goad { target }` or `Effect::GoadAll { filter, controlled_by }`.
    ///
    /// Strictly models CR 701.15 ("until your next turn"): `Duration$`,
    /// `RememberGoaded$`, and `NoLonger$` stay unclaimed and refused.
    pub(super) fn goad_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let defined = p.take("Defined");
        match defined.as_deref() {
            None | Some("Targeted" | "ParentTarget") => {
                let tgt = target.filter(|t| {
                    t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::ObjectOfEventPlayer(")
                })?;
                Some(vec![format!("Effect::Goad {{ target: {tgt} }}")])
            }
            Some("Self") if target.is_none() && !self.on_a_spell => Some(vec![
                "Effect::Goad { target: TargetSpec::ThisObject }".to_string(),
            ]),
            Some(def) if target.is_none() && self.is_event_object_defined(def) => Some(vec![
                "Effect::Goad { target: TargetSpec::EventObject }".to_string(),
            ]),
            Some(valid) if valid.starts_with("Valid ") => {
                let rest = valid.trim_start_matches("Valid ").trim();
                self.goad_all_effect(rest, target, targets_a_player)
            }
            _ => None,
        }
    }

    fn goad_all_effect(
        &mut self,
        rest: &str,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if let Some(base) = rest
            .strip_suffix(".ControlledBy TargetedPlayer")
            .or_else(|| rest.strip_suffix("+ControlledBy TargetedPlayer"))
        {
            if !targets_a_player {
                return None;
            }
            let expr = self.filter_expr(base.trim())?;
            let filter = self.body.filter_static("GOAD", &expr);
            return Some(vec![format!(
                "Effect::GoadAll {{ filter: &{filter}, controlled_by: Some(PlayerRel::Chosen) }}"
            )]);
        }
        if target.is_some() {
            return None;
        }
        if let Some((base, ctrl_raw)) = rest
            .rsplit_once(".ControlledBy ")
            .or_else(|| rest.rsplit_once("+ControlledBy "))
        {
            let ctrl_key = ctrl_raw
                .trim()
                .strip_prefix("Player.")
                .unwrap_or(ctrl_raw.trim());
            let who = self.player_rel(Some(ctrl_key))?;
            let expr = self.filter_expr(base.trim())?;
            let filter = self.body.filter_static("GOAD", &expr);
            return Some(vec![format!(
                "Effect::GoadAll {{ filter: &{filter}, controlled_by: Some({who}) }}"
            )]);
        }
        let expr = self.filter_expr(rest)?;
        let filter = self.body.filter_static("GOAD", &expr);
        Some(vec![format!(
            "Effect::GoadAll {{ filter: &{filter}, controlled_by: None }}"
        )])
    }

    /// Resolution-time `ChooseColor` chains: delegates `Animate`, `Effect`,
    /// and malformed headers to [`Self::choose_color_animate`] verbatim, and
    /// lowers `ProtectionAll`, `Protection`, and resolution-chosen-color
    /// sweeps (`DestroyAll`, `ChangeZoneAll`, `Discard`, `DealDamage`, …).
    pub(super) fn choose_color_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &mut super::Chain,
    ) -> Option<()> {
        let sub_body = sub.and_then(|name| self.svars.get(name).cloned());
        let sub_api = sub_body
            .as_deref()
            .and_then(Params::parse)
            .map(|(api, _)| api);
        let delegate_legacy = sub.is_none()
            || p.peek("Defined") != Some("You")
            || !p.entries.iter().all(|(k, _)| k == "Defined")
            || matches!(sub_api.as_deref(), None | Some("Animate" | "Effect"));
        if delegate_legacy {
            let effects = self.choose_color_animate(p, sub, chain)?;
            chain.effects.extend(effects);
            return Some(());
        }
        p.take("Defined");
        let sub_body = sub_body?;
        let (api, mut sp) = Params::parse(&sub_body)?;
        if api == "ProtectionAll" {
            return self.choose_color_protection_all(&mut sp, chain);
        }
        if api == "Protection" {
            return self.choose_color_protection(&mut sp, chain);
        }
        let sweep_ok = matches!(
            api.as_str(),
            "DestroyAll"
                | "ChangeZoneAll"
                | "Discard"
                | "DealDamage"
                | "DamageAll"
                | "PumpAll"
                | "TapAll"
                | "UntapAll"
                | "LoseLife"
                | "GainLife"
                | "RepeatEach"
                | "SacrificeAll"
                | "PutCounterAll"
                | "RemoveCounterAll"
                | "Destroy"
                | "Tap"
                | "Untap"
                | "ChangeZone"
                | "ChooseCard"
                | "PeekAndReveal"
                | "Reveal"
        );
        if !sweep_ok {
            return self.deny(format!("`ChooseColor` followed by `{api}`"));
        }
        chain.effects.push("Effect::ChooseColor".to_string());
        self.chain(&sub_body, chain)
    }

    fn choose_color_protection_all(
        &mut self,
        sp: &mut Params,
        chain: &mut super::Chain,
    ) -> Option<()> {
        sp.drop_prose();
        if sp.take("Gains").as_deref() != Some("ChosenColor") {
            return self.deny("`ProtectionAll` not granting `ChosenColor`".to_string());
        }
        if let Some(dur) = sp.take("Duration") {
            return self.deny(format!("`ProtectionAll` lasting `{dur}`"));
        }
        let Some(valid) = sp.take("ValidCards") else {
            return self.deny("`ProtectionAll` with no `ValidCards$`".to_string());
        };
        let next_sub = sp.take("SubAbility");
        if let Some(key) = sp.first_key() {
            return self.deny(format!("unclaimed parameter `ProtectionAll.{key}`"));
        }
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("PROT", &expr);
        chain.effects.push("Effect::ChooseColor".to_string());
        chain.effects.push(format!(
            "Effect::GrantProtectionFromChosenColorUntilEot {{ filter: &{filter} }}"
        ));
        self.chain_sub_ability(next_sub, chain)
    }

    fn choose_color_protection(&mut self, sp: &mut Params, chain: &mut super::Chain) -> Option<()> {
        sp.drop_prose();
        if sp.take("Gains").as_deref() != Some("ChosenColor") {
            return self.deny("`ChooseColor` `Protection` not granting `ChosenColor`".to_string());
        }
        if let Some(dur) = sp.take("Duration") {
            return self.deny(format!("`ChooseColor` `Protection` lasting `{dur}`"));
        }
        let next_sub = sp.take("SubAbility");
        sp.claim_label("ValidTgtsDesc", "ValidTgts");
        let valid_tgts = sp.take("ValidTgts");
        let defined = sp.take("Defined");
        let filter = if let Some(valid) = valid_tgts {
            if !matches!(defined.as_deref(), None | Some("Targeted")) {
                return self.deny(
                    "`ChooseColor` `Protection` with both `ValidTgts$` and `Defined$`".to_string(),
                );
            }
            self.chain_target("Protection", sp, &valid, chain)?;
            "Filter::This"
        } else {
            match defined.as_deref() {
                None | Some("Self") if !self.on_a_spell => "Filter::This",
                Some("Enchanted" | "Equipped") => "Filter::AttachedToBySource",
                Some(other) => {
                    return self.deny(format!("`ChooseColor` `Protection` on `Defined$ {other}`"));
                }
                None => return self.deny("`ChooseColor` `Protection` with no subject".to_string()),
            }
        };
        if let Some(key) = sp.first_key() {
            return self.deny(format!("unclaimed parameter `Protection.{key}`"));
        }
        chain.effects.push("Effect::ChooseColor".to_string());
        chain.effects.push(format!(
            "Effect::GrantProtectionFromChosenColorUntilEot {{ filter: &{filter} }}"
        ));
        self.chain_sub_ability(next_sub, chain)
    }

    // -------------------------------------------------------------------------
    // Shared Wave 8b helpers for scoped sub-chains and per-player counts.
    // -------------------------------------------------------------------------

    fn is_single_seat_rel(rel: &str) -> bool {
        matches!(
            rel,
            "PlayerRel::You"
                | "PlayerRel::Chosen"
                | "PlayerRel::EventPlayer"
                | "PlayerRel::ControllerOfTarget"
                | "PlayerRel::ControllerOfEvent"
                | "PlayerRel::OwnerOfEvent"
                | "PlayerRel::ActivePlayer"
                | "PlayerRel::DefendingPlayer"
                | "PlayerRel::DamagedPlayer"
                | "PlayerRel::ControllerOfAttached"
        )
    }

    fn is_wave8_cleanup_svar(&self, name: &str, allow_chosen_card: bool) -> bool {
        let Some(body) = self.svars.get(name.trim()) else {
            return false;
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return false;
        };
        p.drop_prose();
        if api != "Cleanup" {
            return false;
        }
        let mut any = false;
        if let Some(v) = p.take("ClearRemembered") {
            if v != "True" {
                return false;
            }
            any = true;
        }
        if allow_chosen_card && let Some(v) = p.take("ClearChosenCard") {
            if v != "True" {
                return false;
            }
            any = true;
        }
        if let Some(v) = p.take("ClearChosenColor") {
            if v != "True" {
                return false;
            }
            any = true;
        }
        any && p.exhausted()
    }

    fn reconstruct_spec(api: &str, p: &Params) -> String {
        let mut out = format!("DB$ {api}");
        for (k, v) in &p.entries {
            out.push_str(" | ");
            out.push_str(k);
            out.push_str("$ ");
            out.push_str(v);
        }
        out
    }

    fn wrap_amount_suffix(base: String, suffix: &str) -> Option<String> {
        if suffix.is_empty() {
            return Some(base);
        }
        if suffix == "HalfUp" {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: true }}"));
        }
        if suffix == "HalfDown" {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: false }}"));
        }
        if let Some(k) = suffix
            .strip_prefix("Times.")
            .and_then(|s| s.parse::<u32>().ok())
        {
            return Some(format!("Amount::Times {{ base: &{base}, factor: {k} }}"));
        }
        if let Some(k) = suffix
            .strip_prefix("Plus.")
            .and_then(|s| s.parse::<u32>().ok())
        {
            return Some(format!("Amount::Plus {{ base: &{base}, offset: {k} }}"));
        }
        if let Some(k) = suffix
            .strip_prefix("Minus.")
            .and_then(|s| s.parse::<u32>().ok())
        {
            return Some(format!(
                "Amount::SaturatingSub {{ base: &{base}, subtract: {k} }}"
            ));
        }
        None
    }

    fn strip_remembered_ctrl_or_own(valid: &str, suffix_a: &str, suffix_b: &str) -> Option<String> {
        let mut parts = Vec::new();
        for alt in valid.split(',') {
            let trimmed = alt.trim();
            let stripped = trimmed
                .strip_suffix(&format!(".{suffix_a}"))
                .or_else(|| trimmed.strip_suffix(&format!("+{suffix_a}")))
                .or_else(|| trimmed.strip_suffix(&format!(".{suffix_b}")))
                .or_else(|| trimmed.strip_suffix(&format!("+{suffix_b}")))?;
            if stripped.contains("Remembered") {
                return None;
            }
            parts.push(stripped.trim());
        }
        if parts.is_empty() {
            return None;
        }
        Some(parts.join(","))
    }

    /// Resolves per-player `SVar` counts referencing the active `RepeatEach`
    /// loop player (`RememberedPlayerCtrl`, `RememberedPlayerOwn`,
    /// `PlayerCountRemembered$…`) against `who` (`EventPlayer` or `Recipient`).
    fn scoped_player_amount(&mut self, raw: &str, who: &str) -> Option<String> {
        let def = self.svars.get(raw.trim())?.trim().to_string();
        let (head, suffix) = def
            .split_once('/')
            .map_or((def.as_str(), ""), |(h, s)| (h, s));
        let base = if let Some(valid) = head.strip_prefix("Count$Valid ") {
            let clean = Self::strip_remembered_ctrl_or_own(
                valid.trim(),
                "RememberedPlayerCtrl",
                "RememberedPlayerCtrl",
            )?;
            let expr = self.filter_expr(&clean)?;
            let filter = self.body.filter_static("COUNT", &expr);
            format!("Amount::ControlledCount {{ who: {who}, filter: &{filter} }}")
        } else if let Some(valid) = head.strip_prefix("PlayerCountRemembered$Valid ") {
            let clean = Self::strip_remembered_ctrl_or_own(valid.trim(), "YouCtrl", "YouCtrl")?;
            let expr = self.filter_expr(&clean)?;
            let filter = self.body.filter_static("COUNT", &expr);
            format!("Amount::ControlledCount {{ who: {who}, filter: &{filter} }}")
        } else if matches!(
            head,
            "Count$ValidHand Card.RememberedPlayerOwn"
                | "Count$ValidHand Card.RememberedPlayerCtrl"
                | "PlayerCountRemembered$CardsInHand"
        ) {
            format!("Amount::HandSize({who})")
        } else if matches!(
            head,
            "Count$ValidLibrary Card.RememberedPlayerOwn"
                | "Count$ValidLibrary Card.RememberedPlayerCtrl"
                | "PlayerCountRemembered$CardsInLibrary"
        ) {
            format!("Amount::LibrarySize({who})")
        } else if head == "PlayerCountRemembered$LifeTotal" {
            format!("Amount::LifeTotal({who})")
        } else {
            return None;
        };
        Self::wrap_amount_suffix(base, suffix)
    }

    /// `AB$/SP$/DB$ RepeatEach` -> preserves legacy special cases, then lowers
    /// `DamageMap$ True` simultaneous sweeps and general `Effect::RepeatEach`.
    pub(super) fn repeat_each_chain(
        &mut self,
        p: Params,
        sub: Option<&str>,
        chain: &mut super::Chain,
    ) -> Option<()> {
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        if let Some(effects) = self.repeat_each_effect(p.clone(), sub, chain) {
            chain.effects.extend(effects);
            return Some(());
        }
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        self.body.statics.truncate(saved_statics_len);
        self.wave8_repeat_each(p, sub, chain)
    }

    #[allow(clippy::too_many_lines)]
    fn wave8_repeat_each(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &mut super::Chain,
    ) -> Option<()> {
        let outer_cond = self.take_wave8_line_condition(&mut p).ok()?;
        if let Some(effects) = self.try_repeat_damage_map(&mut p, outer_cond.as_deref()) {
            chain.effects.extend(effects);
            return self.chain_sub_ability(sub.map(str::to_string), chain);
        }
        let repeat_players = p.take("RepeatPlayers");
        let repeat_cards = p.take("RepeatCards");
        let repeat_present = p.take("RepeatPresent");
        let Some(repeat_sub) = p.take("RepeatSubAbility") else {
            return self.deny("`RepeatEach` with no `RepeatSubAbility$`".to_string());
        };
        let (domain, scope) = match (repeat_players, repeat_cards, repeat_present) {
            (Some(rp), None, None) => {
                let d = match rp.as_str() {
                    "Player" => "RepeatDomain::EachPlayer",
                    "Opponent" | "Player.Opponent" => "RepeatDomain::EachOpponent",
                    other => {
                        return self.deny(format!("`RepeatEach` with `RepeatPlayers$ {other}`"));
                    }
                };
                (d.to_string(), Wave8Scope::RepeatPlayer)
            }
            (None, Some(rc), None) => {
                if let Some(z) = p.take("Zone")
                    && z != "Battlefield"
                {
                    return self.deny(format!("`RepeatEach` `RepeatCards$` in `Zone$ {z}`"));
                }
                if rc.contains("IsRemembered")
                    || rc.contains("IsImprinted")
                    || rc.contains("Remembered")
                {
                    return self.deny(format!("`RepeatEach` over remembered `{rc}`"));
                }
                let expr = self.filter_expr(&rc)?;
                let filter = self.body.filter_static("REPEAT", &expr);
                (
                    format!("RepeatDomain::EachMatchingPermanent(&{filter})"),
                    Wave8Scope::RepeatPermanent,
                )
            }
            (None, None, Some(rp)) => {
                if let Some(z) = p.take("PresentZone")
                    && z != "Battlefield"
                {
                    return self.deny(format!(
                        "`RepeatEach` `RepeatPresent$` in `PresentZone$ {z}`"
                    ));
                }
                if rp.contains("IsRemembered")
                    || rp.contains("IsImprinted")
                    || rp.contains("sharesNameWith")
                    || rp.contains("Remembered")
                {
                    return self.deny(format!("`RepeatEach` `RepeatPresent$ {rp}`"));
                }
                let expr = self.filter_expr(&rp)?;
                let filter = self.body.filter_static("REPEAT", &expr);
                (
                    format!(
                        "RepeatDomain::Times(Amount::CountOf {{ filter: &{filter}, zone: ZoneSel::Battlefield }})"
                    ),
                    Wave8Scope::Times,
                )
            }
            _ => {
                return self.deny("`RepeatEach` with unsupported repeat domain".to_string());
            }
        };
        let change_zone_table = match p.take("ChangeZoneTable").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`RepeatEach` with `ChangeZoneTable$ {other}`"));
            }
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `RepeatEach.{key}`"));
        }
        let body_effects = self.compile_wave8_scoped_chain(&repeat_sub, scope, None, false)?;
        if change_zone_table
            && !body_effects
                .iter()
                .all(|e| e.starts_with("Effect::CreateToken"))
        {
            return self.deny("`RepeatEach` with `ChangeZoneTable$ True`".to_string());
        }
        if (chain.target.is_some() || chain.second_target.is_some())
            && body_effects.iter().any(|e| e.contains("Filter::Reference"))
        {
            return self
                .deny("`RepeatEach` `Filter::Reference` beside an outer target".to_string());
        }
        let repeat_expr = format!(
            "Effect::RepeatEach {{ domain: {domain}, effects: &[{}] }}",
            body_effects.join(", ")
        );
        chain.effects.push(match outer_cond {
            Some(c) => format!(
                "Effect::IfCondition {{ condition: {c}, then: &[{repeat_expr}], otherwise: &[] }}"
            ),
            None => repeat_expr,
        });
        self.chain_sub_ability(sub.map(str::to_string), chain)
    }

    /// Simultaneous `RepeatEach | RepeatPlayers$ Player/Opponent | DamageMap$ True`
    /// where `RepeatSubAbility$` is a pure per-player `DealDamage`.
    fn try_repeat_damage_map(
        &mut self,
        p: &mut Params,
        outer_cond: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.peek("DamageMap") != Some("True") {
            return None;
        }
        let who = match p.peek("RepeatPlayers") {
            Some("Player") => "PlayerRel::EachPlayer",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            _ => return None,
        };
        let sub_name = p.peek("RepeatSubAbility")?.to_string();
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "DealDamage"
            || !matches!(
                sp.take("Defined").as_deref(),
                Some("Remembered" | "Player.IsRemembered" | "RememberedPlayer")
            )
        {
            return None;
        }
        let dmg_raw = sp.take("NumDmg")?;
        if let Some(sub_clean) = sp.take("SubAbility")
            && !self.is_wave8_cleanup_svar(&sub_clean, false)
        {
            return None;
        }
        if !sp.exhausted() {
            return None;
        }
        let amount = self.scoped_player_amount(&dmg_raw, "PlayerRel::Recipient")?;
        let mut probe = p.clone();
        probe.take("DamageMap");
        probe.take("RepeatPlayers");
        probe.take("RepeatSubAbility");
        if !probe.exhausted() {
            return None;
        }
        p.take("DamageMap");
        p.take("RepeatPlayers");
        p.take("RepeatSubAbility");
        let eff =
            format!("Effect::DealDamage {{ amount: {amount}, target: TargetSpec::Player({who}) }}");
        Some(vec![match outer_cond {
            Some(c) => {
                format!("Effect::IfCondition {{ condition: {c}, then: &[{eff}], otherwise: &[] }}")
            }
            None => eff,
        }])
    }

    fn take_wave8_line_condition(&mut self, p: &mut Params) -> Result<Option<String>, ()> {
        if let Some(raw) = p.peek("Condition")
            && let Some(cond) =
                Self::named_condition(raw).or_else(|| Self::wave7_named_condition(raw))
        {
            p.take("Condition");
            p.take("ConditionDescription");
            return Ok(Some(cond.to_string()));
        }
        if let Some(var) = p.peek("ConditionCheckSVar").map(str::to_string) {
            let cmp = p
                .peek("ConditionSVarCompare")
                .map_or_else(|| "GE1".to_string(), str::to_string);
            let Some(cond) = self
                .svar_condition(&var, &cmp)
                .or_else(|| self.extra_svar_condition_clause(&var, &cmp))
                .or_else(|| self.wave7_svar_condition(&var, &cmp))
            else {
                return Err(());
            };
            p.take("ConditionCheckSVar");
            p.take("ConditionSVarCompare");
            p.take("ConditionDescription");
            return Ok(Some(cond));
        }
        Ok(None)
    }

    /// Compiles a non-targeting sub-ability chain inside `RepeatEach`,
    /// `ChooseCard.then`, `PeekAndReveal.if_picked`, or `RevealFromHand.if_revealed`.
    fn compile_wave8_scoped_chain(
        &mut self,
        start_svar: &str,
        scope: Wave8Scope,
        strip_remembered_filter: Option<&str>,
        allow_chosen_card_cleanup: bool,
    ) -> Option<Vec<String>> {
        let mut out = Vec::new();
        let mut cursor = Some(start_svar.trim().to_string());
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 16 {
                return self.deny("a cyclic scoped `SubAbility$` chain".to_string());
            }
            if self.is_wave8_cleanup_svar(&name, allow_chosen_card_cleanup) {
                break;
            }
            let Some(body) = self.svars.get(&name).cloned() else {
                return self.deny(format!("`SubAbility$ {name}` names no SVar"));
            };
            let (mut effects, next) = self.compile_wave8_scoped_step(
                &body,
                scope,
                strip_remembered_filter,
                allow_chosen_card_cleanup,
            )?;
            out.append(&mut effects);
            cursor = next;
        }
        if out.is_empty() {
            return self.deny("an empty scoped sub-ability chain".to_string());
        }
        Some(out)
    }

    fn strip_matching_remembered_guard(
        sp: &mut Params,
        expected_defined: &str,
        expected_filter: Option<&str>,
    ) -> bool {
        if sp.peek("ConditionDefined") != Some(expected_defined) {
            return false;
        }
        if !matches!(sp.peek("ConditionCompare"), None | Some("GE1" | "EQ1")) {
            return false;
        }
        let Some(present) = sp.peek("ConditionPresent") else {
            return false;
        };
        let matches_filter = present == "Card" || expected_filter == Some(present);
        if !matches_filter {
            return false;
        }
        sp.take("ConditionDefined");
        sp.take("ConditionPresent");
        sp.take("ConditionCompare");
        sp.take("ConditionDescription");
        true
    }

    #[allow(clippy::too_many_lines)]
    fn compile_wave8_scoped_step(
        &mut self,
        body: &str,
        scope: Wave8Scope,
        strip_remembered_filter: Option<&str>,
        allow_chosen_card_cleanup: bool,
    ) -> Option<(Vec<String>, Option<String>)> {
        let (api, mut sp) = Params::parse(body)?;
        sp.drop_prose();
        if sp.has("ValidTgts") {
            return self.deny(format!("scoped `{api}` with `ValidTgts$`"));
        }
        let next_sub = sp.take("SubAbility");
        if scope == Wave8Scope::ChosenObject {
            Self::strip_matching_remembered_guard(&mut sp, "ChosenCard", None);
        } else if scope == Wave8Scope::RevealedObject {
            Self::strip_matching_remembered_guard(&mut sp, "Remembered", strip_remembered_filter);
        }
        if scope == Wave8Scope::RepeatPlayer {
            if let Some(res) = self.try_repeat_player_special(&api, &mut sp, next_sub.as_deref()) {
                return Some(res);
            }
            if sp.entries.iter().any(|(_, v)| v.contains("Triggered")) {
                return self.deny(format!("`RepeatEach` `{api}` referencing outer trigger"));
            }
            for (k, v) in &mut sp.entries {
                if matches!(k.as_str(), "Defined" | "UnlessPayer")
                    && matches!(
                        v.as_str(),
                        "Remembered" | "Player.IsRemembered" | "RememberedPlayer"
                    )
                {
                    *v = "TriggeredPlayer".to_string();
                }
            }
            let spec = Self::reconstruct_spec(&api, &sp);
            let mut inner = super::Chain::default();
            let saved_mode = self.trigger_mode.replace("LifeGained".into());
            let ok = self.chain(&spec, &mut inner);
            self.trigger_mode = saved_mode;
            ok?;
            if inner.target.is_some() || inner.second_target.is_some() {
                return self.deny(format!("scoped `{api}` declaring a target"));
            }
            return Some((inner.effects, next_sub));
        }
        if matches!(
            scope,
            Wave8Scope::RepeatPermanent | Wave8Scope::ChosenObject | Wave8Scope::RevealedObject
        ) {
            if sp
                .entries
                .iter()
                .any(|(_, v)| v.contains("DelayTriggerRemembered") || v.contains("Triggered"))
            {
                return self.deny(format!("scoped `{api}` referencing outer event object"));
            }
            let mut uses_event_obj = false;
            let mut uses_event_player = false;
            for (k, v) in &mut sp.entries {
                if k == "Defined" {
                    let is_obj = match scope {
                        Wave8Scope::ChosenObject => v == "ChosenCard",
                        Wave8Scope::RepeatPermanent | Wave8Scope::RevealedObject => {
                            matches!(v.as_str(), "Remembered" | "RememberedLKI")
                        }
                        _ => false,
                    };
                    let is_ctrl = match scope {
                        Wave8Scope::ChosenObject => v == "ChosenCardController",
                        Wave8Scope::RepeatPermanent => v == "RememberedController",
                        _ => false,
                    };
                    if is_obj {
                        *v = "DelayTriggerRemembered".to_string();
                        uses_event_obj = true;
                    } else if is_ctrl {
                        *v = "TriggeredPlayer".to_string();
                        uses_event_player = true;
                    }
                } else if k == "UnlessPayer"
                    && ((scope == Wave8Scope::RepeatPermanent && v == "RememberedController")
                        || (scope == Wave8Scope::ChosenObject && v == "ChosenCardController"))
                {
                    *v = "TriggeredPlayer".to_string();
                    uses_event_player = true;
                }
            }
            let spec = Self::reconstruct_spec(&api, &sp);
            let mut inner = super::Chain::default();
            let saved_delayed = std::mem::replace(&mut self.in_delayed, uses_event_obj);
            let saved_mode = if uses_event_player {
                self.trigger_mode.replace("LifeGained".into())
            } else {
                self.trigger_mode.clone()
            };
            let ok = self.chain(&spec, &mut inner);
            self.in_delayed = saved_delayed;
            self.trigger_mode = saved_mode;
            ok?;
            if inner.target.is_some() || inner.second_target.is_some() {
                return self.deny(format!("scoped `{api}` declaring a target"));
            }
            let _ = allow_chosen_card_cleanup;
            return Some((inner.effects, next_sub));
        }
        let spec = Self::reconstruct_spec(&api, &sp);
        let mut inner = super::Chain::default();
        self.chain(&spec, &mut inner)?;
        if inner.target.is_some() || inner.second_target.is_some() {
            return self.deny(format!("scoped `{api}` declaring a target"));
        }
        Some((inner.effects, next_sub))
    }

    /// Per-player `RepeatEach` body steps that use `RememberedPlayerCtrl` /
    /// `PlayerCountRemembered$` counts, `TokenOwner$ Player.IsRemembered`, or
    /// the wheel (`ChangeZoneAll Hand->Library Shuffle` + `Draw Remembered$Amount`).
    #[allow(clippy::too_many_lines)]
    fn try_repeat_player_special(
        &mut self,
        api: &str,
        sp: &mut Params,
        next_sub: Option<&str>,
    ) -> Option<(Vec<String>, Option<String>)> {
        let is_rem_player = |v: Option<&str>| {
            matches!(
                v,
                Some("Remembered" | "Player.IsRemembered" | "RememberedPlayer")
            )
        };
        if api == "Token" && is_rem_player(sp.peek("TokenOwner")) {
            let mut probe = sp.clone();
            probe.take("TokenOwner");
            let stem = probe.take("TokenScript")?;
            let raw_amt = probe.take("TokenAmount");
            if !probe.exhausted() {
                return None;
            }
            let tokens = self.tokens?;
            let body = tokens.body(&stem, self.cats)?;
            let token = format!("&generated_tokens::{}", body.constant);
            let amount = match raw_amt.as_deref() {
                None => "Amount::Fixed(1)".to_string(),
                Some(raw) => self
                    .scoped_player_amount(raw, "PlayerRel::EventPlayer")
                    .or_else(|| self.amount_or_count_ext(raw))?,
            };
            return Some((
                vec![format!(
                    "Effect::CreateTokensFor {{ player: PlayerRel::EventPlayer, token: {token}, amount: {amount} }}"
                )],
                next_sub.map(str::to_string),
            ));
        }
        // Wheel inside `RepeatEach`: shuffle hand into library, then draw that many cards.
        if api == "ChangeZoneAll"
            && is_rem_player(sp.peek("Defined"))
            && sp.peek("Origin") == Some("Hand")
            && sp.peek("Destination") == Some("Library")
            && sp.peek("Shuffle") == Some("True")
            && sp.peek("RememberChanged") == Some("True")
            && let Some(draw_name) = next_sub
            && let Some(draw_body) = self.svars.get(draw_name.trim()).cloned()
            && let Some((draw_api, mut dp)) = Params::parse(&draw_body)
        {
            dp.drop_prose();
            let mut probe = sp.clone();
            probe.take("Defined");
            probe.take("Origin");
            probe.take("Destination");
            probe.take("Shuffle");
            probe.take("RememberChanged");
            probe.take("LibraryPosition");
            let after_draw = dp.take("SubAbility");
            let draw_var = dp.take("NumCards");
            let draw_def = dp.take("Defined");
            if probe.exhausted()
                && draw_api == "Draw"
                && is_rem_player(draw_def.as_deref())
                && dp.exhausted()
                && draw_var
                    .as_deref()
                    .and_then(|v| self.svars.get(v.trim()))
                    .map(|s| s.trim())
                    == Some("Remembered$Amount")
            {
                return Some((
                    vec![
                        "Effect::ShuffleIntoLibrary { who: PlayerRel::EventPlayer, hand: true, graveyard: false }"
                            .to_string(),
                        "Effect::DrawCardsFor { amount: Amount::ThisWayBy { what: ThisWay::PutIntoLibraryFromHand, who: PlayerRel::EventPlayer }, who: PlayerRel::EventPlayer }"
                            .to_string(),
                    ],
                    after_draw,
                ));
            }
        }
        if !is_rem_player(sp.peek("Defined")) {
            return None;
        }
        let mut probe = sp.clone();
        probe.take("Defined");
        let effect = match api {
            "LoseLife" => {
                let raw = probe.take("LifeAmount")?;
                if !probe.exhausted() {
                    return None;
                }
                let amt = self.scoped_player_amount(&raw, "PlayerRel::EventPlayer")?;
                format!("Effect::LoseLife {{ amount: {amt}, target: PlayerRel::EventPlayer }}")
            }
            "GainLife" => {
                let raw = probe.take("LifeAmount")?;
                if !probe.exhausted() {
                    return None;
                }
                let amt = self.scoped_player_amount(&raw, "PlayerRel::EventPlayer")?;
                format!("Effect::GainLifeFor {{ amount: {amt}, target: PlayerRel::EventPlayer }}")
            }
            "DealDamage" => {
                let raw = probe.take("NumDmg")?;
                if !probe.exhausted() {
                    return None;
                }
                let amt = self.scoped_player_amount(&raw, "PlayerRel::EventPlayer")?;
                format!(
                    "Effect::DealDamage {{ amount: {amt}, target: TargetSpec::Player(PlayerRel::EventPlayer) }}"
                )
            }
            "Draw" => {
                let raw = probe.take("NumCards")?;
                if !probe.exhausted() {
                    return None;
                }
                let amt = self.scoped_player_amount(&raw, "PlayerRel::EventPlayer")?;
                format!("Effect::DrawCardsFor {{ amount: {amt}, who: PlayerRel::EventPlayer }}")
            }
            "Mill" => {
                let raw = probe.take("NumCards")?;
                if !probe.exhausted() {
                    return None;
                }
                let amt = self.scoped_player_amount(&raw, "PlayerRel::EventPlayer")?;
                format!("Effect::Mill {{ amount: {amt}, target: PlayerRel::EventPlayer }}")
            }
            "Discard" => {
                let mode = probe.take("Mode")?;
                let raw = probe.take("NumCards")?;
                if !probe.exhausted() {
                    return None;
                }
                let amt = self.scoped_player_amount(&raw, "PlayerRel::EventPlayer")?;
                match mode.as_str() {
                    "TgtChoose" => format!(
                        "Effect::DiscardAmountForPlayers {{ who: PlayerRel::EventPlayer, amount: {amt} }}"
                    ),
                    "Random" => format!(
                        "Effect::DiscardRandom {{ who: PlayerRel::EventPlayer, count: {amt} }}"
                    ),
                    _ => return None,
                }
            }
            _ => return None,
        };
        Some((vec![effect], next_sub.map(str::to_string)))
    }

    // -------------------------------------------------------------------------
    // `ChooseCard` -> `Effect::ChooseCard` (Battlefield & Graveyard only)
    // -------------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    pub(super) fn choose_card_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut super::Chain,
    ) -> Option<()> {
        p.take("ChoiceTitle");
        p.take("ChoiceDesc");
        let outer_cond = self.take_wave8_line_condition(&mut p).ok()?;
        let chooser =
            self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let repeat_domain = match chooser {
            "PlayerRel::EachPlayer" => Some("RepeatDomain::EachPlayer"),
            "PlayerRel::EachOpponent" => Some("RepeatDomain::EachOpponent"),
            _ if Self::is_single_seat_rel(chooser) => None,
            _ => return self.deny(format!("`ChooseCard` for multi-seat `{chooser}`")),
        };
        let eff_chooser = if repeat_domain.is_some() {
            "PlayerRel::EventPlayer"
        } else {
            chooser
        };
        let controlled_by_chooser = match p.peek("ControlledByPlayer") {
            Some("Chooser") if chooser == "PlayerRel::You" => {
                p.take("ControlledByPlayer");
                true
            }
            _ => false,
        };
        let Some(choices_raw) = p.take("Choices") else {
            return self.deny("`ChooseCard` with no `Choices$`".to_string());
        };
        if choices_raw.contains("IsRemembered")
            || choices_raw.contains("Remembered")
            || choices_raw.contains("Targeted") && !choices_raw.contains("TargetedPlayerOwn")
        {
            return self.deny(format!("`ChooseCard` with `Choices$ {choices_raw}`"));
        }
        let choices_effective = if controlled_by_chooser {
            choices_raw
                .split(',')
                .map(|alt| {
                    let trimmed = alt.trim();
                    if trimmed.contains('.') {
                        format!("{trimmed}+YouCtrl")
                    } else {
                        format!("{trimmed}.YouCtrl")
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        } else {
            choices_raw
        };
        let (zone, zone_owner, filter_name) = match p.take("ChoiceZone").as_deref() {
            None | Some("Battlefield") => {
                let expr = self.filter_expr(&choices_effective)?;
                let name = self.body.filter_static("CHOOSE", &expr);
                ("ChooseZone::Battlefield".to_string(), None, name)
            }
            Some("Graveyard") if !controlled_by_chooser => {
                let (owner_rel, stripped) =
                    self.extract_choose_graveyard_owner(&choices_effective, targets_a_player)?;
                let expr = self.filter_expr_in_zone(&stripped, true)?;
                let name = self.body.filter_static("CHOOSE", &expr);
                (
                    format!("ChooseZone::Graveyard({owner_rel})"),
                    Some(owner_rel),
                    name,
                )
            }
            Some(other) => {
                return self.deny(format!("`ChooseCard` in `ChoiceZone$ {other}`"));
            }
        };
        let mandatory = match p.take("Mandatory").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`ChooseCard` with `Mandatory$ {other}`"));
            }
        };
        let raw_amount = p.take("Amount");
        let max_expr = match raw_amount.as_deref() {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => self.amount_or_count_ext(raw)?,
        };
        let min: u8 = if let Some(raw_min) = p.take("MinAmount") {
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
        let Some(sub_name) = sub else {
            return self.deny("`ChooseCard` with no `SubAbility$`".to_string());
        };
        let (movement, then_effects, trailing_sub) =
            self.choose_card_followup(sub_name, zone_owner)?;
        if (chain.target.is_some() || chain.second_target.is_some())
            && then_effects.iter().any(|e| e.contains("Filter::Reference"))
        {
            return self
                .deny("`ChooseCard` `Filter::Reference` beside an outer target".to_string());
        }
        let then_joined = then_effects.join(", ");
        let inner_choose = format!(
            "Effect::ChooseCard {{ chooser: {eff_chooser}, zone: {zone}, filter: &{filter_name}, \
             min: {min}, max: {max_expr}, movement: {movement}, then: &[{then_joined}] }}"
        );
        let choose_expr = match repeat_domain {
            Some(domain) => {
                format!("Effect::RepeatEach {{ domain: {domain}, effects: &[{inner_choose}] }}")
            }
            None => inner_choose,
        };
        chain.effects.push(match outer_cond {
            Some(c) => format!(
                "Effect::IfCondition {{ condition: {c}, then: &[{choose_expr}], otherwise: &[] }}"
            ),
            None => choose_expr,
        });
        self.chain_sub_ability(trailing_sub, chain)
    }

    fn extract_choose_graveyard_owner(
        &mut self,
        raw: &str,
        targets_a_player: bool,
    ) -> Option<(&'static str, String)> {
        let mut owner: Option<&'static str> = None;
        let mut stripped_alts = Vec::new();
        for alt in raw.split(',') {
            let trimmed = alt.trim();
            let (alt_owner, clean) = if let Some(rest) = trimmed
                .strip_suffix(".YouOwn")
                .or_else(|| trimmed.strip_suffix("+YouOwn"))
            {
                ("PlayerRel::You", rest)
            } else if let Some(rest) = trimmed
                .strip_suffix(".OppOwn")
                .or_else(|| trimmed.strip_suffix("+OppOwn"))
            {
                ("PlayerRel::EachOpponent", rest)
            } else if let Some(rest) = trimmed
                .strip_suffix(".TargetedPlayerOwn")
                .or_else(|| trimmed.strip_suffix("+TargetedPlayerOwn"))
            {
                if !targets_a_player {
                    return self.deny("`TargetedPlayerOwn` without player target".to_string());
                }
                ("PlayerRel::Chosen", rest)
            } else if trimmed.contains("Own") || trimmed.contains("Ctrl") {
                return self.deny(format!("`ChooseCard` graveyard owner in `{trimmed}`"));
            } else {
                ("PlayerRel::EachPlayer", trimmed)
            };
            if let Some(prev) = owner
                && prev != alt_owner
            {
                return self.deny("`ChooseCard` mixing graveyard owners".to_string());
            }
            owner = Some(alt_owner);
            stripped_alts.push(clean.trim());
        }
        Some((owner?, stripped_alts.join(",")))
    }

    fn choose_card_followup(
        &mut self,
        sub_name: &str,
        zone_owner: Option<&'static str>,
    ) -> Option<(String, Vec<String>, Option<String>)> {
        let Some(body) = self.svars.get(sub_name.trim()).cloned() else {
            return self.deny(format!("`SubAbility$ {sub_name}` names no SVar"));
        };
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        let expected_origin = if zone_owner.is_some() {
            "Graveyard"
        } else {
            "Battlefield"
        };
        if api == "ChangeZone"
            && sp.peek("Defined") == Some("ChosenCard")
            && sp.peek("Origin") == Some(expected_origin)
        {
            sp.take("Defined");
            sp.take("Origin");
            let after = sp.take("SubAbility");
            let tapped = match sp.take("Tapped").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(other) => return self.deny(format!("`ChangeZone` with `Tapped$ {other}`")),
            };
            let gain = match sp.take("GainControl").as_deref() {
                None | Some("False") => false,
                Some("True") => true,
                Some(other) => {
                    return self.deny(format!("`ChangeZone` with `GainControl$ {other}`"));
                }
            };
            let movement = match sp.take("Destination").as_deref() {
                Some("Hand") if !tapped && !gain => "ChooseMove::Hand".to_string(),
                Some("Graveyard") if !tapped && !gain => "ChooseMove::Graveyard".to_string(),
                Some("Exile") if !tapped && !gain => "ChooseMove::Exile".to_string(),
                Some("Battlefield") => {
                    // `ChooseMove::Battlefield` puts the card onto the
                    // battlefield under `you`'s control; require `GainControl`
                    // whenever the chosen card could belong to another player.
                    if zone_owner != Some("PlayerRel::You") && !gain {
                        return self.deny(
                            "`ChooseCard` battlefield move without `GainControl$ True`".to_string(),
                        );
                    }
                    format!("ChooseMove::Battlefield {{ tapped: {tapped} }}")
                }
                Some("Library") if !tapped && !gain => {
                    match sp.take("LibraryPosition").as_deref() {
                        Some("0") => "ChooseMove::TopOfLibrary".to_string(),
                        Some("-1") => "ChooseMove::BottomOfLibrary".to_string(),
                        other => {
                            return self.deny(format!(
                                "`ChooseCard` `LibraryPosition$ {}`",
                                other.unwrap_or("none")
                            ));
                        }
                    }
                }
                other => {
                    return self.deny(format!(
                        "`ChooseCard` move to `{}`",
                        other.unwrap_or("none")
                    ));
                }
            };
            if let Some(key) = sp.first_key() {
                return self.deny(format!("unclaimed parameter `ChangeZone.{key}`"));
            }
            let trailing = match after {
                Some(ref name) if self.is_wave8_cleanup_svar(name, true) => None,
                other => other,
            };
            return Some((movement, Vec::new(), trailing));
        }
        let then_effects =
            self.compile_wave8_scoped_chain(sub_name, Wave8Scope::ChosenObject, None, true)?;
        Some(("ChooseMove::None".to_string(), then_effects, None))
    }

    // -------------------------------------------------------------------------
    // `PeekAndReveal` -> `Effect::PeekAndReveal` / `Effect::RevealTop`
    // -------------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    pub(super) fn peek_and_reveal_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut super::Chain,
    ) -> Option<()> {
        let outer_cond = self.take_wave8_line_condition(&mut p).ok()?;
        let library =
            self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let count = match p.take("PeekAmount") {
            None => "Amount::Fixed(1)".to_string(),
            Some(raw) => self.amount_or_count_ext(&raw)?,
        };
        let take_bool = |p: &mut Params, key: &str| -> Result<bool, String> {
            match p.take(key).as_deref() {
                None | Some("False") => Ok(false),
                Some("True") => Ok(true),
                Some(other) => Err(format!("`PeekAndReveal` with `{key}$ {other}`")),
            }
        };
        let no_peek = take_bool(&mut p, "NoPeek").map_err(|e| self.note(e)).ok()?;
        let no_reveal = take_bool(&mut p, "NoReveal")
            .map_err(|e| self.note(e))
            .ok()?;
        let reveal_optional = take_bool(&mut p, "RevealOptional")
            .map_err(|e| self.note(e))
            .ok()?;
        let remember_revealed = take_bool(&mut p, "RememberRevealed")
            .map_err(|e| self.note(e))
            .ok()?;
        let remember_peeked = take_bool(&mut p, "RememberPeeked")
            .map_err(|e| self.note(e))
            .ok()?;
        let reveal_valid = p.take("RevealValid");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `PeekAndReveal.{key}`"));
        }
        if !Self::is_single_seat_rel(library) {
            if no_reveal || reveal_optional || remember_peeked || reveal_valid.is_some() {
                return self.deny(format!("`PeekAndReveal` for multi-seat `{library}`"));
            }
            return self.peek_multi_seat_reveal(
                library,
                &count,
                remember_revealed,
                outer_cond.as_deref(),
                sub,
                chain,
            );
        }
        let (effect, trailing_sub) = if no_reveal
            && !no_peek
            && !remember_revealed
            && !remember_peeked
            && reveal_valid.is_none()
            && !reveal_optional
        {
            // Pure private look (e.g. "Look at the top N cards of your library").
            (
                format!(
                    "Effect::PeekAndReveal {{ library: {library}, count: {count}, \
                     reveal_all: false, reveal_chosen: false, filter: &Filter::Any, \
                     min_pick: 0, max_pick: 0, picked: PeekPickDest::StayOnTop, \
                     rest: PeekRestDest::StayInOrder, if_picked: &[] }}"
                ),
                sub.map(str::to_string),
            )
        } else if count == "Amount::Fixed(1)"
            && !no_peek
            && !no_reveal
            && reveal_optional
            && remember_revealed
        {
            // Kinship & optional top-1 reveal (`RevealValid$ <F> | RevealOptional$ True`).
            let sub_name = sub
                .ok_or_else(|| {
                    self.note("`PeekAndReveal` `RevealOptional` with no `SubAbility$`".to_string());
                })
                .ok()?;
            let eff = self.peek_optional_top_one(library, reveal_valid.as_deref(), sub_name)?;
            (eff, None)
        } else if no_peek
            && !no_reveal
            && !reveal_optional
            && reveal_valid.is_none()
            && !remember_peeked
        {
            // Public reveal of top N cards (`Effect::RevealTop` or pick/rest `PeekAndReveal`).
            self.peek_public_reveal_top(library, &count, remember_revealed, sub)?
        } else if no_reveal && !no_peek && remember_peeked && reveal_valid.is_none() {
            // Private multi-card peek -> ChangeZone pick + rest placement.
            let sub_name = sub
                .ok_or_else(|| {
                    self.note("`PeekAndReveal` `RememberPeeked` with no `SubAbility$`".to_string());
                })
                .ok()?;
            let eff = self.peek_multi_pick_rest(library, &count, false, sub_name)?;
            (eff, None)
        } else {
            return self.deny("unsupported `PeekAndReveal` combination".to_string());
        };
        if (chain.target.is_some() || chain.second_target.is_some())
            && effect.contains("Filter::Reference")
        {
            return self
                .deny("`PeekAndReveal` `Filter::Reference` beside an outer target".to_string());
        }
        chain.effects.push(match outer_cond {
            Some(c) => format!(
                "Effect::IfCondition {{ condition: {c}, then: &[{effect}], otherwise: &[] }}"
            ),
            None => effect,
        });
        self.chain_sub_ability(trailing_sub, chain)
    }

    /// Multi-seat `PeekAndReveal` (`Defined$ Player / Opponent`): lowers
    /// APNAP top-of-library reveal via `Effect::RepeatEach { ..., RevealTop }`
    /// and handles Parley chains counting `Remembered$Valid <F>` via
    /// `Amount::ThisWayBy { what: ThisWay::Revealed(&F), who }`.
    fn peek_multi_seat_reveal(
        &mut self,
        library: &str,
        count: &str,
        remember_revealed: bool,
        outer_cond: Option<&str>,
        sub: Option<&str>,
        chain: &mut super::Chain,
    ) -> Option<()> {
        let domain = match library {
            "PlayerRel::EachPlayer" => "RepeatDomain::EachPlayer",
            "PlayerRel::EachOpponent" => "RepeatDomain::EachOpponent",
            _ => return self.deny(format!("`PeekAndReveal` for multi-seat `{library}`")),
        };
        let reveal_loop = format!(
            "Effect::RepeatEach {{ domain: {domain}, effects: &[Effect::RevealTop {{ \
             player: PlayerRel::EventPlayer, count: {count}, filter: &Filter::Any, \
             if_matches: &[], otherwise: &[] }}] }}"
        );
        if !remember_revealed {
            chain.effects.push(match outer_cond {
                Some(c) => format!(
                    "Effect::IfCondition {{ condition: {c}, then: &[{reveal_loop}], otherwise: &[] }}"
                ),
                None => reveal_loop,
            });
            return self.chain_sub_ability(sub.map(str::to_string), chain);
        }
        let Some(sub_name) = sub else {
            return self.deny(format!("`PeekAndReveal` for multi-seat `{library}`"));
        };
        let (parley_effs, trailing) = self.try_parley_followup(sub_name, library)?;
        let mut all = vec![reveal_loop];
        all.extend(parley_effs);
        if let Some(c) = outer_cond {
            chain.effects.push(format!(
                "Effect::IfCondition {{ condition: {c}, then: &[{}], otherwise: &[] }}",
                all.join(", ")
            ));
        } else {
            chain.effects.extend(all);
        }
        self.chain_sub_ability(trailing, chain)
    }

    fn parley_valid_amount(&mut self, raw: &str, who: &str) -> Option<String> {
        let def = self.svars.get(raw.trim())?.trim().to_string();
        let (head, suffix) = def
            .split_once('/')
            .map_or((def.as_str(), ""), |(h, s)| (h, s));
        let valid = head.strip_prefix("Remembered$Valid ")?.trim();
        let filter_ref = if valid == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(valid, true)?;
            format!("&{}", self.body.filter_static("PARLEY", &expr))
        };
        let base =
            format!("Amount::ThisWayBy {{ what: ThisWay::Revealed({filter_ref}), who: {who} }}");
        Self::wrap_amount_suffix(base, suffix)
    }

    #[allow(clippy::too_many_lines)]
    fn try_parley_followup(
        &mut self,
        start_svar: &str,
        reveal_who: &str,
    ) -> Option<(Vec<String>, Option<String>)> {
        let mut out = Vec::new();
        let mut cursor = Some(start_svar.trim().to_string());
        let mut used_remembered = false;
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 6 {
                return None;
            }
            if self.is_wave8_cleanup_svar(&name, false) {
                break;
            }
            let body = self.svars.get(&name)?.clone();
            let (api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if sp.has("ValidTgts") {
                return None;
            }
            let next = sp.take("SubAbility");
            if api == "Draw"
                && sp.peek("Defined") == Some("Player")
                && matches!(sp.peek("NumCards"), None | Some("1"))
                && !used_remembered
            {
                sp.take("Defined");
                sp.take("NumCards");
                if !sp.exhausted() {
                    return None;
                }
                out.push(
                    "Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::EachPlayer }"
                        .to_string(),
                );
                cursor = next;
                continue;
            }
            let eff = match api.as_str() {
                "PutCounter" => {
                    if !matches!(sp.take("Defined").as_deref(), None | Some("Self"))
                        || self.on_a_spell
                    {
                        return None;
                    }
                    let code = sp.take("CounterType")?;
                    let kind = super::counter_kind(&code)?;
                    let amt = self.parley_valid_amount(&sp.take("CounterNum")?, reveal_who)?;
                    if !sp.exhausted() {
                        return None;
                    }
                    format!(
                        "Effect::AddCounterAmount {{ target: TargetSpec::ThisObject, kind: {kind}, amount: {amt} }}"
                    )
                }
                "GainLife" => {
                    let who = self.player_rel(sp.take("Defined").as_deref())?;
                    let amt = self.parley_valid_amount(&sp.take("LifeAmount")?, reveal_who)?;
                    if !sp.exhausted() {
                        return None;
                    }
                    if who == "PlayerRel::You" {
                        format!("Effect::GainLife {{ amount: {amt} }}")
                    } else {
                        format!("Effect::GainLifeFor {{ amount: {amt}, target: {who} }}")
                    }
                }
                "LoseLife" => {
                    let who = self.player_rel(sp.take("Defined").as_deref())?;
                    let amt = self.parley_valid_amount(&sp.take("LifeAmount")?, reveal_who)?;
                    if !sp.exhausted() {
                        return None;
                    }
                    format!("Effect::LoseLife {{ amount: {amt}, target: {who} }}")
                }
                "DealDamage" => {
                    let who = self.player_rel(sp.take("Defined").as_deref())?;
                    let amt = self.parley_valid_amount(&sp.take("NumDmg")?, reveal_who)?;
                    if !sp.exhausted() {
                        return None;
                    }
                    format!(
                        "Effect::DealDamage {{ amount: {amt}, target: TargetSpec::Player({who}) }}"
                    )
                }
                "Token" => {
                    let who = self.player_rel(sp.take("TokenOwner").as_deref())?;
                    let stem = sp.take("TokenScript")?;
                    let amt = self.parley_valid_amount(&sp.take("TokenAmount")?, reveal_who)?;
                    if !sp.exhausted() {
                        return None;
                    }
                    let body = self.tokens?.body(&stem, self.cats)?;
                    let token = format!("&generated_tokens::{}", body.constant);
                    if who == "PlayerRel::You" {
                        format!("Effect::CreateTokenN {{ token: {token}, amount: {amt} }}")
                    } else {
                        format!(
                            "Effect::CreateTokensFor {{ player: {who}, token: {token}, amount: {amt} }}"
                        )
                    }
                }
                _ => return None,
            };
            used_remembered = true;
            out.push(eff);
            cursor = next;
        }
        if !used_remembered {
            return None;
        }
        Some((out, None))
    }

    fn peek_optional_top_one(
        &mut self,
        library: &str,
        reveal_valid: Option<&str>,
        sub_name: &str,
    ) -> Option<String> {
        let filter_ref = match reveal_valid {
            None | Some("Card") => "&Filter::Any".to_string(),
            Some(raw) => {
                let expr = self.filter_expr_in_zone(raw, true)?;
                format!("&{}", self.body.filter_static("PEEK", &expr))
            }
        };
        let Some(body) = self.svars.get(sub_name.trim()).cloned() else {
            return self.deny(format!("`SubAbility$ {sub_name}` names no SVar"));
        };
        if let Some((picked, rest)) = self.try_peek_top_one_move(&body, library, reveal_valid) {
            return Some(format!(
                "Effect::PeekAndReveal {{ library: {library}, count: Amount::Fixed(1), \
                 reveal_all: false, reveal_chosen: true, filter: {filter_ref}, \
                 min_pick: 0, max_pick: 1, picked: {picked}, rest: {rest}, if_picked: &[] }}"
            ));
        }
        // Otherwise require every non-cleanup step to be guarded by `ConditionDefined$ Remembered`.
        let mut cursor = Some(sub_name.trim().to_string());
        let mut if_picked = Vec::new();
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 12 {
                return self.deny("a cyclic `PeekAndReveal` chain".to_string());
            }
            if self.is_wave8_cleanup_svar(&name, false) {
                break;
            }
            let Some(step_body) = self.svars.get(&name).cloned() else {
                return self.deny(format!("`SubAbility$ {name}` names no SVar"));
            };
            let (_, sp_check) = Params::parse(&step_body)?;
            if sp_check.peek("ConditionDefined") != Some("Remembered") {
                return self.deny(
                    "`PeekAndReveal` `RevealOptional` follow-up without `ConditionDefined$ Remembered`"
                        .to_string(),
                );
            }
            let (mut effs, next) = self.compile_wave8_scoped_step(
                &step_body,
                Wave8Scope::RevealedObject,
                reveal_valid,
                false,
            )?;
            if_picked.append(&mut effs);
            cursor = next;
        }
        if if_picked.is_empty() {
            return None;
        }
        Some(format!(
            "Effect::PeekAndReveal {{ library: {library}, count: Amount::Fixed(1), \
             reveal_all: false, reveal_chosen: true, filter: {filter_ref}, \
             min_pick: 0, max_pick: 1, picked: PeekPickDest::StayOnTop, \
             rest: PeekRestDest::StayInOrder, if_picked: &[{}] }}",
            if_picked.join(", ")
        ))
    }

    fn try_peek_top_one_move(
        &mut self,
        body: &str,
        library: &str,
        reveal_valid: Option<&str>,
    ) -> Option<(String, String)> {
        let (api, mut sp) = Params::parse(body)?;
        sp.drop_prose();
        let def = sp.take("Defined");
        if api != "ChangeZone"
            || !matches!(def.as_deref(), Some("TopOfLibrary" | "Remembered"))
            || sp.take("Origin").as_deref() != Some("Library")
        {
            return None;
        }
        let guarded = Self::strip_matching_remembered_guard(&mut sp, "Remembered", reveal_valid);
        if !guarded && def.as_deref() != Some("Remembered") {
            return None;
        }
        let next = sp.take("SubAbility");
        let picked = Self::parse_peek_pick_dest(&mut sp, library)?;
        if !sp.exhausted() {
            return None;
        }
        let rest = match next {
            None => "PeekRestDest::StayInOrder".to_string(),
            Some(ref name) if self.is_wave8_cleanup_svar(name, false) => {
                "PeekRestDest::StayInOrder".to_string()
            }
            Some(else_name) => {
                let else_body = self.svars.get(else_name.trim())?.clone();
                let (e_api, mut ep) = Params::parse(&else_body)?;
                ep.drop_prose();
                if e_api != "ChangeZone"
                    || !matches!(
                        ep.take("Defined").as_deref(),
                        Some("TopOfLibrary" | "Remembered")
                    )
                    || ep.take("Origin").as_deref() != Some("Library")
                    || ep.take("ConditionDefined").as_deref() != Some("Remembered")
                    || ep.take("ConditionCompare").as_deref() != Some("EQ0")
                {
                    return None;
                }
                let present = ep.take("ConditionPresent")?;
                if present != "Card" && reveal_valid != Some(present.as_str()) {
                    return None;
                }
                ep.take("ConditionDescription");
                let after = ep.take("SubAbility");
                if let Some(ref clean) = after
                    && !self.is_wave8_cleanup_svar(clean, false)
                {
                    return None;
                }
                let rest_dest = Self::parse_peek_rest_dest(&mut ep)?;
                if !ep.exhausted() {
                    return None;
                }
                rest_dest
            }
        };
        Some((picked, rest))
    }

    fn parse_peek_pick_dest(sp: &mut Params, library: &str) -> Option<String> {
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
        match sp.take("Destination").as_deref() {
            Some("Hand") if !tapped && !gain => Some("PeekPickDest::Hand".to_string()),
            Some("Graveyard") if !tapped && !gain => Some("PeekPickDest::Graveyard".to_string()),
            Some("Exile") if !tapped && !gain => Some("PeekPickDest::Exile".to_string()),
            Some("Battlefield") => {
                if library != "PlayerRel::You" && !gain {
                    return None;
                }
                Some(format!("PeekPickDest::Battlefield {{ tapped: {tapped} }}"))
            }
            Some("Library")
                if !tapped && !gain && sp.take("LibraryPosition").as_deref() == Some("0") =>
            {
                Some("PeekPickDest::TopOfLibrary".to_string())
            }
            _ => None,
        }
    }

    fn parse_peek_rest_dest(ep: &mut Params) -> Option<String> {
        match ep.take("Destination").as_deref() {
            Some("Graveyard") => Some("PeekRestDest::Graveyard".to_string()),
            Some("Exile") => Some("PeekRestDest::Exile".to_string()),
            Some("Library") => match ep.take("LibraryPosition").as_deref() {
                Some("-1") => {
                    let random = match ep.take("RandomOrder").as_deref() {
                        None | Some("False") => false,
                        Some("True") => true,
                        Some(_) => return None,
                    };
                    Some(format!(
                        "PeekRestDest::BottomOfLibrary {{ random: {random} }}"
                    ))
                }
                Some("0") => Some("PeekRestDest::TopAnyOrder".to_string()),
                _ => None,
            },
            _ => None,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn peek_public_reveal_top(
        &mut self,
        library: &str,
        count: &str,
        remember_revealed: bool,
        sub: Option<&str>,
    ) -> Option<(String, Option<String>)> {
        if !remember_revealed {
            return Some((
                format!(
                    "Effect::RevealTop {{ player: {library}, count: {count}, \
                     filter: &Filter::Any, if_matches: &[], otherwise: &[] }}"
                ),
                sub.map(str::to_string),
            ));
        }
        let Some(sub_name) = sub else {
            return self
                .deny("`PeekAndReveal` `RememberRevealed` with no `SubAbility$`".to_string());
        };
        // First try multi-card pick + rest (e.g. reveal top N, pick up to K, rest to bottom/GY).
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics = self.body.statics.len();
        if let Some(eff) = self.peek_multi_pick_rest(library, count, true, sub_name) {
            return Some((eff, None));
        }
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        self.body.statics.truncate(saved_statics);

        // Next try conditional `RevealTop` branches (`ConditionDefined$ Remembered`).
        let mut cursor = Some(sub_name.trim().to_string());
        let mut filter_raw: Option<String> = None;
        let mut if_matches = Vec::new();
        let mut otherwise = Vec::new();
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 12 {
                return self.deny("a cyclic `RevealTop` chain".to_string());
            }
            if self.is_wave8_cleanup_svar(&name, false) {
                break;
            }
            let Some(body) = self.svars.get(&name).cloned() else {
                return self.deny(format!("`SubAbility$ {name}` names no SVar"));
            };
            let (api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if sp.peek("ConditionDefined") != Some("Remembered") {
                if count == "Amount::Fixed(1)" && filter_raw.is_none() && steps == 1 {
                    let next = sp.take("SubAbility");
                    if api == "ChangeZone"
                        && sp.take("Defined").as_deref() == Some("Remembered")
                        && sp.take("Origin").as_deref() == Some("Library")
                        && let Some(picked) = Self::parse_peek_pick_dest(&mut sp, library)
                        && sp.exhausted()
                        && next
                            .as_deref()
                            .is_none_or(|n| self.is_wave8_cleanup_svar(n, false))
                    {
                        return Some((
                            format!(
                                "Effect::PeekAndReveal {{ library: {library}, count: Amount::Fixed(1), \
                                 reveal_all: true, reveal_chosen: false, filter: &Filter::Any, \
                                 min_pick: 1, max_pick: 1, picked: {picked}, \
                                 rest: PeekRestDest::StayInOrder, if_picked: &[] }}"
                            ),
                            None,
                        ));
                    }
                }
                return self.deny("`PeekAndReveal` `NoPeek` follow-up shape".to_string());
            }
            let Some(present) = sp.peek("ConditionPresent").map(str::to_string) else {
                return self.deny("`RevealTop` branch with no `ConditionPresent$`".to_string());
            };
            let cmp = sp.peek("ConditionCompare").map(str::to_string);
            let is_then = match cmp.as_deref() {
                None | Some("GE1") => true,
                Some("EQ1") if count == "Amount::Fixed(1)" => true,
                Some("EQ0") => false,
                Some(other) => {
                    return self.deny(format!("`RevealTop` `ConditionCompare$ {other}`"));
                }
            };
            let target_branch = match (&filter_raw, is_then) {
                (None, true) => {
                    filter_raw = Some(present.clone());
                    true
                }
                (Some(prev), true) if prev == &present => true,
                (Some(prev), false) if prev == &present => false,
                (Some(prev), true)
                    if count == "Amount::Fixed(1)"
                        && matches!(
                            (prev.as_str(), present.as_str()),
                            ("Card.Land" | "Land", "Card.nonLand")
                                | ("Card.nonLand", "Card.Land" | "Land")
                        ) =>
                {
                    false
                }
                _ => return self.deny("`RevealTop` branches with differing filters".to_string()),
            };
            sp.take("ConditionDefined");
            sp.take("ConditionPresent");
            sp.take("ConditionCompare");
            sp.take("ConditionDescription");
            let next_sub = sp.take("SubAbility");
            let stripped_spec = Self::reconstruct_spec(&api, &sp);
            let (mut effs, _) = self.compile_wave8_scoped_step(
                &stripped_spec,
                Wave8Scope::RevealedObject,
                None,
                false,
            )?;
            if target_branch {
                if_matches.append(&mut effs);
            } else {
                otherwise.append(&mut effs);
            }
            cursor = next_sub;
        }
        let raw = filter_raw?;
        let filter_ref = if raw == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&raw, true)?;
            format!("&{}", self.body.filter_static("REVEAL_TOP", &expr))
        };
        Some((
            format!(
                "Effect::RevealTop {{ player: {library}, count: {count}, filter: {filter_ref}, \
                 if_matches: &[{}], otherwise: &[{}] }}",
                if_matches.join(", "),
                otherwise.join(", ")
            ),
            None,
        ))
    }

    fn peek_multi_pick_rest(
        &mut self,
        library: &str,
        count: &str,
        reveal_all: bool,
        sub_name: &str,
    ) -> Option<String> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if api != "ChangeZone"
            || sp.take("Origin").as_deref() != Some("Library")
            || sp.take("ForgetChanged").as_deref() != Some("True")
        {
            return None;
        }
        sp.take("SelectPrompt");
        if let Some(shuf) = sp.take("Shuffle")
            && shuf != "False"
        {
            return None;
        }
        let change_type = sp.take("ChangeType")?;
        let stripped = Self::strip_remembered_filter_atom(&change_type)?;
        let filter_ref = if stripped == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&stripped, true)?;
            format!("&{}", self.body.filter_static("PEEK", &expr))
        };
        let max_pick: u8 = sp
            .take("ChangeNum")
            .as_deref()
            .unwrap_or("1")
            .trim()
            .parse::<u8>()
            .ok()?;
        let mandatory = match sp.take("Mandatory").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let optional = match sp.take("Optional").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let min_pick = if mandatory && !optional { max_pick } else { 0 };
        let picked = Self::parse_peek_pick_dest(&mut sp, library)?;
        let rest_sub = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        let rest = match rest_sub {
            None => "PeekRestDest::StayInOrder".to_string(),
            Some(ref clean) if self.is_wave8_cleanup_svar(clean, false) => {
                "PeekRestDest::StayInOrder".to_string()
            }
            Some(rest_name) => {
                let rbody = self.svars.get(rest_name.trim())?.clone();
                let (rapi, mut rp) = Params::parse(&rbody)?;
                rp.drop_prose();
                if rapi != "ChangeZoneAll"
                    || rp.take("Origin").as_deref() != Some("Library")
                    || !matches!(rp.take("ChangeType").as_deref(), Some("Card.IsRemembered"))
                {
                    return None;
                }
                if let Some(shuf) = rp.take("Shuffle")
                    && shuf != "False"
                {
                    return None;
                }
                let after = rp.take("SubAbility");
                if let Some(ref clean) = after
                    && !self.is_wave8_cleanup_svar(clean, false)
                {
                    return None;
                }
                let dest = Self::parse_peek_rest_dest(&mut rp)?;
                if !rp.exhausted() {
                    return None;
                }
                dest
            }
        };
        let reveal_chosen = !reveal_all && filter_ref != "&Filter::Any";
        Some(format!(
            "Effect::PeekAndReveal {{ library: {library}, count: {count}, \
             reveal_all: {reveal_all}, reveal_chosen: {reveal_chosen}, filter: {filter_ref}, \
             min_pick: {min_pick}, max_pick: {max_pick}, picked: {picked}, rest: {rest}, \
             if_picked: &[] }}"
        ))
    }

    fn strip_remembered_filter_atom(raw: &str) -> Option<String> {
        let mut alts = Vec::new();
        for alt in raw.split(',') {
            let trimmed = alt.trim();
            let (noun, attrs) = trimmed.split_once('.')?;
            let mut kept = Vec::new();
            let mut found = false;
            for atom in attrs.split('+').map(str::trim) {
                if atom == "IsRemembered" {
                    found = true;
                } else if !atom.is_empty() {
                    kept.push(atom);
                }
            }
            if !found {
                return None;
            }
            if kept.is_empty() {
                alts.push(noun.trim().to_string());
            } else {
                alts.push(format!("{}.{}", noun.trim(), kept.join("+")));
            }
        }
        Some(alts.join(","))
    }

    // -------------------------------------------------------------------------
    // `Reveal` (hand) -> `Effect::RevealFromHand`
    // -------------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    pub(super) fn reveal_from_hand_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut super::Chain,
    ) -> Option<()> {
        let outer_cond = self.take_wave8_line_condition(&mut p).ok()?;
        let who = self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let any_number = match p.take("AnyNumber").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Reveal` with `AnyNumber$ {other}`")),
        };
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Reveal` with `Optional$ {other}`")),
        };
        let remember = match p.take("RememberRevealed").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Reveal` with `RememberRevealed$ {other}`")),
        };
        let raw_num = p.take("NumCards");
        if any_number && raw_num.is_some() {
            return self.deny("`Reveal` with both `AnyNumber$` and `NumCards$`".to_string());
        }
        let raw_valid = p.take("RevealValid");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Reveal.{key}`"));
        }
        let filter_ref = match raw_valid.as_deref() {
            None | Some("Card") => "&Filter::Any".to_string(),
            Some(raw) => {
                let cleaned = if who == "PlayerRel::You" {
                    Self::strip_your_hand_owner(raw)
                } else {
                    raw.to_string()
                };
                let expr = self.filter_expr_in_zone(&cleaned, true)?;
                format!("&{}", self.body.filter_static("REVEAL", &expr))
            }
        };
        let (min, max_expr) = if any_number {
            (0u8, "Amount::Fixed(255)".to_string())
        } else {
            let max = match raw_num.as_deref() {
                None => "Amount::Fixed(1)".to_string(),
                Some(r) => self.amount_or_count_ext(r)?,
            };
            let min = if optional {
                0
            } else {
                match raw_num.as_deref() {
                    None => 1,
                    Some(r) => r.trim().parse::<u8>().unwrap_or(u8::MAX),
                }
            };
            (min, max)
        };
        // Engine enforces `min = 0` on filtered hidden hands (CR 701.20a): refuse
        // scripts that require a mandatory filtered reveal.
        if min > 0 && filter_ref != "&Filter::Any" {
            return self.deny("mandatory filtered `Reveal` from a hidden hand".to_string());
        }
        if !remember {
            let reveal_eff = format!(
                "Effect::RevealFromHand {{ who: {who}, filter: {filter_ref}, \
                 min: {min}, max: {max_expr}, if_revealed: &[] }}"
            );
            chain.effects.push(match outer_cond {
                Some(c) => format!(
                    "Effect::IfCondition {{ condition: {c}, then: &[{reveal_eff}], otherwise: &[] }}"
                ),
                None => reveal_eff,
            });
            return self.chain_sub_ability(sub.map(str::to_string), chain);
        }
        let Some(sub_name) = sub else {
            return self.deny("`Reveal` `RememberRevealed` with no `SubAbility$`".to_string());
        };
        // Try counted `AnyNumber` follow-up (`Remembered$Amount` -> `ThisWay::Revealed`).
        if any_number
            && who == "PlayerRel::You"
            && let Some((followup_effs, trailing)) =
                self.try_reveal_counted_followup(sub_name, chain)
        {
            let reveal_eff = format!(
                "Effect::RevealFromHand {{ who: {who}, filter: {filter_ref}, \
                 min: {min}, max: {max_expr}, if_revealed: &[] }}"
            );
            if let Some(c) = outer_cond {
                let mut all = vec![reveal_eff];
                all.extend(followup_effs);
                chain.effects.push(format!(
                    "Effect::IfCondition {{ condition: {c}, then: &[{}], otherwise: &[] }}",
                    all.join(", ")
                ));
            } else {
                chain.effects.push(reveal_eff);
                chain.effects.extend(followup_effs);
            }
            return self.chain_sub_ability(trailing, chain);
        }
        // Otherwise require every non-cleanup follow-up step to check `ConditionDefined$ Remembered`.
        let mut cursor = Some(sub_name.trim().to_string());
        let mut if_revealed = Vec::new();
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 12 {
                return self.deny("a cyclic `Reveal` chain".to_string());
            }
            if self.is_wave8_cleanup_svar(&name, false) {
                break;
            }
            let Some(step_body) = self.svars.get(&name).cloned() else {
                return self.deny(format!("`SubAbility$ {name}` names no SVar"));
            };
            let (_, sp_check) = Params::parse(&step_body)?;
            if sp_check.peek("ConditionDefined") != Some("Remembered") {
                return self.deny(
                    "`Reveal` `RememberRevealed` follow-up without `ConditionDefined$ Remembered`"
                        .to_string(),
                );
            }
            let (mut effs, next) = self.compile_wave8_scoped_step(
                &step_body,
                Wave8Scope::RevealedObject,
                raw_valid.as_deref(),
                false,
            )?;
            if_revealed.append(&mut effs);
            cursor = next;
        }
        if if_revealed.is_empty() {
            return None;
        }
        if (chain.target.is_some() || chain.second_target.is_some())
            && if_revealed.iter().any(|e| e.contains("Filter::Reference"))
        {
            return self.deny("`Reveal` `Filter::Reference` beside an outer target".to_string());
        }
        let reveal_eff = format!(
            "Effect::RevealFromHand {{ who: {who}, filter: {filter_ref}, \
             min: {min}, max: {max_expr}, if_revealed: &[{}] }}",
            if_revealed.join(", ")
        );
        chain.effects.push(match outer_cond {
            Some(c) => format!(
                "Effect::IfCondition {{ condition: {c}, then: &[{reveal_eff}], otherwise: &[] }}"
            ),
            None => reveal_eff,
        });
        Some(())
    }

    fn strip_your_hand_owner(raw: &str) -> String {
        raw.split(',')
            .map(|alt| {
                let trimmed = alt.trim();
                trimmed
                    .strip_suffix("+YouOwn")
                    .or_else(|| trimmed.strip_suffix(".YouOwn"))
                    .or_else(|| trimmed.strip_suffix("+YouCtrl"))
                    .or_else(|| trimmed.strip_suffix(".YouCtrl"))
                    .unwrap_or(trimmed)
                    .trim()
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    fn revealed_count_amount(&self, raw: &str) -> Option<(String, bool)> {
        let trimmed = raw.trim();
        let neg = trimmed.starts_with('-');
        let key = trimmed.trim_start_matches(['+', '-']);
        let def = self.svars.get(key)?.trim();
        let (head, suffix) = def.split_once('/').map_or((def, ""), |(h, s)| (h, s));
        if head != "Remembered$Amount" {
            return None;
        }
        let base = "Amount::ThisWay(ThisWay::Revealed(&Filter::Any))".to_string();
        let mag = Self::wrap_amount_suffix(base, suffix)?;
        Some((mag, neg))
    }

    #[allow(clippy::too_many_lines)]
    fn try_reveal_counted_followup(
        &mut self,
        sub_name: &str,
        chain: &mut super::Chain,
    ) -> Option<(Vec<String>, Option<String>)> {
        let body = self.svars.get(sub_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        let next = sp.take("SubAbility");
        let trailing = match next {
            Some(ref clean) if self.is_wave8_cleanup_svar(clean, false) => None,
            other => other,
        };
        let saved_target = chain.target.clone();
        sp.claim_label("ValidTgtsDesc", "ValidTgts");
        let targets_here = if let Some(valid) = sp.take("ValidTgts") {
            if self.chain_target(&api, &mut sp, &valid, chain).is_none() {
                chain.target = saved_target;
                return None;
            }
            true
        } else {
            false
        };
        let target = Self::effect_target(chain);
        let targets_a_player =
            targets_here && target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
        let eff = match api.as_str() {
            "GainLife" => {
                let raw = sp.take("LifeAmount")?;
                let (amt, neg) = self.revealed_count_amount(&raw)?;
                let who = self.player_of_line(
                    sp.take("Defined").as_deref(),
                    target.as_deref(),
                    targets_a_player,
                )?;
                if neg || !sp.exhausted() {
                    chain.target = saved_target;
                    return None;
                }
                if who == "PlayerRel::You" {
                    format!("Effect::GainLife {{ amount: {amt} }}")
                } else {
                    format!("Effect::GainLifeFor {{ amount: {amt}, target: {who} }}")
                }
            }
            "LoseLife" => {
                let raw = sp.take("LifeAmount")?;
                let (amt, neg) = self.revealed_count_amount(&raw)?;
                let who = self.player_of_line(
                    sp.take("Defined").as_deref(),
                    target.as_deref(),
                    targets_a_player,
                )?;
                if neg || !sp.exhausted() {
                    chain.target = saved_target;
                    return None;
                }
                format!("Effect::LoseLife {{ amount: {amt}, target: {who} }}")
            }
            "DealDamage" => {
                let raw = sp.take("NumDmg")?;
                let (amt, neg) = self.revealed_count_amount(&raw)?;
                if neg || sp.take("Defined").is_some() || !sp.exhausted() {
                    chain.target = saved_target;
                    return None;
                }
                let tgt = target.as_deref()?;
                format!("Effect::DealDamage {{ amount: {amt}, target: {tgt} }}")
            }
            "Pump" => {
                let att_raw = sp.take("NumAtt");
                let def_raw = sp.take("NumDef");
                let duration = match sp.take("Duration").as_deref() {
                    None => "Duration::UntilEndOfTurn",
                    Some("Permanent") => "Duration::Indefinitely",
                    Some(_) => {
                        chain.target = saved_target;
                        return None;
                    }
                };
                let defined = sp.take("Defined");
                let subject_ok = match (defined.as_deref(), target.as_deref()) {
                    (None | Some("Targeted"), Some(t)) if t.starts_with("TargetSpec::Object(") => {
                        true
                    }
                    (None | Some("Self"), None) if !self.on_a_spell => true,
                    _ => false,
                };
                if !subject_ok || !sp.exhausted() {
                    chain.target = saved_target;
                    return None;
                }
                let mut side = |raw: Option<&str>| -> Option<String> {
                    match raw {
                        None => Some("Amount::Fixed(0)".to_string()),
                        Some(r) => {
                            if let Some((mag, neg)) = self.revealed_count_amount(r) {
                                Some(if neg {
                                    format!("Amount::Negated(&{mag})")
                                } else {
                                    mag
                                })
                            } else {
                                self.amount_or_count_ext(r)
                            }
                        }
                    }
                };
                let att = side(att_raw.as_deref())?;
                let def = side(def_raw.as_deref())?;
                if att == "Amount::Fixed(0)" && def == "Amount::Fixed(0)" {
                    chain.target = saved_target;
                    return None;
                }
                format!(
                    "Effect::PumpAmount {{ filter: &Filter::This, power: {att}, toughness: {def}, duration: {duration} }}"
                )
            }
            _ => {
                chain.target = saved_target;
                return None;
            }
        };
        Some((vec![eff], trailing))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Wave8Scope {
    RepeatPlayer,
    RepeatPermanent,
    ChosenObject,
    RevealedObject,
    Times,
}
