//! Reading the effects that are more than one line: tokens, animation,
//! pumps, zone changes and library searches.

use super::{
    CLUE_TOKEN_SCRIPT, Chain, Params, Tx, card_type_const, color_word, keyword_const, pump_amount,
};
use baylee_core::types::SubtypeKind;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EffectSubject {
    Unbound,
    Targets,
    Source,
    Event,
    Player(&'static str),
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
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if who == "PlayerRel::ControllerOfTarget"
            && self
                .oracle
                .to_ascii_lowercase()
                .contains("sacrifices another ")
        {
            return self.deny("`Sacrifice` excluding the targeted permanent".to_string());
        }
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
            other => match self.wave11_player_of_line(other, target, targets_a_player) {
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
                "Wither" => "KeywordSet::WITHER",
                "Horsemanship" => "KeywordSet::HORSEMANSHIP",
                "Skulk" => "KeywordSet::SKULK",
                "Enlist" => "KeywordSet::ENLIST",
                "Read ahead" => "KeywordSet::READ_AHEAD",
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
        if !second
            && p.peek("PumpZone") == Some("Graveyard")
            && p.peek("KW").map(str::trim) == Some("Flashback")
            && !p.has("NumAtt")
            && !p.has("NumDef")
            && target.starts_with("TargetSpec::CardInGraveyard(")
        {
            if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn"))
                || !matches!(
                    p.take("Defined").as_deref(),
                    None | Some("Targeted" | "ParentTarget")
                )
            {
                return None;
            }
            p.take("PumpZone");
            p.take("KW");
            p.take("IsCurse");
            return Some(vec!["Effect::GrantFlashback".to_string()]);
        }
        if !second
            && !p.has("DefinedKW")
            && !p.has("NumAtt")
            && !p.has("NumDef")
            && p.peek("KW")
                .is_some_and(|kw| kw.trim().starts_with("Protection:"))
        {
            return self.pump_protection_effect(p, target);
        }
        let power = self.pump_side_ext(p.take("NumAtt").as_deref().unwrap_or("0"))?;
        let toughness = self.pump_side_ext(p.take("NumDef").as_deref().unwrap_or("0"))?;
        let (keywords, prevent_mods) = match p.take("KW") {
            None => ("KeywordSet::EMPTY".to_string(), Vec::new()),
            Some(kw) => Self::pump_kw_and_prevent(&kw)?,
        };
        let kw_choices = match p.take("KWChoice") {
            None => None,
            Some(_) if keywords != "KeywordSet::EMPTY" || !prevent_mods.is_empty() => return None,
            Some(raw) => {
                let mut opts = Vec::new();
                for part in raw.split(',').map(str::trim) {
                    if part.is_empty() {
                        return None;
                    }
                    opts.push(Self::pump_keyword_bits(part)?);
                }
                if opts.len() < 2 {
                    return None;
                }
                Some(opts)
            }
        };
        // Every duration but the default is a lifetime the DSL spells
        // differently; none of them is "until end of turn" with a longer
        // name.
        let duration = match p.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some("Permanent")
                if power == "Amount::Fixed(0)"
                    && toughness == "Amount::Fixed(0)"
                    && prevent_mods.is_empty()
                    && (keywords != "KeywordSet::EMPTY" || kw_choices.is_some()) =>
            {
                "Duration::Indefinitely"
            }
            Some("UntilUntaps") if !self.on_a_spell && prevent_mods.is_empty() => {
                "Duration::WhileSourceTapped { control: false, power_limited: false }"
            }
            Some("AsLongAsControl") if !self.on_a_spell && prevent_mods.is_empty() => {
                "Duration::WhileYouControlSource"
            }
            Some(_) => return None,
        };
        if !prevent_mods.is_empty() && second {
            return None;
        }
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
        let has_pump = power != "Amount::Fixed(0)"
            || toughness != "Amount::Fixed(0)"
            || keywords != "KeywordSet::EMPTY";
        let empty = !has_pump && kw_choices.is_none() && prevent_mods.is_empty();
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
                ) && matches!(
                    self.trigger_mode.as_deref(),
                    Some(
                        "ChangesZone"
                            | "Exerted"
                            | "Enlisted"
                            | "Sacrificed"
                            | "Taps"
                            | "Untaps"
                            | "TurnFaceUp"
                            | "Transformed"
                    )
                ))
                || (matches!(
                    defined.as_deref(),
                    Some("TriggeredTargetLKICopy" | "TriggeredTarget")
                ) && matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce")
                ))
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
            if let Some(opts) = kw_choices {
                let rendered: Vec<String> = opts
                    .into_iter()
                    .map(|kw| {
                        format!(
                            "Effect::PumpEventObject {{ power: {power}, toughness: {toughness}, \
                             keywords: {kw}, duration: {duration} }}"
                        )
                    })
                    .collect();
                return Some(vec![format!(
                    "Effect::ChooseOne {{ choices: &[{}] }}",
                    rendered.join(", ")
                )]);
            }
            let mut out = Vec::new();
            if has_pump || prevent_mods.is_empty() {
                out.push(format!(
                    "Effect::PumpEventObject {{ power: {power}, toughness: {toughness}, \
                     keywords: {keywords}, duration: {duration} }}"
                ));
            }
            for m in prevent_mods {
                out.push(format!(
                    "Effect::ModifyEvent {{ modifier: {m}, duration: {duration} }}"
                ));
            }
            return Some(out);
        }
        if !second
            && !self.on_a_spell
            && kw_choices.is_none()
            && matches!(
                defined.as_deref(),
                Some("Self & Targeted" | "Targeted & Self")
            )
            && target.starts_with("TargetSpec::Object(")
        {
            let mut out = Vec::new();
            if has_pump || prevent_mods.is_empty() {
                out.push(format!(
                    "Effect::PumpTarget {{ power: {power}, toughness: {toughness}, \
                     keywords: {keywords}, duration: {duration} }}"
                ));
                out.push(format!(
                    "Effect::PumpFilter {{ filter: &Filter::This, controlled_by: None, \
                     power: {power}, toughness: {toughness}, keywords: {keywords}, \
                     duration: {duration} }}"
                ));
            }
            for m in prevent_mods {
                out.push(format!(
                    "Effect::ModifyTargets {{ modifier: {m}, duration: {duration} }}"
                ));
                out.push(format!(
                    "Effect::ModifySource {{ modifier: {m}, duration: {duration} }}"
                ));
            }
            return Some(out);
        }
        // "Enchanted creature gets +1/+0 until end of turn" on an Aura's own
        // activated ability (Firebreathing): the object the source is
        // attached to, which `Filter::AttachedToBySource` binds to the
        // resolving ability's source.
        let whom: Option<(String, Option<&'static str>)> = match defined.as_deref() {
            Some("Self") => Some(("Filter::This".to_string(), Some("ModifySource"))),
            Some("Enchanted" | "Equipped") => Some((
                "Filter::AttachedToBySource".to_string(),
                Some("ModifyAttached"),
            )),
            Some("TriggeredAttackerLKICopy" | "TriggeredAttacker")
                if self.trigger_mode.as_deref() == Some("Attacks") =>
            {
                Some(("Filter::Reference".to_string(), Some("ModifyEvent")))
            }
            Some(raw)
                if !second
                    && target == "TargetSpec::AnyPlayer"
                    && prevent_mods.is_empty()
                    && raw.starts_with("Valid ") =>
            {
                let expr = self.filter_expr(raw["Valid ".len()..].trim())?;
                Some((self.body.filter_static("PUMP", &expr), None))
            }
            _ => None,
        };
        if let Some((whom, modify_variant)) = whom {
            if let Some(opts) = kw_choices {
                let rendered: Vec<String> = opts
                    .into_iter()
                    .map(|kw| {
                        format!(
                            "Effect::PumpFilter {{ filter: &{whom}, controlled_by: None, \
                             power: {power}, toughness: {toughness}, keywords: {kw}, \
                             duration: {duration} }}"
                        )
                    })
                    .collect();
                return Some(vec![format!(
                    "Effect::ChooseOne {{ choices: &[{}] }}",
                    rendered.join(", ")
                )]);
            }
            let mut out = Vec::new();
            if has_pump || prevent_mods.is_empty() {
                out.push(format!(
                    "Effect::PumpFilter {{ filter: &{whom}, controlled_by: None, \
                     power: {power}, toughness: {toughness}, keywords: {keywords}, \
                     duration: {duration} }}"
                ));
            }
            if !prevent_mods.is_empty() {
                let variant = modify_variant?;
                for m in prevent_mods {
                    out.push(format!(
                        "Effect::{variant} {{ modifier: {m}, duration: {duration} }}"
                    ));
                }
            }
            return Some(out);
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
                if let Some(opts) = kw_choices {
                    let rendered: Vec<String> = opts
                        .into_iter()
                        .map(|kw| {
                            format!(
                                "Effect::{kind} power: {power}, toughness: {toughness}, \
                                 keywords: {kw}, duration: {duration} }}"
                            )
                        })
                        .collect();
                    vec![format!(
                        "Effect::ChooseOne {{ choices: &[{}] }}",
                        rendered.join(", ")
                    )]
                } else {
                    let mut out = Vec::new();
                    if has_pump || prevent_mods.is_empty() {
                        out.push(format!(
                            "Effect::{kind} power: {power}, toughness: {toughness}, \
                             keywords: {keywords}, duration: {duration} }}"
                        ));
                    }
                    for m in prevent_mods {
                        out.push(format!(
                            "Effect::ModifyTargets {{ modifier: {m}, duration: {duration} }}"
                        ));
                    }
                    out
                }
            }
            Some("ParentTarget") if !second && target != "TargetSpec::AnyPlayer" => {
                if let Some(opts) = kw_choices {
                    let rendered: Vec<String> = opts
                        .into_iter()
                        .map(|kw| {
                            format!(
                                "Effect::PumpTarget {{ power: {power}, toughness: {toughness}, \
                                 keywords: {kw}, duration: {duration} }}"
                            )
                        })
                        .collect();
                    vec![format!(
                        "Effect::ChooseOne {{ choices: &[{}] }}",
                        rendered.join(", ")
                    )]
                } else {
                    let mut out = Vec::new();
                    if has_pump || prevent_mods.is_empty() {
                        out.push(format!(
                            "Effect::PumpTarget {{ power: {power}, toughness: {toughness}, \
                             keywords: {keywords}, duration: {duration} }}"
                        ));
                    }
                    for m in prevent_mods {
                        out.push(format!(
                            "Effect::ModifyTargets {{ modifier: {m}, duration: {duration} }}"
                        ));
                    }
                    out
                }
            }
            Some(_) => return None,
        })
    }

    fn pump_kw_and_prevent(kw: &str) -> Option<(String, Vec<&'static str>)> {
        let mut normal = Vec::new();
        let mut prevent = Vec::new();
        for part in kw.split('&').map(str::trim).filter(|s| !s.is_empty()) {
            match part {
                "Prevent all combat damage that would be dealt to CARDNAME." => {
                    prevent.push("Modifier::PreventDamageToIt");
                }
                "Prevent all combat damage that would be dealt by CARDNAME." => {
                    prevent.push("Modifier::PreventDamageFromIt");
                }
                "Prevent all combat damage that would be dealt to and dealt by CARDNAME." => {
                    prevent.push("Modifier::PreventDamageToIt");
                    prevent.push("Modifier::PreventDamageFromIt");
                }
                "Prevent all damage that would be dealt by CARDNAME." => {
                    prevent.push("Modifier::PreventAllDamageFromIt");
                }
                "Prevent all damage that would be dealt to CARDNAME." => {
                    prevent.push("Modifier::PreventDamageFrom(&Filter::Any)");
                }
                "Prevent all damage that would be dealt to and dealt by CARDNAME." => {
                    prevent.push("Modifier::PreventDamageFrom(&Filter::Any)");
                    prevent.push("Modifier::PreventAllDamageFromIt");
                }
                "HIDDEN All creatures able to block CARDNAME do so." => {
                    prevent.push("Modifier::MustBeBlockedByAllAble");
                }
                "HIDDEN CARDNAME attacks each combat if able." => {
                    prevent.push("Modifier::AttacksEachCombat");
                }
                "HIDDEN CARDNAME can block any number of creatures." => {
                    prevent.push("Modifier::CanBlockAnyNumber");
                }
                "HIDDEN CARDNAME can't be blocked except by two or more creatures." => {
                    prevent.push("Modifier::MinimumBlockers(2)");
                }
                "HIDDEN CARDNAME can block an additional creature each combat." => {
                    prevent.push("Modifier::CanBlockAdditional(1)");
                }
                "HIDDEN CARDNAME can attack as though it didn't have defender." => {
                    prevent.push("Modifier::AttacksDespiteDefender");
                }
                "HIDDEN CARDNAME can attack as though it had haste." => {
                    prevent.push("Modifier::AttacksAsThoughHaste");
                }
                "HIDDEN CARDNAME blocks each attacking creature if able." => {
                    prevent.push("Modifier::BlocksEachAttackerIfAble");
                }
                other => normal.push(other),
            }
        }
        let bits = if normal.is_empty() {
            "KeywordSet::EMPTY".to_string()
        } else {
            Self::pump_keyword_bits(&normal.join(" & "))?
        };
        Some((bits, prevent))
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
    fn pump_protection_spec(&mut self, kw: &str) -> Option<String> {
        let rest = kw.trim().strip_prefix("Protection:")?;
        let spec = rest.split(':').next()?.trim();
        if spec.is_empty()
            || spec.contains('&')
            || spec.contains("Player")
            || spec.contains("Spell")
            || spec.contains("Chosen")
            || spec.contains("Remembered")
            || spec.contains("EffectSource")
            || spec.contains("Triggered")
        {
            return None;
        }
        let expr = self.filter_expr(spec)?;
        Some(self.body.filter_static("PROT", &expr))
    }

    fn pump_protection_effect(&mut self, p: &mut Params, target: &str) -> Option<Vec<String>> {
        let kw = p.take("KW")?;
        let from = self.pump_protection_spec(&kw)?;
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some(_) => return None,
        };
        p.take("IsCurse");
        let ctor = match p.take("Defined").as_deref() {
            None | Some("Targeted" | "ParentTarget")
                if target.starts_with("TargetSpec::Object(") =>
            {
                "ModifyTargets"
            }
            Some("Self") if !self.on_a_spell => "ModifySource",
            None if target == "TargetSpec::AnyPlayer" && !self.on_a_spell => "ModifySource",
            _ => return None,
        };
        Some(vec![format!(
            "Effect::{ctor} {{ modifier: Modifier::ProtectionFrom(&{from}), duration: {duration} }}"
        )])
    }

    fn pump_all_graveyard_flashback(&mut self, p: &mut Params) -> Option<Vec<String>> {
        if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn")) {
            return None;
        }
        p.take("PumpZone");
        p.take("KW");
        p.take("IsCurse");
        let valid = p.take("ValidCards")?;
        let mut rewritten = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() || alt.contains("OppCtrl") || alt.contains("OppOwn") {
                return None;
            }
            if !alt.contains("YouCtrl") && !alt.contains("YouOwn") {
                return None;
            }
            rewritten.push(alt.replace("YouCtrl", "YouOwn"));
        }
        let base = self.filter_expr_in_zone(&rewritten.join(","), true)?;
        let expr = format!("Filter::And(&[{base}, Filter::InZone(ZoneRef::Graveyard)])");
        let filter = self.body.filter_static("FLASHBACK", &expr);
        Some(vec![format!(
            "Effect::continuous(&{filter}, Modifier::GrantsFlashback, Duration::UntilEndOfTurn)"
        )])
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn pump_all_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        if p.peek("KW").map(str::trim)
            == Some("HIDDEN This card doesn't untap during your next untap step.")
            && p.peek("Duration") == Some("Permanent")
            && !p.has("NumAtt")
            && !p.has("NumDef")
            && !targets_here
            && (target.is_none() || prior_target_used)
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
        if p.peek("PumpZone") == Some("Graveyard")
            && p.peek("KW").map(str::trim) == Some("Flashback")
            && !p.has("NumAtt")
            && !p.has("NumDef")
            && !targets_here
            && (target.is_none() || prior_target_used)
            && p.peek("Defined").is_none()
        {
            return self.pump_all_graveyard_flashback(p);
        }
        if !p.has("DefinedKW")
            && !p.has("NumAtt")
            && !p.has("NumDef")
            && !targets_here
            && (target.is_none() || prior_target_used)
            && p.peek("Defined").is_none()
            && p.peek("KW")
                .is_some_and(|kw| kw.trim().starts_with("Protection:"))
        {
            let kw = p.take("KW")?;
            let from = self.pump_protection_spec(&kw)?;
            let valid = p.take("ValidCards")?;
            let duration = match p.take("Duration").as_deref() {
                None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
                Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
                Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
                Some("Permanent") => "Duration::Indefinitely",
                Some(_) => return None,
            };
            p.take("IsCurse");
            let expr = self.filter_expr(&valid)?;
            let filter = self.body.filter_static("PUMP_ALL", &expr);
            return Some(vec![format!(
                "Effect::continuous(&{filter}, Modifier::ProtectionFrom(&{from}), {duration})"
            )]);
        }
        let defined = p.take("Defined");
        let controlled_by = if target == Some("TargetSpec::Player(PlayerRel::Chosen)")
            && ((targets_a_player && matches!(defined.as_deref(), None | Some("Targeted")))
                || (!targets_here && defined.as_deref() == Some("Targeted")))
        {
            "Some(PlayerRel::Chosen)"
        } else if defined.is_none() && !targets_here && (target.is_none() || prior_target_used) {
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
        let (keywords, extra_mods) = match p.take("KW") {
            None => ("KeywordSet::EMPTY".to_string(), Vec::new()),
            Some(kw) => Self::pump_kw_and_prevent(&kw)?,
        };
        if !extra_mods.is_empty() && controlled_by != "None" {
            return None;
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some(_) => return None,
        };
        p.take("IsCurse");
        let has_pump = power != "Amount::Fixed(0)"
            || toughness != "Amount::Fixed(0)"
            || keywords != "KeywordSet::EMPTY";
        if !has_pump && extra_mods.is_empty() {
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
        let mut out = Vec::new();
        if has_pump {
            out.push(format!(
                "Effect::PumpFilter {{ filter: &{filter}, controlled_by: {controlled_by}, \
                 power: {power}, toughness: {toughness}, keywords: {keywords}, \
                 duration: {duration} }}"
            ));
        }
        for m in extra_mods {
            out.push(format!("Effect::continuous(&{filter}, {m}, {duration})"));
        }
        Some(out)
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
    pub(super) fn shuffle_into_library_with_prior(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
        prior_target_used: bool,
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
                let no_open_target = target.is_none() || prior_target_used;
                let player = match change_type.as_deref() {
                    Some("Card.YouOwn" | "Card.YouCtrl") if defined.is_none() && no_open_target => {
                        "PlayerRel::You"
                    }
                    Some("Card.OppOwn" | "Card.OppCtrl" | "Card.YouDontOwn")
                        if defined.is_none() && no_open_target =>
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
                        self.wave11_player_of_line(def, target, targets_a_player)?
                    }
                    Some("Card") if no_open_target => "PlayerRel::EachPlayer",
                    _ => return None,
                };
                return Some(vec![format!(
                    "Effect::ExileGraveyard {{ player: {player} }}"
                )]);
            }
            if (target.is_some() && !prior_target_used)
                || targets_a_player
                || p.take("Defined").is_some()
            {
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
                    if let Some(stripped) = Self::strip_your_graveyard_owner(&valid) {
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
            p.take("IsCurse");
            let defined = p.take("Defined");
            let change_type = p.take("ChangeType");
            let no_open_target = target.is_none() || prior_target_used;
            let player = match change_type.as_deref() {
                Some("Card.YouOwn") if defined.is_none() && no_open_target => "PlayerRel::You",
                Some("Card") | None if defined.is_some() || targets_a_player => {
                    self.wave11_player_of_line(defined.as_deref(), target, targets_a_player)?
                }
                Some("Card") if no_open_target => "PlayerRel::EachPlayer",
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
        // Putting cards at any position/order in a library and immediately
        // shuffling (`Shuffle$ True`) is equivalent to shuffling them in.
        if !matches!(
            p.take("LibraryPosition").as_deref(),
            None | Some("0" | "-1")
        ) {
            return None;
        }
        if !matches!(p.take("RandomOrder").as_deref(), None | Some("True")) {
            return None;
        }
        // The reference's "from every zone listed" rather than one of them:
        // what two origins mean on a card.
        if hand && graveyard {
            p.take("UseAllOriginZones");
        }
        p.take("IsCurse");
        let defined = p.take("Defined");
        let change_type = p.take("ChangeType");
        let no_open_target = target.is_none() || prior_target_used;
        if let Some(effects) = self.shuffle_matching_graveyard(
            !hand && graveyard && defined.is_none() && no_open_target,
            change_type.as_deref(),
        ) {
            return Some(effects);
        }
        let who = match change_type.as_deref() {
            Some("Card.YouOwn" | "Card.YouCtrl") if defined.is_none() && no_open_target => {
                "PlayerRel::You".to_string()
            }
            Some("Card") | None if defined.is_some() || targets_a_player => self
                .wave11_player_of_line(defined.as_deref(), target, targets_a_player)?
                .to_string(),
            Some("Card") if no_open_target => "PlayerRel::EachPlayer".to_string(),
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

    fn try_one_shot_effect_probe(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let snap = p.clone();
        let saved_unclaimed = self.unclaimed.borrow().clone();
        let saved_statics_len = self.body.statics.len();
        if let Some(effects) = self.try_reduce_next_spell_cost_effect(p, target) {
            return Some(effects);
        }
        p.clone_from(&snap);
        self.unclaimed.borrow_mut().clone_from(&saved_unclaimed);
        self.body.statics.truncate(saved_statics_len);
        if let Some(effects) = self.try_one_shot_spell_effect(p, target) {
            return Some(effects);
        }
        *p = snap;
        *self.unclaimed.borrow_mut() = saved_unclaimed;
        self.body.statics.truncate(saved_statics_len);
        None
    }

    /// Wave-3 extensions for `SP$/AB$/DB$ Effect` (called from `chains.rs`
    /// only after `static_effect` returns `None`).
    pub(super) fn wave3_effect_api(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if p.peek("Triggers").is_some() || p.peek("Abilities").is_some() {
            return None;
        }
        if p.peek("ReplacementEffects").is_some() && p.peek("StaticAbilities").is_none() {
            let snap = p.clone();
            if let Some(effects) = self.dont_untap_replacement_effect(p, target) {
                return Some(effects);
            }
            *p = snap;
        }
        if p.peek("StaticAbilities").is_some()
            && p.peek("ReplacementEffects").is_none()
            && let Some(effects) = self.try_one_shot_effect_probe(p, target)
        {
            return Some(effects);
        }
        let mut host_leaves_or_eot = false;
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilHostLeavesPlayOrEOT") if !self.on_a_spell => {
                host_leaves_or_eot = true;
                "Duration::UntilEndOfTurn"
            }
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
        p.take("Name");
        if let Some(non_stacking) = p.take("NonStackingEffect")
            && non_stacking != "True"
        {
            return None;
        }
        if let Some(stackable) = p.take("Stackable")
            && stackable != "False"
        {
            return None;
        }
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
        let statics = p.take("StaticAbilities");
        let replacements = p.take("ReplacementEffects");
        if statics.is_none() && replacements.is_none() {
            return None;
        }
        let mut out = Vec::new();
        if let Some(statics) = statics {
            for svar_name in statics.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let body = self.svars.get(svar_name)?.clone();
                let (mode, mut st) = Params::parse(&body)?;
                st.drop_prose();
                out.extend(self.wave3_static_effect_line(&mode, &mut st, subject, duration)?);
            }
        }
        if let Some(replacements) = replacements {
            for svar_name in replacements
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let body = self.svars.get(svar_name)?.clone();
                let (event, mut r) = Params::parse(&body)?;
                r.drop_prose();
                out.extend(self.wave11_effect_replacement_line(
                    &event,
                    &mut r,
                    subject,
                    host_leaves_or_eot,
                    duration,
                )?);
            }
        }
        if host_leaves_or_eot
            && (subject != EffectSubject::Unbound
                || !out.iter().all(|e| e.starts_with("Effect::ModifySource {")))
        {
            return None;
        }
        (!out.is_empty()).then_some(out)
    }

    fn strip_in_zone_battlefield(raw: &str) -> Option<(String, bool)> {
        let mut stripped_bf = false;
        let mut alts = Vec::new();
        for alt in raw.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (noun, rest) = match alt.split_once('.') {
                Some((n, r)) => (n.trim(), Some(r)),
                None => (alt, None),
            };
            if noun.starts_with("inZone") {
                return None;
            }
            if let Some(rest) = rest {
                let mut kept = Vec::new();
                for atom in rest.split('+').map(str::trim) {
                    if atom == "inZoneBattlefield" {
                        stripped_bf = true;
                    } else if atom.starts_with("inZone") {
                        return None;
                    } else if !atom.is_empty() {
                        kept.push(atom);
                    }
                }
                if kept.is_empty() {
                    alts.push(noun.to_string());
                } else {
                    alts.push(format!("{noun}.{}", kept.join("+")));
                }
            } else {
                alts.push(noun.to_string());
            }
        }
        Some((alts.join(","), stripped_bf))
    }

    #[allow(clippy::too_many_lines)]
    fn wave12_damage_prevention_replacement(
        &mut self,
        r: &mut Params,
        subject: EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
        if r.take("Prevent").as_deref() != Some("True") {
            return None;
        }
        if !matches!(r.take("ActiveZones").as_deref(), None | Some("Command")) {
            return None;
        }
        let (combat_only, noncombat_only) = match r.take("IsCombat").as_deref() {
            None => (false, false),
            Some("True") => (true, false),
            Some("False") => (false, true),
            Some(_) => return None,
        };
        let valid_source = r.take("ValidSource");
        let valid_target = r.take("ValidTarget");
        if !r.exhausted() {
            return None;
        }
        let Some(vt) = valid_target else {
            if noncombat_only {
                return None;
            }
            let vs = valid_source?;
            if matches!(
                vs.as_str(),
                "Card.IsRemembered"
                    | "Creature.IsRemembered"
                    | "Card.Self"
                    | "Creature.Self"
                    | "Card.EffectSource"
                    | "Creature.EffectSource"
            ) {
                let modifier = if combat_only {
                    "Modifier::PreventDamageFromIt"
                } else {
                    "Modifier::PreventAllDamageFromIt"
                };
                return self.emit_effect_modifiers(subject, &vs, &[modifier.to_string()], duration);
            }
            if subject != EffectSubject::Unbound
                || vs.contains("IsRemembered")
                || vs.contains("EffectSource")
                || vs.contains("RememberedPlayer")
                || vs.contains("ChosenCard")
            {
                return None;
            }
            let (vs_clean, stripped_bf) = Self::strip_in_zone_battlefield(&vs)?;
            let base = self.filter_expr(&vs_clean)?;
            let expr = if stripped_bf {
                format!("Filter::And(&[Filter::InZone(ZoneRef::Battlefield), {base}])")
            } else {
                base
            };
            let name = self.body.filter_static("EFFECT", &expr);
            let modifier = if combat_only {
                "Modifier::PreventDamageFromIt"
            } else {
                "Modifier::PreventAllDamageFromIt"
            };
            return Some(vec![format!(
                "Effect::continuous(&{name}, {modifier}, {duration})"
            )]);
        };
        let opt_src_expr = if let Some(vs) = valid_source {
            if vs.contains("IsRemembered")
                || vs.contains("EffectSource")
                || vs.contains("RememberedPlayer")
                || vs.contains("ChosenCard")
            {
                return None;
            }
            let (vs_clean, stripped_bf) = Self::strip_in_zone_battlefield(&vs)?;
            let base = self.filter_expr(&vs_clean)?;
            Some(if stripped_bf {
                format!("Filter::And(&[Filter::InZone(ZoneRef::Battlefield), {base}])")
            } else {
                base
            })
        } else {
            None
        };
        let mut player_rels = Vec::new();
        let mut perm_alts = Vec::new();
        for part in vt.split(',').map(str::trim) {
            if part.is_empty() {
                return None;
            }
            if part == "You" {
                player_rels.push("PlayerRel::You");
                continue;
            }
            if noncombat_only {
                match (part, subject) {
                    ("Player", EffectSubject::Unbound) => {
                        player_rels.push("PlayerRel::EachPlayer");
                        continue;
                    }
                    ("Opponent" | "Player.Opponent", EffectSubject::Unbound) => {
                        player_rels.push("PlayerRel::EachOpponent");
                        continue;
                    }
                    ("Player.IsRemembered", EffectSubject::Player(who)) => {
                        player_rels.push(who);
                        continue;
                    }
                    _ => {}
                }
            }
            if part
                .split(['.', '+'])
                .any(|atom| matches!(atom, "You" | "Player" | "Opponent"))
            {
                return None;
            }
            let (clean, _) = Self::strip_in_zone_battlefield(part)?;
            perm_alts.push(clean);
        }
        if !player_rels.is_empty() && combat_only {
            return None;
        }
        if !player_rels.is_empty() && !noncombat_only && subject != EffectSubject::Unbound {
            return None;
        }
        let src_ref = match &opt_src_expr {
            Some(expr) => format!("&{}", self.body.filter_static("SOURCE", expr)),
            None => "&Filter::Any".to_string(),
        };
        let mut out = Vec::new();
        for who in player_rels {
            if noncombat_only {
                out.push(format!(
                    "Effect::continuous(&Filter::Any, Modifier::PreventNoncombatDamageToPlayers \
                     {{ who: {who}, from: {src_ref} }}, {duration})"
                ));
            } else {
                out.push(format!(
                    "Effect::continuous(&Filter::Any, Modifier::PreventDamageToYouFrom({src_ref}), \
                     {duration})"
                ));
            }
        }
        if !perm_alts.is_empty() {
            let perm_valid = perm_alts.join(",");
            let modifier = match (opt_src_expr.is_some(), combat_only, noncombat_only) {
                (_, _, true) => format!("Modifier::PreventNoncombatDamageFrom({src_ref})"),
                (false, true, false) => "Modifier::PreventDamageToIt".to_string(),
                (false, false, false) => "Modifier::PreventDamageFrom(&Filter::Any)".to_string(),
                (true, true, false) => format!("Modifier::PreventCombatDamageFrom({src_ref})"),
                (true, false, false) => format!("Modifier::PreventDamageFrom({src_ref})"),
            };
            out.extend(self.emit_effect_modifiers(subject, &perm_valid, &[modifier], duration)?);
        }
        (!out.is_empty()).then_some(out)
    }

    #[allow(clippy::too_many_lines)]
    fn wave14_damage_modify_replacement(
        &mut self,
        r: &mut Params,
        subject: EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
        if r.has("Prevent") || r.has("PreventionEffect") || r.has("DamageTarget") {
            return None;
        }
        if !matches!(
            r.take("ActiveZones").as_deref(),
            None | Some("Command" | "Battlefield")
        ) {
            return None;
        }
        let (combat_only, noncombat_only) = match r.take("IsCombat").as_deref() {
            None => (false, false),
            Some("True") => (true, false),
            Some("False") => (false, true),
            Some(_) => return None,
        };
        let rep_name = r.take("ReplaceWith")?;
        let valid_source = r.take("ValidSource");
        let valid_target = r.take("ValidTarget");
        if !r.exhausted() {
            return None;
        }
        let (rep_api, mut rep) = Params::parse(self.svars.get(rep_name.trim())?)?;
        rep.drop_prose();
        if rep_api != "ReplaceEffect" || rep.take("VarName").as_deref() != Some("DamageAmount") {
            return None;
        }
        let raw_val = rep.take("VarValue")?;
        if !rep.exhausted() {
            return None;
        }
        let resolved_val = self
            .svars
            .get(raw_val.trim())
            .map_or(raw_val.trim(), String::as_str)
            .trim();
        let (is_mul, num): (bool, u8) = match resolved_val {
            "ReplaceCount$DamageAmount/Twice" => (true, 2),
            "ReplaceCount$DamageAmount/Thrice" => (true, 3),
            _ if resolved_val.starts_with("ReplaceCount$DamageAmount/Times.") => {
                let f = resolved_val.strip_prefix("ReplaceCount$DamageAmount/Times.")?;
                (true, f.trim().parse::<u8>().ok().filter(|&n| n >= 2)?)
            }
            _ if resolved_val.starts_with("ReplaceCount$DamageAmount/Plus.") => {
                let e = resolved_val.strip_prefix("ReplaceCount$DamageAmount/Plus.")?;
                (false, e.trim().parse::<u8>().ok().filter(|&n| n >= 1)?)
            }
            _ => return None,
        };
        let src_ref =
            match valid_source.as_deref() {
                None => "&Filter::Any".to_string(),
                Some(
                    "Card.Self" | "Creature.Self" | "Card.EffectSource" | "Creature.EffectSource",
                ) if !self.on_a_spell => "&Filter::This".to_string(),
                Some(vs)
                    if !vs.contains("IsRemembered")
                        && !vs.contains("EffectSource")
                        && !vs.contains("RememberedPlayer")
                        && !vs.contains("ChosenCard") =>
                {
                    let (vs_clean, stripped_bf) = Self::strip_in_zone_battlefield(vs)?;
                    let base = self.filter_expr(&vs_clean)?;
                    let expr = if stripped_bf {
                        format!("Filter::And(&[Filter::InZone(ZoneRef::Battlefield), {base}])")
                    } else {
                        base
                    };
                    format!("&{}", self.body.filter_static("SOURCE", &expr))
                }
                Some(_) => return None,
            };
        let build_mod = |recipients: &str| {
            if is_mul {
                format!(
                    "Modifier::MultiplyDamageFrom {{ sources: {src_ref}, factor: {num}, \
                     combat_only: {combat_only}, noncombat_only: {noncombat_only}, \
                     recipients: {recipients} }}"
                )
            } else {
                format!(
                    "Modifier::AddDamageFrom {{ sources: {src_ref}, extra: {num}, \
                     combat_only: {combat_only}, noncombat_only: {noncombat_only}, \
                     recipients: {recipients} }}"
                )
            }
        };
        match (valid_target.as_deref(), subject) {
            (
                None
                | Some("Permanent,Player" | "Player,Permanent" | "Card,Player" | "Player,Card"),
                EffectSubject::Unbound,
            ) => Some(vec![format!(
                "Effect::continuous(&Filter::Any, {}, {duration})",
                build_mod("DamageRecipients::Any")
            )]),
            (Some("Player"), EffectSubject::Unbound) => Some(vec![format!(
                "Effect::continuous(&Filter::Any, {}, {duration})",
                build_mod("DamageRecipients::Players(PlayerRel::EachPlayer)")
            )]),
            (Some("You"), EffectSubject::Unbound) => Some(vec![format!(
                "Effect::continuous(&Filter::Any, {}, {duration})",
                build_mod("DamageRecipients::Players(PlayerRel::You)")
            )]),
            (Some("Opponent" | "Player.Opponent"), EffectSubject::Unbound) => Some(vec![format!(
                "Effect::continuous(&Filter::Any, {}, {duration})",
                build_mod("DamageRecipients::Players(PlayerRel::EachOpponent)")
            )]),
            (Some("Player.IsRemembered"), EffectSubject::Player(who)) => Some(vec![format!(
                "Effect::continuous(&Filter::Any, {}, {duration})",
                build_mod(&format!("DamageRecipients::Players({who})"))
            )]),
            (
                Some("Player.Opponent,Permanent.OppCtrl" | "Permanent.OppCtrl,Player.Opponent"),
                EffectSubject::Unbound,
            ) => Some(vec![format!(
                "Effect::continuous(&Filter::Any, {}, {duration})",
                build_mod("DamageRecipients::PlayersOrTheirPermanents(PlayerRel::EachOpponent)")
            )]),
            (Some("You,Permanent.YouCtrl" | "Permanent.YouCtrl,You"), EffectSubject::Unbound) => {
                Some(vec![format!(
                    "Effect::continuous(&Filter::Any, {}, {duration})",
                    build_mod("DamageRecipients::PlayersOrTheirPermanents(PlayerRel::You)")
                )])
            }
            (
                Some(vt @ ("Card.IsRemembered" | "Creature.IsRemembered")),
                EffectSubject::Targets | EffectSubject::Source | EffectSubject::Event,
            ) => {
                let m = build_mod("DamageRecipients::Permanents(&Filter::This)");
                self.emit_effect_modifiers(subject, vt, &[m], duration)
            }
            (Some(vt), EffectSubject::Unbound)
                if !vt.contains("IsRemembered")
                    && !vt.contains("EffectSource")
                    && !vt.contains("RememberedPlayer")
                    && !vt.split(',').any(|p| {
                        p.trim()
                            .split(['.', '+'])
                            .any(|a| matches!(a, "You" | "Player" | "Opponent"))
                    }) =>
            {
                let (clean, _) = Self::strip_in_zone_battlefield(vt)?;
                let expr = self.filter_expr(&clean)?;
                let name = self.body.filter_static("EFFECT", &expr);
                let m = build_mod(&format!("DamageRecipients::Permanents(&{name})"));
                Some(vec![format!(
                    "Effect::continuous(&Filter::Any, {m}, {duration})"
                )])
            }
            _ => None,
        }
    }

    pub(super) fn wave11_effect_replacement_line(
        &mut self,
        event: &str,
        r: &mut Params,
        subject: EffectSubject,
        host_leaves_or_eot: bool,
        duration: &str,
    ) -> Option<Vec<String>> {
        match r.take("Secondary").as_deref() {
            None | Some("True") => {}
            Some(_) => return None,
        }
        if event == "DamageDone" {
            if r.peek("Prevent") == Some("True") {
                return self.wave12_damage_prevention_replacement(r, subject, duration);
            }
            if r.peek("ReplaceWith").is_some() {
                return self.wave14_damage_modify_replacement(r, subject, duration);
            }
            return None;
        }
        if subject != EffectSubject::Unbound || host_leaves_or_eot {
            return None;
        }
        match event {
            "GameLoss" => {
                if r.take("Layer").as_deref() != Some("CantHappen") {
                    return None;
                }
                let modifier = match r.take("ValidPlayer").as_deref() {
                    Some("You") => "Modifier::NamedPlayersCantLose(PlayerRel::You)",
                    Some("Player") => "Modifier::PlayersCantLose",
                    Some("Opponent" | "Player.Opponent") => {
                        "Modifier::NamedPlayersCantLose(PlayerRel::EachOpponent)"
                    }
                    _ => return None,
                };
                r.exhausted().then(|| {
                    vec![format!(
                        "Effect::continuous(&Filter::Any, {modifier}, {duration})"
                    )]
                })
            }
            "GameWin" => {
                if r.take("Layer").as_deref() != Some("CantHappen") {
                    return None;
                }
                let modifier = match r.take("ValidPlayer").as_deref() {
                    Some("Opponent" | "Player.Opponent") => {
                        "Modifier::PlayersCantWin(PlayerRel::EachOpponent)"
                    }
                    Some("Player") => "Modifier::PlayersCantWin(PlayerRel::EachPlayer)",
                    Some("You") => "Modifier::PlayersCantWin(PlayerRel::You)",
                    _ => return None,
                };
                r.exhausted().then(|| {
                    vec![format!(
                        "Effect::continuous(&Filter::Any, {modifier}, {duration})"
                    )]
                })
            }
            "LifeReduced" => {
                if r.take("IsDamage").as_deref() != Some("True") {
                    return None;
                }
                let min_p: i32 = r
                    .take("ValidPlayer")?
                    .strip_prefix("You.lifeGE")?
                    .parse()
                    .ok()?;
                let min_r: i32 = r.take("Result")?.strip_prefix("LT")?.parse().ok()?;
                if min_p != min_r || min_p < 1 {
                    return None;
                }
                let rep_name = r.take("ReplaceWith")?;
                if !r.exhausted() {
                    return None;
                }
                let (api, mut rep) = Params::parse(self.svars.get(rep_name.trim())?)?;
                rep.drop_prose();
                if api != "ReplaceEffect" || rep.take("VarName").as_deref() != Some("Amount") {
                    return None;
                }
                let var = rep.take("VarValue")?;
                if !rep.exhausted() {
                    return None;
                }
                let expr = self.svars.get(var.trim()).map(String::as_str)?.trim();
                let exact_diff = format!("Count$YourLifeTotal/Minus.{min_p}");
                let ok = expr == exact_diff
                    || expr
                        .strip_prefix("ReplaceCount$Amount/LimitMax.")
                        .and_then(|d| self.svars.get(d.trim()))
                        .is_some_and(|s| s.trim() == exact_diff);
                ok.then(|| {
                    vec![format!(
                        "Effect::continuous(&Filter::Any, Modifier::DamageLifeLossFloor {{ \
                         players: PlayerRel::You, minimum: {min_p} }}, {duration})"
                    )]
                })
            }
            _ => None,
        }
    }

    fn effect_static_player(
        &self,
        raw: Option<&str>,
        subject: EffectSubject,
    ) -> Option<&'static str> {
        match (subject, raw) {
            (EffectSubject::Unbound, None | Some("Player")) => Some("PlayerRel::EachPlayer"),
            (EffectSubject::Unbound, Some("You")) => Some("PlayerRel::You"),
            (EffectSubject::Unbound, Some("Opponent" | "Player.Opponent")) => {
                Some("PlayerRel::EachOpponent")
            }
            (EffectSubject::Unbound, Some("EnchantedPlayer" | "Player.EnchantedBy")) => {
                Some("PlayerRel::EnchantedPlayer")
            }
            (EffectSubject::Player(who), Some("Player.IsRemembered")) => Some(who),
            (EffectSubject::Event, Some("Player.IsRemembered"))
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("DamageDone" | "DamageDoneOnce")
                ) =>
            {
                Some("PlayerRel::DamagedPlayer")
            }
            _ => None,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn wave3_static_effect_line(
        &mut self,
        mode: &str,
        st: &mut Params,
        subject: EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
        if !matches!(st.take("EffectZone").as_deref(), None | Some("Command")) {
            return None;
        }
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
            "CantAttack"
                if subject == EffectSubject::Unbound && st.peek("Target") == Some("You") =>
            {
                st.take("Target");
                let valid = st.take("ValidCard")?;
                st.exhausted().then_some(())?;
                self.emit_effect_modifiers(
                    subject,
                    &valid,
                    &["Modifier::CantAttackYou".to_string()],
                    duration,
                )
            }
            "CantAttackUnless" if subject == EffectSubject::Unbound => {
                self.wave11_effect_attack_tax(st, subject, duration)
            }
            "CantLoseLife" if subject == EffectSubject::Unbound => {
                let who = self.effect_static_player(st.take("ValidPlayer").as_deref(), subject)?;
                st.exhausted().then_some(())?;
                Some(vec![format!(
                    "Effect::continuous(&Filter::Any, Modifier::CantLoseLife {{ who: {who} }}, {duration})"
                )])
            }
            "CantGainLife" => {
                let who = self.effect_static_player(st.take("ValidPlayer").as_deref(), subject)?;
                st.exhausted().then_some(())?;
                Some(vec![format!(
                    "Effect::continuous(&Filter::Any, Modifier::PlayersCantGainLife({who}), {duration})"
                )])
            }
            "CantChangeLife" if subject == EffectSubject::Unbound => {
                let who = self.effect_static_player(st.take("ValidPlayer").as_deref(), subject)?;
                st.exhausted().then_some(())?;
                Some(vec![format!(
                    "Effect::continuous(&Filter::Any, Modifier::LifeTotalCantChange {{ who: {who} }}, {duration})"
                )])
            }
            "CantSearchLibrary" if subject == EffectSubject::Unbound => {
                if !matches!(
                    st.take("ValidPlayer").as_deref(),
                    Some("Opponent" | "Player.Opponent")
                ) || !st.exhausted()
                {
                    return None;
                }
                Some(vec![format!(
                    "Effect::continuous(&Filter::Any, Modifier::OpponentsCantSearch, {duration})"
                )])
            }
            "CantPlayLand" if subject == EffectSubject::Unbound => {
                if !matches!(st.take("Player").as_deref(), None | Some("Player")) || !st.exhausted()
                {
                    return None;
                }
                Some(vec![format!(
                    "Effect::continuous(&Filter::Any, Modifier::PlayersCantPlay(&Filter::LAND), {duration})"
                )])
            }
            "CanAttackDefender" => {
                let valid = st.take("ValidCard")?;
                st.exhausted().then_some(())?;
                self.emit_effect_modifiers(
                    subject,
                    &valid,
                    &["Modifier::AttacksDespiteDefender".to_string()],
                    duration,
                )
            }
            "MinMaxBlocker" => {
                let valid = st.take("ValidCard")?;
                let mut mods = Vec::new();
                if let Some(raw_max) = st.take("Max") {
                    let n: u8 = raw_max.trim().parse().ok().filter(|&v| v > 0)?;
                    mods.push(format!("Modifier::MaxBlockers({n})"));
                }
                if let Some(raw_min) = st.take("Min") {
                    let n: u8 = raw_min.trim().parse().ok().filter(|&v| v > 0)?;
                    mods.push(format!("Modifier::MinimumBlockers({n})"));
                }
                if mods.is_empty() || !st.exhausted() {
                    return None;
                }
                self.emit_effect_modifiers(subject, &valid, &mods, duration)
            }
            "CantPreventDamage" if subject == EffectSubject::Unbound => {
                if st.take("CombatDamage").as_deref() != Some("True") || !st.exhausted() {
                    return None;
                }
                Some(vec![format!(
                    "Effect::continuous(&Filter::Any, Modifier::CombatDamageCantBePrevented, {duration})"
                )])
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
            "CantBeCast"
                if matches!(subject, EffectSubject::Unbound | EffectSubject::Player(_)) =>
            {
                self.wave11_effect_cant_be_cast(st, subject, duration)
            }
            "CastWithFlash" if subject == EffectSubject::Unbound => {
                Self::wave3_cast_with_flash(st, duration)
            }
            "ReduceCost" | "RaiseCost" if subject == EffectSubject::Unbound => {
                self.wave9_effect_spell_cost(mode, st, duration)
            }
            "CantBeActivated" => {
                let modifier = match st.take("ValidSA").as_deref() {
                    Some("Activated.!ManaAbility") => "Modifier::CantActivateNonMana",
                    None => "Modifier::CantActivate",
                    Some(_) => return None,
                };
                let valid = st.take("ValidCard")?;
                st.exhausted().then_some(())?;
                self.emit_effect_modifiers(subject, &valid, &[modifier.to_string()], duration)
            }
            "Continuous" => self.effect_continuous_static(st, subject, duration),
            _ => None,
        }
    }

    fn wave11_effect_attack_tax(
        &mut self,
        st: &mut Params,
        subject: EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
        let target_ok = matches!(
            st.take("Target").as_deref(),
            Some("You,Planeswalker.YouCtrl" | "Planeswalker.YouCtrl,You")
        );
        if !target_ok {
            return None;
        }
        let valid = st.take("ValidCard")?;
        let cost = st.take("Cost")?;
        let modifier = if let Ok(mana) = cost.trim().parse::<u32>()
            && mana > 0
        {
            format!("Modifier::AttackTaxMana {{ mana: {mana} }}")
        } else {
            let life = cost
                .trim()
                .strip_prefix("PayLife<")
                .and_then(|s| s.strip_suffix('>'))
                .and_then(|s| s.parse::<u16>().ok())
                .filter(|&n| n > 0)?;
            format!("Modifier::AttackTaxLife {{ life: {life} }}")
        };
        st.exhausted().then_some(())?;
        self.emit_effect_modifiers(subject, &valid, &[modifier], duration)
    }

    fn wave11_effect_cant_be_cast(
        &mut self,
        st: &mut Params,
        subject: EffectSubject,
        duration: &str,
    ) -> Option<Vec<String>> {
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
        let caster = st.take("Caster");
        let modifier = match (subject, caster.as_deref()) {
            (EffectSubject::Unbound, Some("Opponent" | "Player.Opponent")) => {
                format!("Modifier::OpponentsCantCast({filter_ref})")
            }
            (EffectSubject::Unbound, None | Some("Player")) => format!(
                "Modifier::PlayersCantCast {{ who: PlayerRel::EachPlayer, filter: {filter_ref} }}"
            ),
            (EffectSubject::Unbound, Some("You")) => {
                format!("Modifier::PlayersCantCast {{ who: PlayerRel::You, filter: {filter_ref} }}")
            }
            (EffectSubject::Unbound, Some("EnchantedPlayer" | "Player.EnchantedBy")) => {
                format!(
                    "Modifier::PlayersCantCast {{ who: PlayerRel::EnchantedPlayer, filter: {filter_ref} }}"
                )
            }
            (EffectSubject::Player(who), Some("Player.IsRemembered")) => {
                format!("Modifier::PlayersCantCast {{ who: {who}, filter: {filter_ref} }}")
            }
            _ => return None,
        };
        st.exhausted().then_some(())?;
        Some(vec![format!(
            "Effect::continuous(&Filter::Any, {modifier}, {duration})"
        )])
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

    /// `DB$ Effect` carrying `Mode$ ReduceCost` or `Mode$ RaiseCost` for spells
    /// (`Modifier::SpellsCostLess`, `Modifier::SpellsCostMore`,
    /// `Modifier::SpellsCostMoreMana`). `ForgetOnCast$` stays unclaimed on the
    /// enclosing `Effect` line so one-shot "next spell" discounts stay refused.
    fn wave9_effect_spell_cost(
        &mut self,
        mode: &str,
        st: &mut Params,
        duration: &str,
    ) -> Option<Vec<String>> {
        if st.take("Type").as_deref() != Some("Spell")
            || !matches!(st.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return None;
        }
        let raw_valid = st.take("ValidCard")?;
        if raw_valid.contains("IsRemembered")
            || raw_valid.contains("EffectSource")
            || raw_valid.contains("RememberedPlayer")
            || raw_valid.contains("sharesCreatureTypeWith")
            || raw_valid.contains("faceDown")
        {
            return None;
        }
        let append_ctrl = |valid: &str, suffix: &str| {
            valid
                .split(',')
                .map(|alt| {
                    let alt = alt.trim();
                    if alt.contains('.') {
                        format!("{alt}+{suffix}")
                    } else {
                        format!("{alt}.{suffix}")
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        };
        let valid = match st.take("Activator").as_deref() {
            None | Some("Player") => raw_valid,
            Some("You") => append_ctrl(&raw_valid, "YouCtrl"),
            Some("Opponent") => append_ctrl(&raw_valid, "OppCtrl"),
            Some(_) => return None,
        };
        let modifier = if mode == "ReduceCost" {
            let amount: u8 = st.take("Amount")?.trim().parse().ok()?;
            if amount == 0 {
                return None;
            }
            format!("Modifier::SpellsCostLess({amount})")
        } else {
            match (st.take("Amount"), st.take("Cost")) {
                (Some(raw_amt), None) => {
                    let amount: u8 = raw_amt.trim().parse().ok()?;
                    if amount == 0 {
                        return None;
                    }
                    format!("Modifier::SpellsCostMore({amount})")
                }
                (None, Some(cost)) => {
                    let mana = super::printed_mana(&cost)?;
                    if let Some(amount) = super::generic_mana(&mana) {
                        format!("Modifier::SpellsCostMore({amount})")
                    } else {
                        format!("Modifier::SpellsCostMoreMana(mana!(\"{mana}\"))")
                    }
                }
                _ => return None,
            }
        };
        if !st.exhausted() {
            return None;
        }
        let filter_ref = if valid == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            let name = self.body.filter_static("EFFECT", &expr);
            format!("&{name}")
        };
        Some(vec![format!(
            "Effect::continuous({filter_ref}, {modifier}, {duration})"
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

    pub(super) fn effect_remember_subject(
        &self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<EffectSubject> {
        let obj_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        let player_target = matches!(
            target,
            Some(
                "TargetSpec::AnyPlayer"
                    | "TargetSpec::AnyOpponent"
                    | "TargetSpec::Player(PlayerRel::Chosen)"
            )
        );
        Some(match p.take("RememberObjects").as_deref() {
            None => EffectSubject::Unbound,
            Some("Targeted" | "ParentTarget" | "ThisTargetedCard") if obj_target => {
                EffectSubject::Targets
            }
            Some("Targeted" | "ParentTarget") if player_target => {
                EffectSubject::Player("PlayerRel::Chosen")
            }
            Some("TargetedController") if target.is_some() => {
                EffectSubject::Player("PlayerRel::ControllerOfTarget")
            }
            Some("TriggeredDefendingPlayer")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerBlocked" | "AttackerUnblocked")
                ) =>
            {
                EffectSubject::Player("PlayerRel::DefendingPlayer")
            }
            Some("TriggeredPlayer") if self.trigger_mode.as_deref() == Some("Phase") => {
                EffectSubject::Player("PlayerRel::ActivePlayer")
            }
            Some("TriggeredPlayer" | "TriggeredCardController")
                if self.trigger_mode.as_deref() == Some("Sacrificed") =>
            {
                EffectSubject::Player("PlayerRel::ControllerOfEvent")
            }
            Some("TriggeredAttackingPlayer" | "TriggeredPlayer")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacked" | "AttackersDeclared" | "AttackersDeclaredOneTarget")
                ) =>
            {
                EffectSubject::Player("PlayerRel::EventPlayer")
            }
            Some("EnchantedPlayer" | "Player.EnchantedBy") => {
                EffectSubject::Player("PlayerRel::EnchantedPlayer")
            }
            Some("Self") if !self.on_a_spell => EffectSubject::Source,
            Some("DelayTriggerRemembered" | "DelayTriggerRememberedLKI")
                if self.in_delayed && target.is_none() =>
            {
                EffectSubject::Event
            }
            Some("TriggeredCard" | "TriggeredCardLKICopy")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("ChangesZone" | "Sacrificed")
                ) =>
            {
                EffectSubject::Event
            }
            Some("TriggeredAttacker" | "TriggeredAttackerLKICopy")
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
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
        let blocker = st.take("ValidBlocker");
        if !self.on_a_spell
            && subject == EffectSubject::Targets
            && matches!(
                attacker.as_str(),
                "Card.EffectSource" | "Creature.EffectSource"
            )
            && matches!(
                blocker.as_deref(),
                Some("Card.IsRemembered" | "Creature.IsRemembered")
            )
        {
            return st.exhausted().then(|| {
                vec![format!(
                    "Effect::ModifyTargets {{ modifier: Modifier::CantBlock(&Filter::This), \
                     duration: {duration} }}"
                )]
            });
        }
        let modifier = match blocker {
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

    #[allow(clippy::too_many_lines)]
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
        if let Some(player_who) = match (subject, affected.as_str()) {
            (EffectSubject::Unbound, "You") => Some("PlayerRel::You"),
            (EffectSubject::Player(who), "Player.IsRemembered") => Some(who),
            _ => None,
        } {
            let modifier =
                if player_who == "PlayerRel::You" && st.peek("NoHandSize") == Some("True") {
                    st.take("NoHandSize");
                    "Modifier::NoMaxHandSize".to_string()
                } else if player_who == "PlayerRel::You"
                    && st.peek("SetMaxHandSize") == Some("Unlimited")
                {
                    st.take("SetMaxHandSize");
                    "Modifier::NoMaxHandSize".to_string()
                } else if st.peek("AddKeyword") == Some("Hexproof") {
                    st.take("AddKeyword");
                    format!("Modifier::PlayerHexproof {{ who: {player_who} }}")
                } else if st.peek("AddKeyword") == Some("Protection from everything") {
                    st.take("AddKeyword");
                    format!("Modifier::PlayerProtectionFromEverything {{ who: {player_who} }}")
                } else if player_who == "PlayerRel::You" {
                    let raw_drops = st.take("AdjustLandPlays")?;
                    let n: u8 = raw_drops.trim().parse().ok().filter(|&v| v > 0)?;
                    format!("Modifier::ExtraLandDrops({n})")
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
            match raw_kw.trim() {
                "Prevent all combat damage that would be dealt by CARDNAME." => {
                    mods.push("Modifier::PreventDamageFromIt".to_string());
                }
                "Prevent all damage that would be dealt by CARDNAME." => {
                    mods.push("Modifier::PreventAllDamageFromIt".to_string());
                }
                _ => {
                    let kw = Self::pump_keyword_bits(&raw_kw)?;
                    mods.push(format!("Modifier::AddKeyword({kw})"));
                }
            }
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
                "CARDNAME can't be blocked except by two or more creatures." => {
                    "Modifier::MinimumBlockers(2)"
                }
                "CARDNAME can block an additional creature each combat." => {
                    "Modifier::CanBlockAdditional(1)"
                }
                "CARDNAME can attack as though it didn't have defender." => {
                    "Modifier::AttacksDespiteDefender"
                }
                "CARDNAME can attack as though it had haste." => "Modifier::AttacksAsThoughHaste",
                "CARDNAME can't be blocked." => "Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)",
                "CARDNAME can only attack alone." => "Modifier::AttacksAloneOnly",
                "CARDNAME blocks each attacking creature if able." => {
                    "Modifier::BlocksEachAttackerIfAble"
                }
                _ => return None,
            };
            mods.push(m.to_string());
        }
        if let Some(raw_rem) = st
            .take("RemoveKeyword")
            .or_else(|| st.take("RemoveKeywords"))
        {
            let kw = Self::pump_keyword_bits(&raw_rem)?;
            mods.push(format!("Modifier::RemoveKeyword({kw})"));
        }
        if let Some(raw_cant) = st.take("CantHaveKeyword") {
            let kw = Self::pump_keyword_bits(&raw_cant)?;
            mods.push(format!("Modifier::CantHaveKeywords({kw})"));
        }
        match st.take("MayAssignDamageAsUnblocked").as_deref() {
            None => {}
            Some("True") => mods.push("Modifier::MayAssignCombatDamageAsThoughUnblocked".into()),
            Some(_) => return None,
        }
        match st.take("RemoveAllAbilities").as_deref() {
            None => {}
            Some("True") => mods.push("Modifier::LoseAllAbilities".to_string()),
            Some(_) => return None,
        }
        if let Some(types) = st.take("AddType") {
            for word in types.split('&').map(str::trim).filter(|s| !s.is_empty()) {
                if let Some(tc) = super::card_type_const(word) {
                    mods.push(format!("Modifier::AddType({tc})"));
                } else if !word.contains(['.', '+', ',', ' ', '!'])
                    && let Some(expr) = self.filter_expr(word)
                    && let Some(sub_const) = expr
                        .strip_prefix("Filter::HasSubtype(")
                        .and_then(|s| s.strip_suffix(')'))
                {
                    mods.push(format!("Modifier::AddSubtype({sub_const})"));
                } else {
                    return None;
                }
            }
        }
        if let Some(raw_colors) = st.take("SetColor") {
            let cs = Self::parse_copy_colors(&raw_colors)?;
            mods.push(format!("Modifier::SetColor({cs})"));
        }
        if let Some(raw_colors) = st.take("AddColor") {
            let cs = Self::parse_copy_colors(&raw_colors)?;
            mods.push(format!("Modifier::AddColor({cs})"));
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
        let mut out = vec![format!(
            "Effect::continuous(&Filter::This, Modifier::GainControl, {duration})"
        )];
        if untap {
            // A source may trigger on untapping. Its controller at that
            // instant depends on the printed order (CR 608.2c).
            let oracle = self.oracle.to_ascii_lowercase();
            let mut orders = oracle
                .split("\\n")
                .filter_map(|line| Some(line.find("gain control of ")? < line.find("untap ")?));
            let control_first = orders.next().unwrap_or(false);
            if orders.any(|order| order != control_first) {
                return self.deny("ambiguous printed control and untap order".to_string());
            }
            out.insert(
                usize::from(control_first),
                "Effect::UntapTarget".to_string(),
            );
        }
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
    /// and optional `Cleanup`: lowers to `Effect::ExileTopToPlay`,
    /// `Effect::ExileTopMayCast`, or `Effect::ExileTopWithPermission`.
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
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return self.deny(format!(
                "top-of-library exile with `ForgetOtherRemembered$ {forget}`"
            ));
        }
        p.take("IsCurse");
        let who = self.player_rel(p.take("DefinedPlayer").as_deref())?;
        let raw_count = p.take("ChangeNum").unwrap_or_else(|| "1".to_string());
        let (fixed_count, amount_expr) = self.exile_top_count_amount(&raw_count)?;
        self.exile_top_play_follow_up(who, fixed_count, &amount_expr, sub)
    }

    /// `Dig | DigNum$ N | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True`
    /// chained into `DB$ Effect` granting `MayPlay$ True` from `Exile`
    /// (impulse draw: `Effect::ExileTopToPlay` / `Effect::ExileTopMayCast` /
    /// `Effect::ExileTopWithPermission`).
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
        let change_num = p.take("ChangeNum");
        let dig_num = p.take("DigNum")?;
        let change_all = change_num.as_deref() == Some("All")
            || (dig_num == "1" && matches!(change_num.as_deref(), None | Some("1")));
        if !change_all
            || p.take("DestinationZone").as_deref() != Some("Exile")
            || p.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        if !matches!(p.take("ChangeValid").as_deref(), None | Some("Card")) {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return self.deny(format!(
                "`Dig` exile with `ForgetOtherRemembered$ {forget}`"
            ));
        }
        if p.peek("Imprint") == Some("True")
            && self.on_a_spell
            && !self
                .svars
                .values()
                .any(|v| v.contains("Imprinted") || v.contains("ExiledWith"))
        {
            p.take("Imprint");
        }
        p.take("IsCurse");
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let (fixed_count, amount_expr) = self.exile_top_count_amount(&dig_num)?;
        self.exile_top_play_follow_up(who, fixed_count, &amount_expr, sub)
    }

    /// `Dig | Defined$ You | DigNum$ N | DestinationZone$ Exile | ExileFaceDown$ True |
    /// WithMayLook$ True | RememberChanged$ True | SubAbility$ DBEffect` granting
    /// indefinite casting of specific spell types (`Vivien, Champion of the Wilds`)
    /// -> `Effect::LookAtTopExileFaceDown { count, cast_types }`.
    pub(super) fn dig_look_at_top_exile_face_down_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || !matches!(p.take("Defined").as_deref(), None | Some("You"))
            || !matches!(p.take("ChangeNum").as_deref(), None | Some("1"))
            || p.take("DestinationZone").as_deref() != Some("Exile")
            || p.take("ExileFaceDown").as_deref() != Some("True")
            || p.take("WithMayLook").as_deref() != Some("True")
            || p.take("RememberChanged").as_deref() != Some("True")
            || !matches!(
                p.take("DestinationZone2").as_deref(),
                None | Some("Library")
            )
            || !matches!(p.take("LibraryPosition2").as_deref(), None | Some("-1"))
            || !matches!(p.take("ChangeValid").as_deref(), None | Some("Card"))
        {
            return None;
        }
        let raw_count = p.take("DigNum")?;
        let count = self
            .plain_num(&raw_count)
            .and_then(|n| u8::try_from(n).ok())
            .filter(|&n| n >= 1)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Dig.{key}`"));
        }
        let Some(eff_name) = sub.take() else {
            return self.deny("face-down `Dig` exile without `SubAbility$`".into());
        };
        let (spec, next_sub) =
            self.consume_remembered_exile_play_effect(&eff_name, true, "face-down `Dig` exile")?;
        if let Some(cname) = next_sub {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        if spec.window != "PlayWindow::WhileExiled" || spec.free || spec.mana != "ExileMana::Exact"
        {
            return self.deny("face-down `Dig` exile with unsupported play permission".into());
        }
        let Some(cast_types) = spec.spell_types else {
            return self.deny("face-down `Dig` exile without typed spell cast restriction".into());
        };
        Some(vec![format!(
            "Effect::LookAtTopExileFaceDown {{ count: {count}, cast_types: {cast_types} }}"
        )])
    }

    fn exile_top_count_amount(&mut self, raw: &str) -> Option<(Option<u8>, String)> {
        if let Some(n) = self.plain_num(raw) {
            let n = u8::try_from(n).ok().filter(|&n| n >= 1)?;
            return Some((Some(n), format!("Amount::Fixed({n})")));
        }
        let amount = self.amount_or_count_ext(raw)?;
        Some((None, amount))
    }

    fn exile_top_play_follow_up(
        &mut self,
        who: &str,
        fixed_count: Option<u8>,
        amount_expr: &str,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let Some(eff_name) = sub.take() else {
            return self.deny("top-of-library exile without `SubAbility$`".into());
        };
        let (spec, next_sub) = self.consume_remembered_exile_play_effect(
            &eff_name,
            who == "PlayerRel::You",
            "top-of-library exile",
        )?;
        if let Some(cname) = next_sub {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        if spec.cast_only
            && spec.spell_types.is_none()
            && fixed_count == Some(1)
            && spec.window == "PlayWindow::ThisTurn"
            && !spec.free
            && spec.mana == "ExileMana::Exact"
        {
            return Some(vec![format!(
                "Effect::ExileTopMayCast {{ who: {who}, nonland_only: true }}"
            )]);
        }
        if !spec.cast_only
            && let Some(count) = fixed_count
            && matches!(
                spec.window,
                "PlayWindow::ThisTurn"
                    | "PlayWindow::UntilYourNextTurn"
                    | "PlayWindow::UntilYourNextUpkeep"
                    | "PlayWindow::ThroughYourNextTurn"
            )
            && spec.mana != "ExileMana::AnyColor"
        {
            let mana_any_type = spec.mana == "ExileMana::AnyType";
            let spell_types = spec
                .spell_types
                .as_deref()
                .map_or_else(|| "None".to_string(), |t| format!("Some({t})"));
            return Some(vec![format!(
                "Effect::ExileTopToPlay {{ who: {who}, play: ExilePlay {{ count: {count}, \
                 window: {}, free: {}, mana_any_type: {mana_any_type}, \
                 spell_types: {spell_types} }} }}",
                spec.window, spec.free
            )]);
        }
        let permission = spec.to_expr();
        Some(vec![format!(
            "Effect::ExileTopWithPermission {{ who: {who}, count: {amount_expr}, \
             permission: {permission} }}"
        )])
    }

    /// `DB$ ChangeZone | Origin$ Battlefield/Graveyard | Destination$ Exile |
    /// RememberChanged$ True | SubAbility$ DBEffect` granting `MayPlay$ True`
    /// from exile: lowers to `Effect::ExileWithPermission`.
    pub(super) fn exile_with_permission_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        let origin = p.take("Origin")?;
        if !matches!(origin.as_str(), "Battlefield" | "Graveyard")
            || p.take("Destination").as_deref() != Some("Exile")
            || p.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return self.deny(format!(
                "exile-with-permission `ForgetOtherRemembered$ {forget}`"
            ));
        }
        p.take("IsCurse");
        let until = match p.take("Duration").as_deref() {
            None => "None",
            Some("UntilHostLeavesPlay") if origin == "Battlefield" && !self.on_a_spell => {
                "Some(ExileUntil::SourceLeavesBattlefield)"
            }
            Some(other) => {
                return self.deny(format!("exile-with-permission `Duration$ {other}`"));
            }
        };
        let defined = p.take("Defined");
        let target_spec = match (origin.as_str(), defined.as_deref()) {
            ("Battlefield", None) if targets_here => target
                .filter(|t| {
                    t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::ObjectOfEventPlayer(")
                })?
                .to_string(),
            ("Battlefield", Some("Targeted" | "ParentTarget")) if !targets_here => target
                .filter(|t| {
                    t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::ObjectOfEventPlayer(")
                })?
                .to_string(),
            ("Battlefield", Some("Self"))
                if !targets_here && !self.on_a_spell && target.is_none() =>
            {
                "TargetSpec::ThisObject".to_string()
            }
            ("Battlefield", Some(def))
                if !targets_here && target.is_none() && self.is_event_object_defined(def) =>
            {
                "TargetSpec::EventObject".to_string()
            }
            ("Graveyard", None) if targets_here => target
                .filter(|t| t.starts_with("TargetSpec::CardInGraveyard("))?
                .to_string(),
            ("Graveyard", Some("Targeted" | "ParentTarget")) if !targets_here => target
                .filter(|t| t.starts_with("TargetSpec::CardInGraveyard("))?
                .to_string(),
            ("Graveyard", Some(def))
                if !targets_here && target.is_none() && self.is_event_object_defined(def) =>
            {
                "TargetSpec::EventObject".to_string()
            }
            _ => {
                return self.deny("exile-with-permission target outside supported zones".into());
            }
        };
        let Some(eff_name) = sub.take() else {
            return self.deny("exile-with-permission without `SubAbility$`".into());
        };
        let (spec, next_sub) =
            self.consume_remembered_exile_play_effect(&eff_name, false, "exile-with-permission")?;
        if let Some(cname) = next_sub {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        let permission = spec.to_expr();
        Some(vec![format!(
            "Effect::ExileWithPermission {{ target: {target_spec}, until: {until}, \
             permission: {permission} }}"
        )])
    }

    /// `ChangeZone | Origin$ Hand | Destination$ Exile | RememberChanged$ True |
    /// SubAbility$ DBEffect` granting indefinite cast permission on the exiled
    /// card -> `Effect::ExileChosenFromHandWithSourceAndCast(&filter)`.
    pub(super) fn exile_chosen_from_hand_and_cast_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || p.take("Origin").as_deref() != Some("Hand")
            || p.take("Destination").as_deref() != Some("Exile")
            || p.take("RememberChanged").as_deref() != Some("True")
            || !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
            || !matches!(p.take("Chooser").as_deref(), None | Some("You"))
            || !matches!(p.take("ChangeNum").as_deref(), None | Some("1"))
            || !matches!(p.take("Hidden").as_deref(), None | Some("True"))
            || !matches!(p.take("Imprint").as_deref(), None | Some("True"))
        {
            return None;
        }
        p.claim_label("ChangeTypeDesc", "ChangeType");
        let optional = match (
            p.take("Optional").as_deref(),
            p.take("Mandatory").as_deref(),
        ) {
            (None | Some("False"), None | Some("True")) => false,
            (Some("True"), None | Some("False")) => true,
            _ => return None,
        };
        let valid = p.take("ChangeType")?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChangeZone.{key}`"));
        }
        let eff_name = sub.take()?;
        let (spec, next_sub) = self.consume_remembered_exile_play_effect(
            &eff_name,
            true,
            "hand-to-exile cast permission",
        )?;
        if let Some(cname) = next_sub {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        let filter_expr = self.filter_expr_in_zone(&valid, true)?;
        let nonland_valid = valid.split(',').all(|alt| {
            let trimmed = alt.trim();
            trimmed.contains("nonLand")
                || trimmed.contains("!Land")
                || matches!(
                    trimmed.split(['.', '+']).next().unwrap_or(""),
                    "Instant" | "Sorcery" | "Planeswalker"
                )
        });
        if spec.window != "PlayWindow::WhileExiled"
            || spec.free
            || spec.mana != "ExileMana::Exact"
            || spec.spell_types.is_some()
            || (!spec.cast_only && !nonland_valid)
        {
            return self.deny(
                "hand-to-exile cast permission outside `ExileChosenFromHandWithSourceAndCast`"
                    .into(),
            );
        }
        let filter = self.body.filter_static("IMPRINT", &filter_expr);
        let inner = format!("Effect::ExileChosenFromHandWithSourceAndCast(&{filter})");
        Some(vec![if optional {
            format!("Effect::MayDo {{ effects: &[{inner}] }}")
        } else {
            inner
        }])
    }

    /// Standalone `DB$ Effect` granting `MayPlay$ True` on cards already exiled
    /// and linked to the source (`Card.ExiledWithEffectSource` /
    /// `Card.ExiledWithSource`): lowers to `Effect::GrantLinkedExilePermission`.
    pub(super) fn grant_linked_exile_permission_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if self.on_a_spell
            || target.is_some()
            || p.has("ReplacementEffects")
            || p.has("Triggers")
            || p.has("Abilities")
            || p.has("Boon")
            || p.has("OneOff")
            || p.has("Unique")
        {
            return None;
        }
        if !matches!(p.take("EffectOwner").as_deref(), None | Some("You"))
            || !matches!(
                p.take("ExileOnMoved")
                    .or_else(|| p.take("ForgetOnMoved"))
                    .as_deref(),
                None | Some("Exile")
            )
        {
            return None;
        }
        let rem_nonland = match p.take("RememberObjects").as_deref() {
            None | Some("ValidExile Card.ExiledWithSource") => false,
            Some(
                "ValidExile Card.ExiledWithSource+nonLand"
                | "ValidExile Card.nonLand+ExiledWithSource",
            ) => true,
            Some(_) => return None,
        };
        let raw_duration = p.take("Duration");
        let static_name = p.take("StaticAbilities")?;
        if static_name.contains(',') {
            return None;
        }
        // Only claim this `DB$ Effect` as a linked-exile permission if the
        // referenced static actually grants `MayPlay$ True` from `Exile`;
        // otherwise return `None` silently without overwriting the true refusal.
        let is_exile_play = self
            .svars
            .get(&static_name)
            .and_then(|body| Params::parse(body))
            .is_some_and(|(mode, sp)| {
                mode == "Continuous"
                    && sp.peek("MayPlay") == Some("True")
                    && sp.peek("AffectedZone") == Some("Exile")
            });
        if !is_exile_play {
            return None;
        }
        let spec =
            self.parse_exile_play_static(&static_name, true, false, raw_duration.as_deref())?;
        if rem_nonland && !spec.cast_only {
            return None;
        }
        Some(vec![format!(
            "Effect::GrantLinkedExilePermission {{ permission: {} }}",
            spec.to_expr()
        )])
    }

    /// `DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand/Exile | RememberChanged$ True`
    /// followed by one or more `DB$` lines guarded by
    /// `ConditionDefined$ Remembered | ConditionPresent$ <valid>` ("If you do, …"):
    /// lowers the move normally and gates the follow-ups with
    /// `ThisWay::ReturnedToHand` / `ThisWay::ExiledFromBattlefield`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn change_zone_then_if_moved(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let origin = p.peek("Origin")?.to_string();
        let dest = p.peek("Destination")?.to_string();
        let gy_exile = origin == "Graveyard" && dest == "Exile";
        let this_way_ctor = if origin == "Battlefield" && dest == "Hand" {
            Some("ThisWay::ReturnedToHand")
        } else if origin == "Battlefield" && dest == "Exile" {
            Some("ThisWay::ExiledFromBattlefield")
        } else if gy_exile {
            if !single_target
                || p.has("Hidden")
                || !target.is_some_and(|t| t.starts_with("TargetSpec::CardInGraveyard"))
            {
                return None;
            }
            None
        } else {
            return None;
        };
        let first_sub_name = sub.as_deref()?.to_string();
        let first_body = self.svars.get(&first_sub_name)?.clone();
        let (_, first_sp) = Params::parse(&first_body)?;
        let first_is_remembered_present = first_sp.peek("ConditionDefined") == Some("Remembered")
            && first_sp.peek("ConditionPresent").is_some();
        let first_is_remembered_amount = gy_exile
            && first_sp
                .peek("ConditionCheckSVar")
                .and_then(|v| self.svars.get(v.trim()))
                .is_some_and(|expr| expr.trim() == "Remembered$Amount");
        if !first_is_remembered_present && !first_is_remembered_amount {
            return None;
        }
        if p.take("RememberChanged").as_deref() != Some("True") {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        let mut out = self.change_zone(p, target, targets_here, single_target)?;
        let mut cursor = sub.take();
        let mut pending_cond: Option<String> = None;
        let mut pending_body: Vec<String> = Vec::new();
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 6 {
                return None;
            }
            let Some(body) = self.svars.get(&name).cloned() else {
                return self.deny(format!("`SubAbility$ {name}` names no SVar"));
            };
            let (s_api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if s_api == "Cleanup" && sp.take("ClearRemembered").as_deref() == Some("True") {
                *sub = sp.take("SubAbility");
                if !sp.exhausted() {
                    return None;
                }
                break;
            }
            let is_present_cond = sp.peek("ConditionDefined") == Some("Remembered")
                && sp.peek("ConditionPresent").is_some()
                && !sp.has("ValidTgts");
            let is_amount_cond = gy_exile
                && !sp.has("ValidTgts")
                && sp
                    .peek("ConditionCheckSVar")
                    .and_then(|v| self.svars.get(v.trim()))
                    .is_some_and(|expr| expr.trim() == "Remembered$Amount");
            if !is_present_cond && !is_amount_cond {
                *sub = Some(name);
                break;
            }
            if sp.entries.iter().any(|(k, v)| {
                k != "ConditionDefined" && (v.contains("Remembered") || v.contains("IsRemembered"))
            }) {
                return None;
            }
            let next_sub = sp.take("SubAbility");
            let (valid, cmp) = if is_present_cond {
                sp.take("ConditionDefined");
                let v = sp.take("ConditionPresent")?;
                let c = sp.take("ConditionCompare");
                (v, c)
            } else {
                sp.take("ConditionCheckSVar");
                let c = sp.take("ConditionSVarCompare");
                ("Card".to_string(), c)
            };
            sp.take("ConditionDescription");
            let positive = match cmp.as_deref() {
                None | Some("GE1" | "EQ1" | "GT0") => true,
                Some("EQ0" | "LT1") => false,
                Some(_) => return None,
            };
            let cond = if let Some(ctor) = this_way_ctor {
                let filter_ref = if valid == "Card" {
                    "&Filter::Any".to_string()
                } else {
                    let expr = self.filter_expr(&valid)?;
                    let fname = self.body.filter_static("MOVED", &expr);
                    format!("&{fname}")
                };
                if positive {
                    format!("Condition::AmountAtLeast(Amount::ThisWay({ctor}({filter_ref})), 1)")
                } else {
                    format!(
                        "Condition::Not(&Condition::AmountAtLeast(Amount::ThisWay({ctor}({filter_ref})), 1))"
                    )
                }
            } else if valid == "Card" {
                if positive {
                    "Condition::ObjectMatches { what: &TargetSpec::Object(&Filter::Any), \
                     filter: &Filter::InZone(ZoneRef::Exile) }"
                        .to_string()
                } else {
                    "Condition::Not(&Condition::ObjectMatches { what: &TargetSpec::Object(&Filter::Any), \
                     filter: &Filter::InZone(ZoneRef::Exile) })"
                        .to_string()
                }
            } else {
                let inner_expr = self.filter_expr_in_zone(&valid, true)?;
                let combined = if positive {
                    format!("Filter::And(&[Filter::InZone(ZoneRef::Exile), {inner_expr}])")
                } else {
                    let inner_name = self.body.filter_static("MOVED", &inner_expr);
                    format!(
                        "Filter::And(&[Filter::InZone(ZoneRef::Exile), Filter::Not(&{inner_name})])"
                    )
                };
                let fname = self.body.filter_static("MOVED", &combined);
                format!(
                    "Condition::ObjectMatches {{ what: &TargetSpec::Object(&Filter::Any), \
                     filter: &{fname} }}"
                )
            };
            if gy_exile
                && sp.peek("Defined").is_none()
                && matches!(
                    s_api.as_str(),
                    "Pump" | "PutCounter" | "Destroy" | "Tap" | "Untap" | "ChangeZone"
                )
            {
                return None;
            }
            let step_effects = self.effect_of_followup(&s_api, &mut sp, target)?;
            if !sp.exhausted() || step_effects.is_empty() {
                return None;
            }
            if pending_cond.as_deref() == Some(cond.as_str()) {
                pending_body.extend(step_effects);
            } else {
                if let Some(prev_cond) = pending_cond.take() {
                    out.push(format!(
                        "Effect::IfCondition {{ condition: {prev_cond}, then: &[{}], otherwise: &[] }}",
                        pending_body.join(", ")
                    ));
                    pending_body.clear();
                }
                pending_cond = Some(cond);
                pending_body.extend(step_effects);
            }
            cursor = next_sub;
        }
        if let Some(prev_cond) = pending_cond.take() {
            out.push(format!(
                "Effect::IfCondition {{ condition: {prev_cond}, then: &[{}], otherwise: &[] }}",
                pending_body.join(", ")
            ));
        }
        Some(out)
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn reanimate_remembered_modified(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        let target = target.filter(|t| t.starts_with("TargetSpec::CardInGraveyard"))?;
        if p.has("Hidden")
            || p.take("Origin").as_deref() != Some("Graveyard")
            || p.take("Destination").as_deref() != Some("Battlefield")
            || p.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        match p.take("Defined").as_deref() {
            None => {}
            Some("Targeted") if !targets_here => {}
            _ => return None,
        }
        if let Some(gain) = p.take("GainControl")
            && gain != "True"
        {
            return None;
        }
        if let Some(num) = p.take("ChangeNum")
            && num != "1"
        {
            return None;
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let mut counters = self.take_with_counters(p)?;
        let mut modifiers: Vec<String> = Vec::new();
        let mut cursor = sub.take();
        let mut consumed_any = false;
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 4 {
                return None;
            }
            let Some(body) = self.svars.get(name.trim()).cloned() else {
                return self.deny(format!("`SubAbility$ {name}` names no SVar"));
            };
            let (s_api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if s_api == "Cleanup" && sp.take("ClearRemembered").as_deref() == Some("True") {
                *sub = sp.take("SubAbility");
                if !sp.exhausted() {
                    return None;
                }
                break;
            }
            if s_api == "PutCounter"
                && sp.peek("Defined") == Some("Remembered")
                && !sp.has("ValidTgts")
                && !sp.has("Choices")
                && !sp.has("UnlessCost")
            {
                sp.take("Defined");
                sp.take("IsCurse");
                let next = sp.take("SubAbility");
                let code = sp.take("CounterType")?;
                let kind = super::counter_kind(&code)?;
                let n: u16 = self
                    .plain_num(sp.take("CounterNum").as_deref().unwrap_or("1"))?
                    .try_into()
                    .ok()?;
                if n == 0 || counters != "None" || !sp.exhausted() {
                    return None;
                }
                counters = format!("Some(({kind}, {n}))");
                consumed_any = true;
                cursor = next;
                continue;
            }
            if s_api == "Animate"
                && sp.peek("Defined") == Some("Remembered")
                && sp.peek("Duration") == Some("Permanent")
                && !sp.has("ValidTgts")
                && !sp.has("AtEOT")
                && !sp.has("UnlessCost")
            {
                sp.take("Defined");
                sp.take("Duration");
                let next = sp.take("SubAbility");
                let before = modifiers.len();
                if let Some(kws) = sp.take("Keywords") {
                    let bits = Self::pump_keyword_bits(&kws)?;
                    modifiers.push(format!("Modifier::AddKeyword({bits})"));
                }
                if let Some(leave) = sp.take("LeaveBattlefield") {
                    if leave != "Exile" {
                        return None;
                    }
                    modifiers.push("Modifier::ExileInsteadOfLeavingBattlefield".to_string());
                }
                if let Some(types) = sp.take("Types") {
                    for part in types.split('&').map(str::trim).filter(|s| !s.is_empty()) {
                        if let Some(ct) = card_type_const(part) {
                            modifiers.push(format!("Modifier::AddType({ct})"));
                        } else if !part.contains(['.', '+', ',', ' ', '!'])
                            && let Some(expr) = self.filter_expr(part)
                            && let Some(sub_const) = expr
                                .strip_prefix("Filter::HasSubtype(")
                                .and_then(|s| s.strip_suffix(')'))
                        {
                            modifiers.push(format!("Modifier::AddSubtype({sub_const})"));
                        } else {
                            return None;
                        }
                    }
                }
                if !sp.exhausted() || modifiers.len() == before {
                    return None;
                }
                consumed_any = true;
                cursor = next;
                continue;
            }
            if s_api == "Pump"
                && sp.peek("Defined") == Some("Remembered")
                && sp.peek("Duration") == Some("Permanent")
                && !sp.has("ValidTgts")
                && !sp.has("AtEOT")
                && !sp.has("UnlessCost")
            {
                sp.take("Defined");
                sp.take("Duration");
                let next = sp.take("SubAbility");
                let kws = sp.take("KW")?;
                let bits = Self::pump_keyword_bits(&kws)?;
                if !sp.exhausted() {
                    return None;
                }
                modifiers.push(format!("Modifier::AddKeyword({bits})"));
                consumed_any = true;
                cursor = next;
                continue;
            }
            if sp
                .entries
                .iter()
                .any(|(_, v)| v.contains("Remembered") || v.contains("IsRemembered"))
            {
                return None;
            }
            *sub = Some(name);
            break;
        }
        if !consumed_any {
            return None;
        }
        if tapped || !modifiers.is_empty() {
            Some(vec![format!(
                "Effect::GraveyardToBattlefieldModified {{ target: {target}, \
                 owner_control: false, tapped: {tapped}, counters: {counters}, \
                 modifiers: &[{}] }}",
                modifiers.join(", ")
            )])
        } else {
            Some(vec![format!(
                "Effect::GraveyardToBattlefield {{ target: {target}, \
                 owner_control: false, counters: {counters} }}"
            )])
        }
    }

    pub(super) fn plot_remembered_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if p.take("Destination").as_deref() != Some("Exile")
            || p.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        p.take("IsCurse");
        let sub_name = sub.take()?;
        let body = self.svars.get(sub_name.trim())?.clone();
        let (s_api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        if s_api != "AlterAttribute"
            || sp.take("Defined").as_deref() != Some("Remembered")
            || sp.take("Attributes").as_deref() != Some("Plotted")
            || sp.has("ValidTgts")
            || sp.has("UnlessCost")
        {
            return None;
        }
        let mut next_sub = sp.take("SubAbility");
        if !sp.exhausted() {
            return None;
        }
        if let Some(clean_name) = next_sub.as_deref()
            && let Some(clean_body) = self.svars.get(clean_name.trim()).cloned()
            && let Some((c_api, mut cp)) = Params::parse(&clean_body)
        {
            cp.drop_prose();
            if c_api == "Cleanup" && cp.take("ClearRemembered").as_deref() == Some("True") {
                let after = cp.take("SubAbility");
                if !cp.exhausted() {
                    return None;
                }
                next_sub = after;
            }
        }
        let origin = p.take("Origin")?;
        let out =
            if origin == "Library" && !targets_here && p.peek("Defined") == Some("TopOfLibrary") {
                p.take("Defined");
                let who = self.wave11_player_rel(p.take("DefinedPlayer").as_deref())?;
                let count = match p.take("ChangeNum").as_deref() {
                    None => "Amount::Fixed(1)".to_string(),
                    Some(raw) => self.amount_or_count_ext(raw)?,
                };
                vec![format!("Effect::PlotTop {{ who: {who}, count: {count} }}")]
            } else {
                let tgt = target?;
                if p.has("Hidden") {
                    return None;
                }
                match p.take("Defined").as_deref() {
                    None if targets_here => {}
                    Some("Targeted" | "ParentTarget") if !targets_here => {}
                    _ => return None,
                }
                let zone_ok = match origin.as_str() {
                    "Stack" if tgt.starts_with("TargetSpec::Spell(") => {
                        match p.take("TargetType").as_deref() {
                            None | Some("Spell" | "Spell.OppCtrl" | "Spell.YouDontCtrl") => true,
                            Some(_) => false,
                        }
                    }
                    "Battlefield" if tgt.starts_with("TargetSpec::Object(") => true,
                    "Graveyard" if tgt.starts_with("TargetSpec::CardInGraveyard") => true,
                    _ => false,
                };
                if !zone_ok {
                    return None;
                }
                vec![format!("Effect::Plot {{ target: {tgt} }}")]
            };
        *sub = next_sub;
        Some(out)
    }

    fn consume_remembered_exile_play_effect(
        &mut self,
        eff_name: &str,
        allow_you_own: bool,
        context: &str,
    ) -> Option<(ExilePermissionSpec, Option<String>)> {
        let Some(eff_body) = self.svars.get(eff_name).cloned() else {
            return self.deny(format!("`SubAbility$ {eff_name}` names no SVar"));
        };
        let Some((eff_api, mut ep)) = Params::parse(&eff_body) else {
            return self.deny(format!("{context} follow-up with no `$`"));
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
            return self.deny(format!(
                "a {context} follow-up other than an exiled-card play permission"
            ));
        }
        let raw_duration = ep.take("Duration");
        let Some(static_name) = ep.take("StaticAbilities") else {
            return self.deny(format!("{context} `Effect` without `StaticAbilities$`"));
        };
        if static_name.contains(',') {
            return self.deny(format!(
                "{context} `Effect` with multiple `StaticAbilities$`"
            ));
        }
        ep.take("Name");
        if let Some(non_stack) = ep.take("NonStackingEffect")
            && non_stack != "True"
        {
            return self.deny(format!(
                "{context} `Effect` with `NonStackingEffect$ {non_stack}`"
            ));
        }
        if let Some(stackable) = ep.take("Stackable")
            && stackable != "False"
        {
            return self.deny(format!("{context} `Effect` with `Stackable$ {stackable}`"));
        }
        if let Some(forget_cast) = ep.take("ForgetOnCast")
            && forget_cast != "False"
        {
            return self.deny(format!(
                "{context} `Effect` with `ForgetOnCast$ {forget_cast}`"
            ));
        }
        let next_sub = ep.take("SubAbility");
        if let Some(key) = ep.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        let spec = self.parse_exile_play_static(
            &static_name,
            false,
            allow_you_own,
            raw_duration.as_deref(),
        )?;
        Some((spec, next_sub))
    }

    #[allow(clippy::too_many_lines)]
    fn parse_exile_play_static(
        &mut self,
        static_name: &str,
        linked_source: bool,
        allow_you_own: bool,
        raw_duration: Option<&str>,
    ) -> Option<ExilePermissionSpec> {
        let Some(static_body) = self.svars.get(static_name).cloned() else {
            return self.deny(format!("`StaticAbilities$ {static_name}` names no SVar"));
        };
        let Some((mode, mut sp)) = Params::parse(&static_body) else {
            return self.deny("exile play static ability without `Mode$`".into());
        };
        let desc_lower = format!(
            "{} {}",
            sp.peek("Description")
                .map_or_default(str::to_ascii_lowercase),
            sp.take("Secondary")
                .map_or_default(|s| s.to_ascii_lowercase()),
        );
        sp.drop_prose();
        if !matches!(sp.take("EffectZone").as_deref(), None | Some("Command")) {
            return self.deny("exile play static ability outside `Command` zone".into());
        }
        if mode != "Continuous"
            || sp.take("MayPlay").as_deref() != Some("True")
            || sp.take("AffectedZone").as_deref() != Some("Exile")
        {
            return self
                .deny("exile play static ability other than playing the exiled card".into());
        }
        let window = match raw_duration {
            None | Some("EndOfTurn" | "UntilEndOfTurn") => "PlayWindow::ThisTurn",
            Some("UntilYourNextTurn") => "PlayWindow::UntilYourNextTurn",
            Some("UntilYourNextUpkeep") => "PlayWindow::UntilYourNextUpkeep",
            Some("UntilTheEndOfYourNextTurn") => "PlayWindow::ThroughYourNextTurn",
            Some("AsLongAsInPlay" | "UntilHostLeavesPlay") if !self.on_a_spell => {
                "PlayWindow::WhileSourceOnBattlefield"
            }
            Some("Permanent") => {
                if desc_lower.contains("you control") {
                    return self
                        .deny("exile permission tied to continuing to control the source".into());
                }
                if desc_lower.contains("remains on the battlefield")
                    || desc_lower.contains("is on the battlefield")
                {
                    if self.on_a_spell {
                        return self.deny(
                            "exile permission tied to source on battlefield on a spell".into(),
                        );
                    }
                    "PlayWindow::WhileSourceOnBattlefield"
                } else {
                    "PlayWindow::WhileExiled"
                }
            }
            Some(other) => {
                return self.deny(format!("exile permission `Duration$ {other}`"));
            }
        };
        let affected = sp.take("Affected")?;
        let Some((cast_only, spell_types)) =
            Self::parse_exile_affected(&affected, linked_source, allow_you_own)
        else {
            return self.deny(format!(
                "exile play static ability with unsupported `Affected$ {affected}`"
            ));
        };
        let free = match sp.take("MayPlayWithoutManaCost").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("exile play `MayPlayWithoutManaCost$ {other}`"));
            }
        };
        let ignore_type = match sp.take("MayPlayIgnoreType").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("exile play `MayPlayIgnoreType$ {other}`"));
            }
        };
        let ignore_color = match sp.take("MayPlayIgnoreColor").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("exile play `MayPlayIgnoreColor$ {other}`"));
            }
        };
        let mana = match (ignore_type, ignore_color) {
            (false, false) => "ExileMana::Exact",
            (false, true) => "ExileMana::AnyColor",
            (true, false) => "ExileMana::AnyType",
            (true, true) => {
                if desc_lower.contains("any color") && !desc_lower.contains("any type") {
                    "ExileMana::AnyColor"
                } else if desc_lower.contains("any type") && !desc_lower.contains("any color") {
                    "ExileMana::AnyType"
                } else {
                    return self.deny(
                        "exile play with both `MayPlayIgnoreType$` and `MayPlayIgnoreColor$`"
                            .into(),
                    );
                }
            }
        };
        if let Some(key) = sp.first_key() {
            return self.deny(format!("unclaimed parameter `StaticAbility.{key}`"));
        }
        Some(ExilePermissionSpec {
            window,
            cast_only,
            spell_types,
            free,
            mana,
        })
    }

    fn parse_exile_affected(
        affected: &str,
        linked_source: bool,
        allow_you_own: bool,
    ) -> Option<(bool, Option<String>)> {
        let is_marker = |atom: &str| {
            if linked_source {
                matches!(atom, "ExiledWithEffectSource" | "ExiledWithSource")
            } else {
                atom == "IsRemembered"
            }
        };
        let alts: Vec<&str> = affected.split(',').map(str::trim).collect();
        if alts.is_empty() {
            return None;
        }
        let mut type_bits: Vec<&'static str> = Vec::new();
        let mut saw_nonland = false;
        let mut saw_untyped = false;
        for alt in &alts {
            let mut parts = alt
                .split(['.', '+'])
                .map(str::trim)
                .filter(|p| !p.is_empty());
            let base = parts.next()?;
            let mut saw_marker = false;
            let mut alt_type: Option<&'static str> = None;
            if base != "Card" {
                let tc = super::card_type_const(base)?;
                if tc == "TypeSet::LAND" {
                    return None;
                }
                alt_type = Some(tc);
            }
            for atom in parts {
                if is_marker(atom) {
                    saw_marker = true;
                } else if matches!(atom, "nonLand" | "!Land") {
                    saw_nonland = true;
                } else if atom == "YouOwn" && allow_you_own {
                    // Own library exile is always owned by you.
                } else if let Some(tc) = super::card_type_const(atom)
                    && tc != "TypeSet::LAND"
                    && alt_type.is_none()
                {
                    alt_type = Some(tc);
                } else {
                    return None;
                }
            }
            if !saw_marker {
                return None;
            }
            if let Some(tc) = alt_type {
                type_bits.push(tc);
            } else {
                saw_untyped = true;
            }
        }
        if saw_untyped {
            if alts.len() != 1 || !type_bits.is_empty() {
                return None;
            }
            return Some((saw_nonland, None));
        }
        if type_bits.is_empty() {
            return None;
        }
        let mut expr = type_bits[0].to_string();
        for bit in &type_bits[1..] {
            expr = format!("{expr}.union({bit})");
        }
        Some((saw_nonland, Some(expr)))
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
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let defined = p.take("Defined");
        let blink_target = match defined.as_deref() {
            None if targets_here => target
                .filter(|t| {
                    t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::ObjectOfEventPlayer(")
                })
                .map(str::to_string),
            Some("Targeted" | "ParentTarget") if !targets_here => target
                .filter(|t| {
                    t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::ObjectOfEventPlayer(")
                })
                .map(str::to_string),
            Some(def) if !targets_here && target.is_none() && self.is_event_object_defined(def) => {
                Some("TargetSpec::EventObject".to_string())
            }
            _ => None,
        };
        let Some(blink_target) = blink_target else {
            return self.deny("a blink that does not refer to a battlefield object".into());
        };
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
            return Some(vec![format!(
                "Effect::Blink {{ target: {blink_target}, owner_control: {owner_control} }}"
            )]);
        }
        if !targets_here || !blink_target.starts_with("TargetSpec::Object(") {
            return self.deny("an end-step blink that does not target a battlefield object".into());
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
    pub(super) fn change_zone(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        self.change_zone_with_prior(p, target, targets_here, single_target, false)
    }

    pub(super) fn has_linked_exile_reader(&self) -> bool {
        self.svars
            .values()
            .any(|s| s.contains("Defined$ ExiledWith") || s.contains("Card.ExiledWithSource"))
            || self.oracle.to_ascii_lowercase().contains("exiled with ")
    }

    /// Whether every line that reads this card's linked exile is the plain
    /// "return the exiled card to the battlefield" (`Defined$ ExiledWith`,
    /// `Origin$ Exile`, `Destination$ Battlefield`, nothing else), which
    /// lowers to `Effect::ReturnLinkedToBattlefield`, and no sentence names
    /// "cards exiled with" this source some other way.
    fn linked_exile_readers_only_return(&self) -> bool {
        if self.oracle.to_ascii_lowercase().contains("exiled with ") {
            return false;
        }
        let mut readers = 0usize;
        for body in self.svars.values() {
            if !body.contains("ExiledWith") {
                continue;
            }
            readers += 1;
            let Some((api, mut rp)) = Params::parse(body) else {
                return false;
            };
            rp.drop_prose();
            if api != "ChangeZone"
                || rp.take("Defined").as_deref() != Some("ExiledWith")
                || rp.take("Origin").as_deref() != Some("Exile")
                || rp.take("Destination").as_deref() != Some("Battlefield")
                || !rp.exhausted()
            {
                return false;
            }
        }
        readers > 0
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn change_zone_with_prior(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        let origin = p.take("Origin")?;
        let destination = p.take("Destination")?;
        p.claim_label("ChangeTypeDesc", "ChangeType");
        if !targets_here
            && !self.on_a_spell
            && origin == "Library"
            && destination == "Exile"
            && p.peek("Defined") == Some("TopOfLibrary")
            && p.peek("ExileFaceDown") == Some("True")
            && !p.has("ChangeNum")
            && !p.has("RememberChanged")
            && !self.oracle.to_ascii_lowercase().contains("look at")
        {
            p.take("Defined");
            p.take("ExileFaceDown");
            return Some(vec!["Effect::ExileTopFaceDownLinked".to_string()]);
        }
        if !targets_here
            && origin == "Library"
            && destination == "Exile"
            && p.peek("Defined") == Some("TopOfLibrary")
            && !p.has("ExileFaceDown")
            && !p.has("RememberChanged")
            && !p.has("Imprint")
            && !p.has("ChangeType")
            && !p.has("Hidden")
        {
            p.take("Defined");
            p.take("IsCurse");
            let who =
                self.wave11_player_of_line(p.take("DefinedPlayer").as_deref(), target, false)?;
            let count = self.amount_or_count_ext(p.take("ChangeNum").as_deref().unwrap_or("1"))?;
            return Some(vec![format!(
                "Effect::ExileTop {{ who: {who}, count: {count} }}"
            )]);
        }
        // A library search is a different effect, not a zone change with a
        // hidden target: a card in a library cannot be targeted at all
        // (CR 115.2 needs a visible object), so `Effect::SearchLibrary`
        // *finds* rather than moves, and carries the shuffle with it.
        if (origin == "Library" && p.peek("OriginAlternative") == Some("Graveyard"))
            || (origin == "Graveyard" && p.peek("OriginAlternative") == Some("Library"))
        {
            p.take("OriginAlternative");
            return self.search_library_or_graveyard(p, &destination, targets_here);
        }
        if origin == "Library" {
            return self.search_library(p, &destination, target, targets_here);
        }
        if matches!(origin.as_str(), "Library,Graveyard" | "Graveyard,Library") {
            return self.search_library_or_graveyard(p, &destination, targets_here);
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
            Some("Parent")
                if self.on_a_spell
                    && origin == "Stack"
                    && matches!(destination.as_str(), "Hand" | "Exile") =>
            {
                true
            }
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
            && matches!(p.peek("DefinedPlayer"), None | Some("You"))
            && matches!(p.peek("Chooser"), None | Some("You"))
            && !p.has("ExileFaceDown")
            && !p.has("AtRandom")
            && !p.has("RememberChanged")
            && (p.peek("Imprint") == Some("True") || !self.has_linked_exile_reader())
        {
            let is_imprint = p.take("Imprint").as_deref() == Some("True");
            p.take("DefinedPlayer");
            p.take("Chooser");
            p.take("IsCurse");
            p.take("SelectPrompt");
            if !matches!(p.take("ChangeNum").as_deref(), None | Some("1")) {
                return None;
            }
            if !matches!(p.take("Hidden").as_deref(), None | Some("True")) {
                return None;
            }
            let optional = match (
                p.take("Optional").as_deref(),
                p.take("Mandatory").as_deref(),
            ) {
                (None | Some("False"), Some("True")) => false,
                (None | Some("False"), None) if is_imprint => false,
                (Some("True"), None | Some("False")) => true,
                (None, None) if self.oracle.to_ascii_lowercase().contains("you may exile") => true,
                _ => return None,
            };
            let valid = p.take("ChangeType")?;
            let expr = self.filter_expr_in_zone(&valid, true)?;
            let filter = self.body.filter_static("IMPRINT", &expr);
            let inner = format!("Effect::ExileChosenFromHandWithSource(&{filter})");
            return Some(vec![if optional {
                format!("Effect::MayDo {{ effects: &[{inner}] }}")
            } else {
                inner
            }]);
        }
        if !itself
            && origin == "Hand"
            && destination == "Library"
            && (matches!(
                target,
                "TargetSpec::AnyPlayer" | "TargetSpec::Player(PlayerRel::Chosen)"
            ) || (prior_target_used && !targets_here))
            && matches!(p.peek("LibraryPosition"), None | Some("0" | "-1"))
            && !p.has("Shuffle")
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
            && ((target == "TargetSpec::AnyPlayer" || prior_target_used) && !targets_here
                || target == "TargetSpec::Player(PlayerRel::Chosen)")
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
                    && (p.peek("SelectPrompt").is_some_and(|s| {
                        let lower = s.to_ascii_lowercase();
                        lower.starts_with("you may")
                            || lower.starts_with("select up to ")
                            || lower.starts_with("choose up to ")
                    }) || {
                        let lower = self.oracle.to_ascii_lowercase();
                        lower.contains("you may ") || lower.contains("up to ")
                    })))
        {
            return self.choose_from_your_graveyard(p, &destination, target);
        }
        // A spell moving itself from the stack sometimes omits `Defined$ Self`
        // (e.g. `Origin$ Stack | Destination$ Exile` / `Hand`).
        if self.on_a_spell
            && !itself
            && target == "TargetSpec::AnyPlayer"
            && origin == "Stack"
            && matches!(destination.as_str(), "Exile" | "Hand")
            && !p.has("ChangeType")
            && !p.has("Defined")
        {
            p.take("IsCurse");
            if p.exhausted() {
                return Some(vec![if destination == "Exile" {
                    "Effect::ExileSource".to_string()
                } else {
                    "Effect::bounce(TargetSpec::ThisObject)".to_string()
                }]);
            }
        }
        // Without a target this would move nothing at all.
        if !itself && target == "TargetSpec::AnyPlayer" {
            return self.deny("`ChangeZone` with neither a target nor `Defined$`".to_string());
        }
        // "Its owner puts it on the top or bottom of their library"
        // (`Subtlety`, `Aether Gust`, `Horned Loch-Whale`, `Lost in Space`).
        if !itself
            && single_target
            && destination == "Library"
            && p.peek("DestinationAlternative") == Some("Library")
            && p.peek("LibraryPositionAlternative") == Some("-1")
            && p.peek("AlternativeDecider") == Some("TargetedOwner")
            && matches!(p.peek("LibraryPosition"), None | Some("0"))
            && !p.has("Shuffle")
        {
            let zone_ok = match origin.as_str() {
                "Battlefield" => target.starts_with("TargetSpec::Object("),
                "Stack" => {
                    target.starts_with("TargetSpec::Spell(")
                        && matches!(
                            p.take("TargetType").as_deref(),
                            None | Some("Spell" | "Spell.OppCtrl" | "Spell.YouDontCtrl")
                        )
                }
                "Battlefield,Stack" | "Stack,Battlefield" => {
                    p.take("UseAllOriginZones");
                    target.starts_with("TargetSpec::StackOrBattlefield(")
                }
                _ => false,
            };
            if zone_ok {
                p.take("DestinationAlternative");
                p.take("LibraryPositionAlternative");
                p.take("AlternativeDecider");
                p.take("LibraryPosition");
                p.take("IsCurse");
                return Some(vec![format!(
                    "Effect::OwnerPutsOnTopOrBottom {{ target: {target} }}"
                )]);
            }
        }
        Some(match (origin.as_str(), destination.as_str(), itself) {
            ("Battlefield" | "Graveyard" | "Stack" | "Exile", "Hand", true) => {
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
            ("Stack", "Hand", false) if target.starts_with("TargetSpec::Spell(") => {
                p.take("IsCurse");
                if !matches!(
                    p.take("TargetType").as_deref(),
                    None | Some("Spell" | "Spell.OppCtrl" | "Spell.YouDontCtrl")
                ) {
                    return None;
                }
                vec![format!("Effect::bounce({target})")]
            }
            ("Stack,Battlefield" | "Battlefield,Stack", "Hand", false)
                if target.starts_with("TargetSpec::StackOrBattlefield(") =>
            {
                p.take("IsCurse");
                p.take("UseAllOriginZones");
                vec![format!("Effect::bounce({target})")]
            }
            ("Stack,Battlefield" | "Battlefield,Stack", "Exile", false)
                if target.starts_with("TargetSpec::StackOrBattlefield(")
                    && !self.has_linked_exile_reader() =>
            {
                p.take("IsCurse");
                p.take("UseAllOriginZones");
                vec![format!("Effect::exile({target})")]
            }
            ("Stack", "Exile", false)
                if target.starts_with("TargetSpec::Spell(") && !self.has_linked_exile_reader() =>
            {
                p.take("IsCurse");
                if !matches!(
                    p.take("TargetType").as_deref(),
                    None | Some("Spell" | "Spell.OppCtrl" | "Spell.YouDontCtrl")
                ) {
                    return None;
                }
                vec![format!("Effect::exile({target})")]
            }
            ("Stack", "Library", false)
                if target.starts_with("TargetSpec::Spell(")
                    && !p.has("Shuffle")
                    && !p.has("DestinationAlternative") =>
            {
                p.take("IsCurse");
                if !matches!(
                    p.take("TargetType").as_deref(),
                    None | Some("Spell" | "Spell.OppCtrl" | "Spell.YouDontCtrl")
                ) {
                    return None;
                }
                match p.take("LibraryPosition").as_deref() {
                    Some("0") => vec!["Effect::PutTargetOnTopOfLibrary".to_string()],
                    Some("-1") => vec!["Effect::PutTargetOnBottomOfLibrary".to_string()],
                    Some(pos) => {
                        let count = self.beneath_top_library_position(pos)?;
                        vec![format!(
                            "Effect::PutTargetIntoLibraryBeneathTop {{ count: {count} }}"
                        )]
                    }
                    None => {
                        return self.deny("`ChangeZone` from `Stack` to `Library`".to_string());
                    }
                }
            }
            ("Battlefield", "Library", true)
                if p.take("LibraryPosition").as_deref() == Some("0") =>
            {
                vec!["Effect::PutSourceOnTopOfLibrary".to_string()]
            }
            ("Battlefield", "Library", false)
                if target.starts_with("TargetSpec::Object(")
                    && !p.has("DestinationAlternative") =>
            {
                p.take("IsCurse");
                match (
                    p.take("Shuffle").as_deref(),
                    p.take("LibraryPosition").as_deref(),
                ) {
                    (Some("True"), None | Some("0" | "-1")) => {
                        vec!["Effect::ShuffleTargetsIntoOwnersLibraries".to_string()]
                    }
                    (None, Some("0")) => {
                        vec!["Effect::PutTargetOnTopOfLibrary".to_string()]
                    }
                    (None, Some("-1")) => {
                        vec!["Effect::PutTargetOnBottomOfLibrary".to_string()]
                    }
                    (None, Some(pos)) => {
                        let count = self.beneath_top_library_position(pos)?;
                        vec![format!(
                            "Effect::PutTargetIntoLibraryBeneathTop {{ count: {count} }}"
                        )]
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
                    let each_opp = target.starts_with("TargetSpec::ObjectOfEachOpponent(");
                    if self.on_a_spell
                        || (!single_target && !each_opp)
                        || (!target.starts_with("TargetSpec::Object(") && !each_opp)
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
            ("Battlefield" | "Graveyard" | "Stack" | "Hand", "Exile", true) => {
                vec!["Effect::ExileSource".to_string()]
            }
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
                match (
                    p.take("Shuffle").as_deref(),
                    p.take("LibraryPosition").as_deref(),
                ) {
                    (Some("True"), None) => {
                        vec!["Effect::ShuffleTargetsIntoOwnersLibraries".to_string()]
                    }
                    (None, None | Some("0")) => {
                        vec![format!("Effect::GraveyardToTop {{ target: {target} }}")]
                    }
                    (None, Some("-1")) => vec![format!(
                        "Effect::PutOnBottomOfLibraryFromGraveyard {{ target: {target} }}"
                    )],
                    _ => {
                        return self.deny("`ChangeZone` from `Graveyard` to `Library`".to_string());
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
            && origin != "Hand"
            && !p.has("Hidden")
            && !p.has("DefinedPlayer")
            && (target != "TargetSpec::AnyPlayer"
                || (self.on_a_spell
                    && origin == "Stack"
                    && matches!(destination, "Exile" | "Hand")
                    && !p.has("ChangeType"))
                || (!self.on_a_spell
                    && ((origin == "Battlefield"
                        && (matches!(destination, "Hand" | "Exile")
                            || (destination == "Library"
                                && p.peek("LibraryPosition") == Some("0"))))
                        || (origin == "Graveyard"
                            && matches!(destination, "Battlefield" | "Hand" | "Exile"))
                        || (origin == "Exile" && matches!(destination, "Hand" | "Graveyard")))
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
        p.take("SelectPrompt");
        let pos = p.take("LibraryPosition").unwrap_or_else(|| "0".to_string());
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
            if let Some(reorder) = p.take("Reorder")
                && reorder != "True"
            {
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

    fn strip_graveyard_owner(valid: &str) -> Option<(Option<&'static str>, String)> {
        if let Some(stripped) = Self::strip_your_graveyard_owner(valid) {
            return Some((Some("PlayerRel::You"), stripped));
        }
        let mut owner: Option<&'static str> = None;
        let mut alts = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = alt.split_once('.').unwrap_or((alt, ""));
            let mut alt_owner: Option<&'static str> = None;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                let parsed = match atom {
                    "OppOwn" | "OppCtrl" | "YouDontOwn" => Some("PlayerRel::EachOpponent"),
                    "TargetedPlayerOwn" | "TargetedPlayerCtrl" => Some("PlayerRel::Chosen"),
                    _ if atom.contains("Ctrl") || atom.contains("Own") => return None,
                    _ => None,
                };
                if let Some(rel) = parsed {
                    if alt_owner.is_some_and(|prev| prev != rel) {
                        return None;
                    }
                    alt_owner = Some(rel);
                } else {
                    kept.push(atom);
                }
            }
            if let Some(rel) = alt_owner {
                if owner.is_some_and(|prev| prev != rel) {
                    return None;
                }
                owner = Some(rel);
            } else if owner.is_some() {
                return None;
            }
            if kept.is_empty() {
                alts.push(base.to_string());
            } else {
                alts.push(format!("{base}.{}", kept.join("+")));
            }
        }
        Some((owner, alts.join(",")))
    }

    fn parse_graveyard_chooser_and_owner(
        &self,
        p: &mut Params,
        target: &str,
        targets_a_player: bool,
    ) -> Option<(&'static str, Option<&'static str>)> {
        let defined_owner = match p.take("DefinedPlayer").as_deref() {
            None => None,
            Some("You") => Some("PlayerRel::You"),
            Some("Opponent" | "Player.Opponent") => Some("PlayerRel::EachOpponent"),
            Some("Player") => Some("PlayerRel::EachPlayer"),
            Some(who) => {
                Some(self.wave11_player_of_line(Some(who), Some(target), targets_a_player)?)
            }
        };
        // With no `Chooser$`, the player whose graveyard is searched does the
        // choosing ("that player exiles two cards from their graveyard"); only
        // a single named player can, so a group owner keeps the controller.
        let chooser = match p.take("Chooser").as_deref() {
            None => match defined_owner {
                Some(owner) if Self::is_single_player_rel(owner) => owner,
                _ => "PlayerRel::You",
            },
            Some("You") => "PlayerRel::You",
            Some(who) => {
                let rel = self.wave11_player_of_line(Some(who), Some(target), targets_a_player)?;
                if !Self::is_single_player_rel(rel) {
                    return None;
                }
                rel
            }
        };
        Some((chooser, defined_owner))
    }

    fn parse_graveyard_optional(&self, p: &mut Params) -> Option<bool> {
        let prompt_opt = p.take("SelectPrompt").is_some_and(|s| {
            let lower = s.to_ascii_lowercase();
            lower.starts_with("you may")
                || lower.starts_with("select up to ")
                || lower.starts_with("choose up to ")
        });
        match (
            p.take("Mandatory").as_deref(),
            p.take("Optional").as_deref(),
        ) {
            (Some("True"), None) => Some(false),
            (None, Some("True")) => Some(true),
            (None, None)
                if prompt_opt || {
                    let lower = self.oracle.to_ascii_lowercase();
                    lower.contains("you may ") || lower.contains("up to ")
                } =>
            {
                Some(true)
            }
            _ => None,
        }
    }

    fn choose_from_your_graveyard(
        &mut self,
        p: &mut Params,
        destination: &str,
        target: &str,
    ) -> Option<Vec<String>> {
        if p.take("Hidden").as_deref() != Some("True") {
            return None;
        }
        p.take("IsCurse");
        p.claim_label("ChangeTypeDesc", "ChangeType");
        let targets_a_player = target == "TargetSpec::Player(PlayerRel::Chosen)";
        let (chooser, defined_owner) =
            self.parse_graveyard_chooser_and_owner(p, target, targets_a_player)?;
        let optional = self.parse_graveyard_optional(p)?;
        let num_raw = p.take("ChangeNum").unwrap_or_else(|| "1".to_string());
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
        let lib_pos = if destination == "Library" {
            Some(p.take("LibraryPosition")?)
        } else {
            None
        };
        let valid = p.take("ChangeType")?;
        let (filter_owner, stripped) = Self::strip_graveyard_owner(&valid)?;
        let owner_rel = match (defined_owner, filter_owner) {
            (Some(a), Some(b)) if a != b => return None,
            (Some(a), _) | (_, Some(a)) => a,
            (None, None) => "PlayerRel::EachPlayer",
        };
        if owner_rel == "PlayerRel::Chosen" && !targets_a_player {
            return None;
        }
        // `ChooseMove::Battlefield` puts chosen cards onto the battlefield under
        // `you`'s control (CR 110.2a), so non-own graveyards require explicit
        // `GainControl$ True`.
        if destination == "Battlefield" && owner_rel != "PlayerRel::You" && !gain_control {
            return None;
        }
        if destination == "Exile" && self.has_linked_exile_reader() {
            return self.deny("`ChooseMove::Exile` on a card referencing `ExiledWith`".to_string());
        }
        let expr = self.filter_expr_in_zone(&stripped, true)?;
        let filter = self.body.filter_static("GRAVE_CHOICE", &expr);
        if num_raw == "1" && owner_rel == "PlayerRel::You" && chooser == "PlayerRel::You" {
            let dest = match (destination, lib_pos.as_deref()) {
                ("Battlefield", None) => Some("SearchDest::Battlefield"),
                ("Hand", None) => Some("SearchDest::Hand"),
                ("Exile", None) => Some("SearchDest::Exile"),
                ("Library", Some("0")) => Some("SearchDest::TopOfLibrary"),
                _ => None,
            };
            if let Some(dest) = dest {
                return Some(vec![format!(
                    "Effect::ChooseFromYourGraveyard {{ filter: &{filter}, destination: {dest}, \
                     tapped: {tapped}, optional: {optional} }}"
                )]);
            }
        }
        let movement = match (destination, lib_pos.as_deref()) {
            ("Battlefield", None) => format!("ChooseMove::Battlefield {{ tapped: {tapped} }}"),
            ("Hand", None) => "ChooseMove::Hand".to_string(),
            ("Exile", None) => "ChooseMove::Exile".to_string(),
            ("Library", Some("0")) => "ChooseMove::TopOfLibrary".to_string(),
            ("Library", Some("-1")) => "ChooseMove::BottomOfLibrary".to_string(),
            _ => return None,
        };
        let (min, max) = if optional {
            let max = self.amount_or_count_ext(&num_raw)?;
            if max == "Amount::Fixed(0)" {
                return None;
            }
            (0u8, max)
        } else {
            let n: u8 = self.plain_num(&num_raw)?.try_into().ok()?;
            if n == 0 {
                return None;
            }
            (n, format!("Amount::Fixed({n})"))
        };
        Some(vec![format!(
            "Effect::ChooseCard {{ chooser: {chooser}, \
             zone: ChooseZone::Graveyard({owner_rel}), filter: &{filter}, \
             min: {min}, max: {max}, movement: {movement}, then: &[] }}"
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
    #[allow(clippy::too_many_lines)]
    pub(super) fn return_chosen(
        &mut self,
        p: &mut Params,
        destination: &str,
    ) -> Option<Vec<String>> {
        if destination != "Hand" {
            let movement = match destination {
                "Exile" => {
                    if self.has_linked_exile_reader() {
                        // "Exile a land you control. When this leaves the
                        // battlefield, return the exiled card" (CR 607.2a):
                        // when every reader of the link is that return, the
                        // chosen permanent is exiled linked to this source.
                        if !self.linked_exile_readers_only_return() {
                            return self.deny(
                                "`ChooseMove::Exile` on a card referencing `ExiledWith`"
                                    .to_string(),
                            );
                        }
                        "ChooseMove::ExileLinked"
                    } else {
                        "ChooseMove::Exile"
                    }
                }
                "Library" => match p.take("LibraryPosition").as_deref() {
                    Some("0") => "ChooseMove::TopOfLibrary",
                    Some("-1") => "ChooseMove::BottomOfLibrary",
                    _ => {
                        self.note(format!(
                            "`ChangeZone` chosen off the battlefield into {destination}"
                        ));
                        return None;
                    }
                },
                _ => {
                    self.note(format!(
                        "`ChangeZone` chosen off the battlefield into {destination}"
                    ));
                    return None;
                }
            };
            if p.take("Hidden").as_deref() != Some("True") {
                self.note(format!(
                    "`ChangeZone` chosen off the battlefield into {destination}"
                ));
                return None;
            }
            p.take("SelectPrompt");
            p.claim_label("ChangeTypeDesc", "ChangeType");
            let optional = match (
                p.take("Mandatory").as_deref(),
                p.take("Optional").as_deref(),
            ) {
                (Some("True"), None) => false,
                (None, Some("True"))
                    if destination == "Exile" && !self.has_linked_exile_reader() =>
                {
                    true
                }
                _ => {
                    self.note(format!(
                        "`ChangeZone` chosen off the battlefield into {destination}"
                    ));
                    return None;
                }
            };
            let n: u8 = self
                .plain_num(p.take("ChangeNum").as_deref().unwrap_or("1"))?
                .try_into()
                .ok()?;
            if n == 0 {
                return None;
            }
            let defined_player = p.take("DefinedPlayer");
            let chooser_raw = p.take("Chooser");
            let change_type = p.take("ChangeType")?;
            let (chooser, filter_expr) = match (defined_player.as_deref(), chooser_raw.as_deref()) {
                (None | Some("You"), None | Some("You")) => {
                    ("PlayerRel::You", self.filter_expr(&change_type)?)
                }
                (Some("TriggeredDefendingPlayer"), Some("DefendingPlayer"))
                    if !optional
                        && matches!(
                            self.trigger_mode.as_deref(),
                            Some("Attacks" | "AttackerBlocked" | "AttackerUnblocked")
                        )
                        && !change_type.contains("Ctrl")
                        && !change_type.contains("Own") =>
                {
                    let base = self.filter_expr(&change_type)?;
                    let scoped = if base == "Filter::Any" {
                        "Filter::ControlledByDefendingPlayer".to_string()
                    } else {
                        format!("Filter::And(&[{base}, Filter::ControlledByDefendingPlayer])")
                    };
                    ("PlayerRel::DefendingPlayer", scoped)
                }
                _ => return None,
            };
            let min = if optional { 0 } else { n };
            let name = self.body.filter_static("CHOOSE_BF", &filter_expr);
            return Some(vec![format!(
                "Effect::ChooseCard {{ chooser: {chooser}, zone: ChooseZone::Battlefield, \
                 filter: &{name}, min: {min}, max: Amount::Fixed({n}), \
                 movement: {movement}, then: &[] }}"
            )]);
        }
        match p.take("Hidden").as_deref() {
            Some("True") => {}
            Some(other) => {
                self.note(format!("`ChangeZone` with `Hidden$ {other}`"));
                return None;
            }
            None => {
                self.note("`ChangeZone` chosen without `Hidden$`".to_string());
                return None;
            }
        }
        let is_sweep_any_num = !p.has("Mandatory")
            && !p.has("Optional")
            && matches!(p.peek("DefinedPlayer"), None | Some("You"))
            && matches!(p.peek("Chooser"), None | Some("You"))
            && p.peek("ChangeNum")
                .zip(p.peek("ChangeType"))
                .is_some_and(|(num_var, ct)| {
                    self.svars
                        .get(num_var.trim())
                        .is_some_and(|def| def.trim() == format!("Count$Valid {ct}"))
                });
        if (p.peek("Optional") == Some("True") && !p.has("Mandatory")) || is_sweep_any_num {
            if !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
                || !matches!(p.take("Chooser").as_deref(), None | Some("You"))
            {
                return None;
            }
            p.take("Optional");
            p.take("SelectPrompt");
            p.claim_label("ChangeTypeDesc", "ChangeType");
            let change_type = p.take("ChangeType")?;
            let max_expr = if is_sweep_any_num {
                p.take("ChangeNum");
                "Amount::Fixed(255)".to_string()
            } else {
                let raw_num = p.take("ChangeNum").unwrap_or_else(|| "1".to_string());
                if self
                    .svars
                    .get(raw_num.trim())
                    .is_some_and(|def| def.trim() == format!("Count$Valid {change_type}"))
                {
                    "Amount::Fixed(255)".to_string()
                } else {
                    let n: u8 = self.plain_num(&raw_num)?.try_into().ok()?;
                    if n == 0 {
                        return None;
                    }
                    format!("Amount::Fixed({n})")
                }
            };
            let filter = self.filter_expr(&change_type)?;
            let name = self.body.filter_static("CHOOSE_BF", &filter);
            return Some(vec![format!(
                "Effect::ChooseCard {{ chooser: PlayerRel::You, zone: ChooseZone::Battlefield, \
                 filter: &{name}, min: 0, max: {max_expr}, \
                 movement: ChooseMove::Hand, then: &[] }}"
            )]);
        }
        match p.take("Mandatory").as_deref() {
            Some("True") => {}
            Some(other) => {
                self.note(format!("`ChangeZone` with `Mandatory$ {other}`"));
                return None;
            }
            None => {
                self.note("`ChangeZone` chosen without `Mandatory$`".to_string());
                return None;
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
        if !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You")) {
            return self.deny("`ChangeZone` from hand naming another `DefinedPlayer$`".to_string());
        }
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
    #[allow(clippy::too_many_lines)]
    pub(super) fn search_library(
        &mut self,
        p: &mut Params,
        destination: &str,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        let targets_a_player =
            targets_here && target == Some("TargetSpec::Player(PlayerRel::Chosen)");
        if targets_here && !targets_a_player {
            self.note("`ChangeZone` searching a library *and* targeting".to_string());
            return None;
        }
        let defined_player = p.take("DefinedPlayer");
        let chooser_raw = p.take("Chooser");
        if targets_a_player {
            if !matches!(
                defined_player.as_deref(),
                None | Some("Targeted" | "TargetedPlayer")
            ) {
                return self.deny(
                    "`ChangeZone` targeting a player with mismatched `DefinedPlayer$`".into(),
                );
            }
            p.take("IsCurse");
        }
        let (group_domain, library_rel) = match defined_player.as_deref() {
            None if !targets_a_player => (None, "PlayerRel::You"),
            Some("You") => (None, "PlayerRel::You"),
            Some("Player") => (Some("RepeatDomain::EachPlayer"), "PlayerRel::EventPlayer"),
            Some("Opponent" | "Player.Opponent") => {
                (Some("RepeatDomain::EachOpponent"), "PlayerRel::EventPlayer")
            }
            other => {
                let rel = self.wave11_player_of_line(other, target, targets_a_player)?;
                if !Self::is_single_player_rel(rel) {
                    return self.deny(format!(
                        "`ChangeZone` searching library of `DefinedPlayer$ {}`",
                        other.unwrap_or_default()
                    ));
                }
                (None, rel)
            }
        };
        let is_own_library = group_domain.is_none() && library_rel == "PlayerRel::You";
        let owner_searches = if is_own_library {
            if !matches!(chooser_raw.as_deref(), None | Some("You")) {
                return self
                    .deny("`ChangeZone` searching your library with another `Chooser$`".into());
            }
            true
        } else if group_domain.is_some() {
            match chooser_raw.as_deref() {
                None => true,
                Some(c) if Some(c) == defined_player.as_deref() => true,
                Some(c) => {
                    return self.deny(format!(
                        "`ChangeZone` group library search with `Chooser$ {c}`"
                    ));
                }
            }
        } else {
            match chooser_raw.as_deref() {
                None => true,
                Some(c) if Some(c) == defined_player.as_deref() => true,
                Some("Targeted" | "TargetedPlayer") if library_rel == "PlayerRel::Chosen" => true,
                Some("You") => false,
                Some(c) => {
                    return self.deny(format!("`ChangeZone` library search with `Chooser$ {c}`"));
                }
            }
        };
        if destination == "Library"
            && is_own_library
            && p.peek("Shuffle") == Some("False")
            && p.peek("Imprint") == Some("True")
            && p.peek("Reveal") == Some("True")
        {
            return self.reveal_library_search(p);
        }
        let gain_control = match p.take("GainControl").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`ChangeZone` found `GainControl$ {other}`"));
            }
        };
        if !owner_searches {
            match destination {
                "Battlefield" if gain_control => {}
                "Exile" | "Graveyard" | "Library" if !gain_control => {}
                _ => {
                    return self.deny(format!(
                        "`ChangeZone` searching another player's library into `{destination}`"
                    ));
                }
            }
        } else if gain_control {
            return self.deny("`GainControl$ True` when library owner searches".into());
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Tapped$ {other}`"));
                return None;
            }
        };
        let any_number = if is_own_library {
            self.any_number_library_search(p, destination)?
        } else {
            false
        };
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
        let reveal_requested = match p.take("Reveal").as_deref() {
            None => false,
            Some("True") => true,
            Some(value) => {
                self.note(format!("`ChangeZone` with `Reveal$ {value}`"));
                return None;
            }
        };
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
        let lower_oracle = self.oracle.to_ascii_lowercase();
        let up_to = lower_oracle.contains("search your library for up to ")
            || (!is_own_library
                && (lower_oracle.contains("search their library for up to ")
                    || lower_oracle.contains("library for up to ")));
        let raw_num = p.take("ChangeNum");
        if is_own_library
            && let Some(var) = raw_num.as_deref()
            && let Some(pair) = self
                .svars
                .get(var.trim())
                .and_then(|s| s.trim().strip_prefix("Count$Threshold."))
            && let Some((hi_raw, lo_raw)) = pair.split_once('.')
            && let (Ok(hi), Ok(lo)) = (hi_raw.parse::<usize>(), lo_raw.parse::<usize>())
            && (1..=4).contains(&hi)
            && (1..=4).contains(&lo)
            && !any_number
        {
            let filter = self.filter_expr_in_zone(&p.take("ChangeType")?, true)?;
            if reveal_requested
                && filter == "Filter::Any"
                && !matches!(destination, "Battlefield" | "Exile" | "Graveyard")
            {
                return self.deny("`ChangeZone` revealing an unrestricted hidden search".into());
            }
            let optional_hi =
                self.resolve_search_optional(hi, &filter, optional, mandatory, up_to)?;
            let optional_lo =
                self.resolve_search_optional(lo, &filter, optional, mandatory, up_to)?;
            let name = self.body.filter_static("SEARCH", &filter);
            let finds_hi = vec![find.as_str(); hi].join(", ");
            let finds_lo = vec![find.as_str(); lo].join(", ");
            return Some(vec![format!(
                "Effect::IfCondition {{ condition: Condition::GraveyardCountAtLeast(7), \
                 then: &[Effect::SearchLibrary {{ filter: &{name}, finds: &[{finds_hi}], \
                 optional: {optional_hi} }}], \
                 otherwise: &[Effect::SearchLibrary {{ filter: &{name}, finds: &[{finds_lo}], \
                 optional: {optional_lo} }}] }}"
            )]);
        }
        let count = self.plain_num(raw_num.as_deref().unwrap_or("1"))?;
        let count = usize::try_from(count).ok()?;
        if count == 0 || count > 4 {
            self.note(format!("`ChangeZone` finding {count} cards"));
            return None;
        }
        let filter = self.filter_expr_in_zone(&p.take("ChangeType")?, true)?;
        if reveal_requested
            && filter == "Filter::Any"
            && !matches!(destination, "Battlefield" | "Exile" | "Graveyard")
        {
            return self.deny("`ChangeZone` revealing an unrestricted hidden search".into());
        }
        let optional = self.resolve_search_optional(count, &filter, optional, mandatory, up_to)?;
        let name = self.body.filter_static("SEARCH", &filter);
        let finds = vec![find.as_str(); count].join(", ");
        if is_own_library {
            Some(vec![format!(
                "Effect::SearchLibrary {{ filter: &{name}, finds: &[{finds}], optional: {optional} }}"
            )])
        } else if let Some(domain) = group_domain {
            Some(vec![format!(
                "Effect::RepeatEach {{ domain: {domain}, effects: &[Effect::SearchLibraryOf {{ \
                 library: PlayerRel::EventPlayer, owner_searches: true, filter: &{name}, \
                 mana_value: None, finds: &[{finds}], optional: {optional} }}] }}"
            )])
        } else {
            Some(vec![format!(
                "Effect::SearchLibraryOf {{ library: {library_rel}, owner_searches: {owner_searches}, \
                 filter: &{name}, mana_value: None, finds: &[{finds}], optional: {optional} }}"
            )])
        }
    }

    fn beneath_top_library_position(&mut self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.starts_with('-') {
            return None;
        }
        if let Ok(n) = trimmed.parse::<u32>() {
            return (n >= 1).then(|| format!("Amount::Fixed({n})"));
        }
        let amt = self.amount_or_count_ext(trimmed)?;
        (amt != "Amount::Fixed(0)").then_some(amt)
    }

    /// `Origin$ Library` with `Shuffle$ False` when a later unconditional step
    /// in the same `SubAbility$` chain shuffles your library ->
    /// `Effect::SearchLibraryBeforeShuffle`.
    pub(super) fn search_library_before_shuffle(
        &mut self,
        p: &mut Params,
        sub: Option<&str>,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || p.peek("Origin") != Some("Library")
            || p.peek("Shuffle") != Some("False")
            || p.has("RememberChanged")
            || p.has("Imprint")
            || !matches!(p.peek("DefinedPlayer"), None | Some("You"))
            || !self.chain_shuffles_your_library(sub)
        {
            return None;
        }
        p.take("Origin");
        let destination = p.take("Destination")?;
        p.claim_label("ChangeTypeDesc", "ChangeType");
        p.take("Shuffle");
        let mut effects = self.search_library(p, &destination, target, targets_here)?;
        if effects.len() != 1 || !effects[0].starts_with("Effect::SearchLibrary {") {
            return None;
        }
        effects[0] = effects[0].replacen(
            "Effect::SearchLibrary {",
            "Effect::SearchLibraryBeforeShuffle {",
            1,
        );
        Some(effects)
    }

    fn chain_shuffles_your_library(&self, sub: Option<&str>) -> bool {
        let mut cursor = sub.map(str::trim);
        let mut seen: Vec<&str> = Vec::new();
        while let Some(name) = cursor {
            if seen.len() >= 8 || seen.contains(&name) {
                return false;
            }
            seen.push(name);
            let Some(body) = self.svars.get(name) else {
                return false;
            };
            let Some((api, mut p)) = Params::parse(body) else {
                return false;
            };
            p.drop_prose();
            if p.has("UnlessCost") || p.entries.iter().any(|(k, _)| k.starts_with("Condition")) {
                return false;
            }
            if api == "Shuffle" && matches!(p.peek("Defined"), None | Some("You")) {
                return true;
            }
            if api == "ChangeZone"
                && matches!(
                    p.peek("Origin"),
                    Some("Library" | "Library,Graveyard" | "Graveyard,Library")
                )
                && matches!(p.peek("DefinedPlayer"), None | Some("You"))
                && p.peek("Shuffle") != Some("False")
            {
                return true;
            }
            cursor = p.peek("SubAbility").and_then(|next| {
                self.svars
                    .get_key_value(next.trim())
                    .map(|(k, _)| k.as_str())
            });
        }
        false
    }

    /// Split-destination library search (`Cultivate`, `Peregrination`, `Jarad's
    /// Orders`, `Viewpoint Synchronization`, `Navigation Orb`):
    /// `Origin$ Library | Destination$ Library | RememberChanged$ True |
    /// Reveal$ True | Shuffle$ False` followed by two `NoLooking$ True` /
    /// `Defined$ Remembered` splits -> `Effect::SearchLibrary` with
    /// heterogeneous `finds`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn split_search_library_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || p.take("Origin").as_deref() != Some("Library")
            || p.take("Destination").as_deref() != Some("Library")
            || p.take("RememberChanged").as_deref() != Some("True")
            || p.take("Reveal").as_deref() != Some("True")
            || p.take("Shuffle").as_deref() != Some("False")
            || !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
            || !matches!(p.take("Chooser").as_deref(), None | Some("You"))
            || !matches!(p.take("Mandatory").as_deref(), None | Some("False"))
            || !matches!(p.take("Optional").as_deref(), None | Some("True"))
            || !matches!(p.take("Hidden").as_deref(), None | Some("True"))
        {
            return None;
        }
        p.claim_label("ChangeTypeDesc", "ChangeType");
        let total = usize::try_from(self.plain_num(&p.take("ChangeNum")?)?).ok()?;
        if !(2..=4).contains(&total) {
            return None;
        }
        let valid = p.take("ChangeType")?;
        if !p.exhausted() {
            return None;
        }
        let filter = self.filter_expr_in_zone(&valid, true)?;
        if filter == "Filter::Any" {
            return None;
        }

        let db1_name = sub.take()?;
        let db1_body = self.svars.get(db1_name.trim())?.clone();
        let (api1, mut sp1) = Params::parse(&db1_body)?;
        sp1.drop_prose();
        if api1 != "ChangeZone"
            || sp1.take("Origin").as_deref() != Some("Library")
            || sp1.take("Mandatory").as_deref() != Some("True")
            || sp1.take("NoLooking").as_deref() != Some("True")
            || !matches!(sp1.take("DefinedPlayer").as_deref(), None | Some("You"))
            || !matches!(sp1.take("Chooser").as_deref(), None | Some("You"))
            || !matches!(sp1.take("Hidden").as_deref(), None | Some("True"))
            || !matches!(sp1.take("ForgetChanged").as_deref(), None | Some("True"))
        {
            return None;
        }
        sp1.take("SelectPrompt");
        let ct1 = sp1.take("ChangeType")?;
        if !Self::all_alts_remembered(&ct1) {
            return None;
        }
        let find1 = Self::split_search_find(&mut sp1)?;
        let count1 =
            usize::try_from(self.plain_num(sp1.take("ChangeNum").as_deref().unwrap_or("1"))?)
                .ok()?;
        if count1 == 0 || count1 >= total {
            return None;
        }
        let shuffles1 = match sp1.take("Shuffle").as_deref() {
            None | Some("True") => true,
            Some("False") => false,
            Some(_) => return None,
        };
        let db2_name = sp1.take("SubAbility")?;
        if !sp1.exhausted() {
            return None;
        }

        let db2_body = self.svars.get(db2_name.trim())?.clone();
        let (api2, mut sp2) = Params::parse(&db2_body)?;
        sp2.drop_prose();
        if api2 != "ChangeZone" || sp2.take("Origin").as_deref() != Some("Library") {
            return None;
        }
        let find2 = Self::split_search_find(&mut sp2)?;
        let (count2, shuffles2) = if sp2.peek("Defined") == Some("Remembered") {
            sp2.take("Defined");
            if sp2.take("ConditionDefined").as_deref() != Some("Remembered")
                || sp2.take("ConditionPresent").as_deref() != Some("Card")
                || !matches!(sp2.take("ForgetChanged").as_deref(), None | Some("True"))
            {
                return None;
            }
            let shuf = match sp2.take("Shuffle").as_deref() {
                Some("True") => true,
                None | Some("False") => false,
                Some(_) => return None,
            };
            (total - count1, shuf)
        } else {
            if sp2.take("Mandatory").as_deref() != Some("True")
                || sp2.take("NoLooking").as_deref() != Some("True")
                || !matches!(sp2.take("DefinedPlayer").as_deref(), None | Some("You"))
                || !matches!(sp2.take("Chooser").as_deref(), None | Some("You"))
                || !matches!(sp2.take("Hidden").as_deref(), None | Some("True"))
                || !matches!(sp2.take("ForgetChanged").as_deref(), None | Some("True"))
            {
                return None;
            }
            sp2.take("SelectPrompt");
            let ct2 = sp2.take("ChangeType")?;
            if !Self::all_alts_remembered(&ct2) {
                return None;
            }
            let c2 = match sp2.take("ChangeNum") {
                None => 1,
                Some(raw) => usize::try_from(self.plain_num(&raw)?).ok()?,
            };
            if count1 + c2 != total {
                return None;
            }
            let shuf = match sp2.take("Shuffle").as_deref() {
                None | Some("True") => true,
                Some("False") => false,
                Some(_) => return None,
            };
            (c2, shuf)
        };
        let mut after = sp2.take("SubAbility");
        if !sp2.exhausted() {
            return None;
        }

        let mut shuffled = shuffles1 || shuffles2;
        for _ in 0..2 {
            let Some(ref next_name) = after else {
                break;
            };
            let Some(next_body) = self.svars.get(next_name.trim()).cloned() else {
                break;
            };
            let Some((next_api, mut np)) = Params::parse(&next_body) else {
                break;
            };
            np.drop_prose();
            if next_api == "Cleanup" && np.take("ClearRemembered").as_deref() == Some("True") {
                let tail = np.take("SubAbility");
                if np.exhausted() {
                    after = tail;
                    continue;
                }
            }
            if !shuffled
                && next_api == "Shuffle"
                && matches!(np.take("Defined").as_deref(), None | Some("You"))
            {
                let tail = np.take("SubAbility");
                if np.exhausted() {
                    shuffled = true;
                    after = tail;
                    continue;
                }
            }
            break;
        }
        if !shuffled {
            return None;
        }
        *sub = after;
        let name = self.body.filter_static("SEARCH", &filter);
        let mut finds = Vec::with_capacity(total);
        for _ in 0..count1 {
            finds.push(find1);
        }
        for _ in 0..count2 {
            finds.push(find2);
        }
        Some(vec![format!(
            "Effect::SearchLibrary {{ filter: &{name}, finds: &[{}], optional: true }}",
            finds.join(", ")
        )])
    }

    fn all_alts_remembered(raw: &str) -> bool {
        let mut any = false;
        for alt in raw.split(',').map(str::trim) {
            if alt.is_empty()
                || alt.contains("Targeted")
                || alt.contains("Opp")
                || !alt
                    .split(['.', '+'])
                    .any(|atom| atom.trim() == "IsRemembered")
            {
                return false;
            }
            any = true;
        }
        any
    }

    fn split_search_find(p: &mut Params) -> Option<&'static str> {
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        match (p.take("Destination").as_deref()?, tapped) {
            ("Battlefield", false) => Some("Find::BATTLEFIELD"),
            ("Battlefield", true) => Some("Find::BATTLEFIELD_TAPPED"),
            ("Hand", false) => Some("Find::HAND"),
            ("Graveyard", false) => Some("Find::GRAVEYARD"),
            ("Exile", false) => Some("Find::EXILE"),
            _ => None,
        }
    }

    /// `Origin$ Library,Graveyard` (or `Origin$ Library | OriginAlternative$ Graveyard`)
    /// -> `Effect::SearchLibraryOrGraveyard`.
    fn search_library_or_graveyard(
        &mut self,
        p: &mut Params,
        destination: &str,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if targets_here
            || p.has("RememberChanged")
            || p.has("DifferentNames")
            || p.has("WithTotalCMC")
            || !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
            || !matches!(p.take("Shuffle").as_deref(), None | Some("True"))
            || !matches!(
                p.take("ShuffleNonMandatory").as_deref(),
                None | Some("True")
            )
            || !matches!(p.take("Mandatory").as_deref(), None | Some("True"))
            || !matches!(p.take("ChangeNum").as_deref(), None | Some("1"))
        {
            return None;
        }
        p.take("Reveal");
        p.take("Hidden");
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if tapped && destination != "Battlefield" {
            return None;
        }
        let find = match (destination, tapped) {
            ("Hand", false) => "Find::HAND",
            ("Battlefield", false) => "Find::BATTLEFIELD",
            ("Battlefield", true) => "Find::BATTLEFIELD_TAPPED",
            ("TopOfLibrary", false) => "Find::TOP_OF_LIBRARY",
            ("Graveyard", false) => "Find::GRAVEYARD",
            ("Exile", false) => "Find::EXILE",
            ("Library", false) if p.take("LibraryPosition").as_deref() == Some("0") => {
                "Find::TOP_OF_LIBRARY"
            }
            _ => return None,
        };
        let valid = p.take("ChangeType")?;
        let cleaned = Self::strip_your_graveyard_owner(&valid).unwrap_or(valid);
        let filter = self.filter_expr_in_zone(&cleaned, true)?;
        let name = self.body.filter_static("SEARCH", &filter);
        let effect =
            format!("Effect::SearchLibraryOrGraveyard {{ filter: &{name}, find: &{find} }}");
        Some(vec![if optional {
            format!("Effect::MayDo {{ effects: &[{effect}] }}")
        } else {
            effect
        }])
    }

    /// Resolve whether a `SearchLibrary` branch of `count` cards may settle for
    /// fewer than `count` (CR 701.23b vs CR 701.23d).
    fn resolve_search_optional(
        &mut self,
        count: usize,
        filter: &str,
        optional: bool,
        mandatory: bool,
        up_to: bool,
    ) -> Option<bool> {
        if count <= 1 {
            return Some(optional);
        }
        let branch_opt = optional || (!mandatory && up_to);
        if branch_opt || (mandatory && filter == "Filter::Any") {
            return Some(branch_opt);
        }
        let msg = if mandatory {
            "`ChangeZone` requiring multiple cards with a stated quality"
        } else {
            "`ChangeZone` finding several cards without saying whether that is a maximum"
        };
        self.note(msg.to_string());
        None
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
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
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
            Some("True") => false,
            Some("False") => true,
            None => {
                if self.oracle.is_empty() {
                    true
                } else {
                    let lower = self.oracle.to_ascii_lowercase();
                    lower.contains("under its owner's control")
                        || lower.contains("under her owner's control")
                        || lower.contains("under his owner's control")
                        || lower.contains("under their owner's control")
                }
            }
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
            "TriggeredSpellAbility" => {
                matches!(
                    self.trigger_mode.as_deref(),
                    Some("SpellCast" | "SpellCastOrCopy")
                )
            }
            "TriggeredNewCardLKICopy"
            | "TriggeredNewCard"
            | "TriggeredCardLKICopy"
            | "TriggeredCard" => {
                matches!(
                    self.trigger_mode.as_deref(),
                    Some(
                        "ChangesZone"
                            | "Discarded"
                            | "Cycled"
                            | "Exerted"
                            | "Enlisted"
                            | "Sacrificed"
                            | "BecomesPlotted"
                            | "Plotted"
                            | "SpellCastOrCopy"
                            | "Taps"
                            | "Untaps"
                            | "TurnFaceUp"
                            | "Transformed"
                    )
                )
            }
            "TriggeredExploitedLKICopy" | "TriggeredExploited" => {
                self.trigger_mode.as_deref() == Some("Exploited")
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

    fn wave6_count_base(&self, base_raw: &str) -> Option<String> {
        Some(match base_raw {
            "Count$LastDieRoll" => "Amount::LastDieRoll".to_string(),
            "Count$CoinsWonThisResolution" => "Amount::CoinsWonThisResolution".to_string(),
            "Count$CastTotalManaSpent Snow" => "Amount::SnowManaSpentToCast".to_string(),
            "Count$DamageOpponentsTookThisTurn" => {
                "Amount::DamageDealtToOpponentsThisTurn".to_string()
            }
            "Count$ThisTurnCast_Card.CastSa Spell.Warp" | "Count$ThisTurnCast_Spell.Warp" => {
                "Amount::SpellsWarpedThisTurn(PlayerRel::EachPlayer)".to_string()
            }
            "Count$ThisTurnCast_Card.YouCtrl+CastSa Spell.Warp"
            | "Count$ThisTurnCast_Card.CastSa Spell.Warp+YouCtrl" => {
                "Amount::SpellsWarpedThisTurn(PlayerRel::You)".to_string()
            }
            "Count$ThisTurnCast_Card.OppCtrl+CastSa Spell.Warp"
            | "Count$ThisTurnCast_Card.CastSa Spell.Warp+OppCtrl" => {
                "Amount::SpellsWarpedThisTurn(PlayerRel::EachOpponent)".to_string()
            }
            "TriggeredSpellAbility$CardManaCostLKI" | "TriggeredSpellAbility$CardManaCost"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some("SpellCast" | "SpellCastOrCopy")
                ) =>
            {
                "Amount::EventManaValue".to_string()
            }
            "TriggeredExploited$CardPower" | "TriggeredExploitedLKICopy$CardPower"
                if self.trigger_mode.as_deref() == Some("Exploited") =>
            {
                "Amount::EventPower".to_string()
            }
            "TriggeredExploited$CardToughness"
            | "TriggeredExploitedLKICopy$CardToughness"
            | "TriggeredExploited$Toughness"
            | "TriggeredExploitedLKICopy$Toughness"
                if self.trigger_mode.as_deref() == Some("Exploited") =>
            {
                "Amount::EventLastToughness".to_string()
            }
            "TriggeredExploited$CardManaCost" | "TriggeredExploitedLKICopy$CardManaCost"
                if self.trigger_mode.as_deref() == Some("Exploited") =>
            {
                "Amount::EventManaValue".to_string()
            }
            "TriggerCount$Amount" | "Count$TriggeredObjectsNum" | "TriggeredNum"
                if matches!(
                    self.trigger_mode.as_deref(),
                    Some(
                        "ChangesZoneAll"
                            | "Attacked"
                            | "AttackersDeclared"
                            | "AttackersDeclaredOneTarget"
                            | "BecomeMonstrous"
                            | "BecomesMonstrous"
                            | "Scry"
                    )
                ) =>
            {
                "Amount::EventAmount".to_string()
            }
            "TriggerCount$Number" if self.trigger_mode.as_deref() == Some("Scry") => {
                "Amount::EventAmount".to_string()
            }
            "TriggeredCard$CardPower" | "TriggeredCardLKICopy$CardPower"
                if self.trigger_mode.as_deref() == Some("Sacrificed") =>
            {
                "Amount::EventPower".to_string()
            }
            "TriggeredCard$CardToughness"
            | "TriggeredCardLKICopy$CardToughness"
            | "TriggeredCard$Toughness"
            | "TriggeredCardLKICopy$Toughness"
                if self.trigger_mode.as_deref() == Some("Sacrificed") =>
            {
                "Amount::EventLastToughness".to_string()
            }
            _ if self.trigger_is_self => {
                let kind_code = base_raw
                    .strip_prefix("TriggeredCard$CardCounters.")
                    .or_else(|| base_raw.strip_prefix("TriggeredCardLKICopy$CardCounters."))?;
                let kind = super::counter_kind(kind_code)?;
                format!("Amount::CountersOnSource({kind})")
            }
            _ => return None,
        })
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
        let base = self.wave6_count_base(base_raw)?;
        let Some(op) = op_raw else {
            return Some(base);
        };
        if op == "Twice" {
            return Some(format!("Amount::Times {{ base: &{base}, factor: 2 }}"));
        }
        if op == "HalfDown" {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: false }}"));
        }
        if op == "HalfUp" {
            return Some(format!("Amount::Half {{ base: &{base}, round_up: true }}"));
        }
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

    fn flip_coin_win_branch(
        &mut self,
        name: &str,
        until_lose: bool,
        target: Option<&str>,
    ) -> Option<(Vec<String>, Vec<String>)> {
        let refs_wins = until_lose
            && self
                .svars
                .get(name.trim())
                .and_then(|body| Params::parse(body))
                .is_some_and(|(_, wp)| wp.entries.iter().any(|(_, v)| v.trim() == "Wins"));
        if refs_wins {
            let after = self.parse_branch_svar(
                name,
                target,
                Some(("Wins", "Count$CoinsWonThisResolution")),
            )?;
            Some((Vec::new(), after))
        } else {
            Some((self.parse_branch_svar(name, target, None)?, Vec::new()))
        }
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
        let (win, win_after_wins) = match p.take("WinSubAbility") {
            None => (Vec::new(), Vec::new()),
            Some(name) => self.flip_coin_win_branch(&name, until_lose, chain.target.as_deref())?,
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
            after.extend(win_after_wins);
            after.extend(wins_follow);
            return Some(vec![format!(
                "Effect::FlipUntilLose {{ max_flips: None, per_win: &[{}], after: &[{}] }}",
                win.join(", "),
                after.join(", ")
            )]);
        }
        if let Some(raw_amt) = p.take("Amount")
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
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
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
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(who) {
            return None;
        }
        Some(vec![format!("Effect::BecomeMonarch({who})")])
    }

    pub(super) fn is_single_player_rel(who: &str) -> bool {
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
                | "PlayerRel::EnchantedPlayer"
        )
    }

    fn is_untap_leaves_lose_control(raw: &str) -> bool {
        let mut parts: Vec<&str> = raw.split(',').map(str::trim).collect();
        parts.sort_unstable();
        parts == ["LeavesPlay", "LoseControl", "Untap"]
    }

    /// `DB$/AB$/SP$ AlterAttribute | Attributes$ Prepared|Saddled` (CR 723.3 / 702.171).
    pub(super) fn alter_attribute_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let attr = p.take("Attributes")?;
        let activate = p.take("Activate");
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let defined = p.take("Defined");
        let object_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        if attr == "Suspected" {
            let unsuspect = match activate.as_deref() {
                None | Some("True") => false,
                Some("False") => true,
                Some(_) => return None,
            };
            let target_spec = self.resolve_suspect_target(
                defined.as_deref(),
                target,
                targets_here,
                object_target,
            )?;
            let variant = if unsuspect { "Unsuspect" } else { "Suspect" };
            let effect = format!("Effect::{variant} {{ target: {target_spec} }}");
            return Some(vec![if optional {
                format!("Effect::MayDo {{ effects: &[{effect}] }}")
            } else {
                effect
            }]);
        }
        if activate.is_some_and(|v| v != "True") {
            return None;
        }
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

    fn resolve_suspect_target(
        &mut self,
        defined: Option<&str>,
        target: Option<&str>,
        targets_here: bool,
        object_target: bool,
    ) -> Option<String> {
        match (targets_here, defined) {
            (true, None | Some("Targeted")) if object_target => Some(target?.to_string()),
            (false, Some("Targeted" | "ParentTarget")) if object_target => {
                Some(target?.to_string())
            }
            (false, None | Some("Self")) if !self.on_a_spell && target.is_none() => {
                Some("TargetSpec::ThisObject".to_string())
            }
            (false, Some("Enchanted" | "Equipped")) if !self.on_a_spell && target.is_none() => {
                Some("TargetSpec::Object(&Filter::AttachedToBySource)".to_string())
            }
            (false, Some(d)) if target.is_none() && self.is_event_object_defined(d) => {
                Some("TargetSpec::EventObject".to_string())
            }
            (false, Some(def))
                if target.is_none()
                    && def.starts_with("Valid ")
                    && !def.contains("Remembered")
                    && !def.contains("Targeted") =>
            {
                let valid = def.strip_prefix("Valid ")?.trim();
                let expr = self.filter_expr(valid)?;
                let name = self.body.filter_static("SUSPECT", &expr);
                Some(format!("TargetSpec::Object(&{name})"))
            }
            _ => None,
        }
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
            || !matches!(
                p.take("ForgetOtherRemembered").as_deref(),
                None | Some("True")
            )
        {
            return None;
        }
        let amount = self.amount_or_count_ext(&p.take("NumCards")?)?;
        let next_name = sub.take()?;
        let next_body = self.svars.get(&next_name)?.clone();
        let (next_api, mut dp) = Params::parse(&next_body)?;
        dp.drop_prose();
        let origin = dp.take("Origin");
        if next_api != "ChangeZone"
            || !matches!(
                origin.as_deref(),
                Some("Graveyard,Exile" | "Exile,Graveyard" | "Graveyard")
            )
            || dp.take("Destination").as_deref() != Some("Hand")
            || !matches!(dp.take("Hidden").as_deref(), None | Some("True"))
            || !matches!(dp.take("ChangeNum").as_deref(), None | Some("1"))
            || !matches!(dp.take("ForgetChanged").as_deref(), None | Some("True"))
            || !matches!(
                dp.take("ForgetOtherRemembered").as_deref(),
                None | Some("True")
            )
        {
            return None;
        }
        if let Some(all_origins) = dp.take("UseAllOriginZones")
            && (all_origins != "True" || origin.as_deref() == Some("Graveyard"))
        {
            return None;
        }
        if dp.has("Mandatory") {
            return None;
        }
        dp.claim_label("ChangeTypeDesc", "ChangeType");
        let prompt_may = dp.take("SelectPrompt").is_some_and(|s| {
            let lower = s.to_ascii_lowercase();
            lower.starts_with("you may")
                || lower.starts_with("select up to ")
                || lower.starts_with("choose up to ")
        });
        let optional = match dp.take("Optional").as_deref() {
            Some("True") => true,
            None => {
                let lower = self.oracle.to_ascii_lowercase();
                prompt_may || lower.contains("you may put ") || lower.contains("you may return ")
            }
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
    /// "if you didn't" (or sacrificed fewer than `count`).
    fn take_remembered_sac_cond(
        &self,
        sp: &mut Params,
        count: u8,
    ) -> Option<(bool, Option<String>)> {
        if count == 1 && sp.peek("ConditionDefined") == Some("Remembered") {
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
            let ge_n = format!("GE{count}");
            let eq_n = format!("EQ{count}");
            let lt_n = format!("LT{count}");
            let le_prev = format!("LE{}", count.saturating_sub(1));
            return match cmp.as_deref() {
                None if count == 1 => Some((true, None)),
                Some(s) if s == ge_n || s == eq_n => Some((true, None)),
                Some(s) if s == lt_n || s == le_prev || (count == 1 && s == "EQ0") => {
                    Some((false, None))
                }
                _ => None,
            };
        }
        None
    }

    /// Reads a chain of follow-up `DB$` lines all sharing the same
    /// `Remembered` sacrifice polarity (`on_sac`: true = paid, false = unpaid),
    /// stopping at an optional `Cleanup | ClearRemembered$ True`.
    /// When `chain.target.is_none()`, allows one step to bind an outer target
    /// (`Victimize`, `Thalakos Deceiver`).
    #[allow(clippy::too_many_lines)]
    fn collect_remembered_sac_body(
        &mut self,
        first_sub: String,
        target: Option<&str>,
        sub_out: &mut Option<String>,
        count: u8,
        allow_sac_lki: bool,
        chain: &mut Chain,
    ) -> Option<(bool, Option<String>, Vec<String>)> {
        let saved_target = chain.target.clone();
        let saved_up_to = chain.target_up_to;
        let saved_exact = chain.target_exact;
        let saved_is_x = chain.target_is_x;
        let saved_second = chain.second_target.clone();
        let saved_second_up_to = chain.second_target_up_to;
        let saved_unique = chain.target_unique;
        let restore = |c: &mut Chain| {
            c.target.clone_from(&saved_target);
            c.target_up_to = saved_up_to;
            c.target_exact = saved_exact;
            c.target_is_x = saved_is_x;
            c.second_target.clone_from(&saved_second);
            c.second_target_up_to = saved_second_up_to;
            c.target_unique = saved_unique;
        };
        let mut cursor = Some(first_sub);
        let mut polarity: Option<bool> = None;
        let mut check_valid: Option<String> = None;
        let mut body_effects = Vec::new();
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 8 {
                restore(chain);
                return None;
            }
            let Some(body) = self.svars.get(name.trim()).cloned() else {
                restore(chain);
                return None;
            };
            let Some((api, mut sp)) = Params::parse(&body) else {
                restore(chain);
                return None;
            };
            let graveyard_desc = if sp.peek("Origin") == Some("Graveyard")
                || sp.peek("TgtZone") == Some("Graveyard")
            {
                sp.take("SpellDescription")
            } else {
                None
            };
            sp.drop_prose();
            if api == "Cleanup" && sp.take("ClearRemembered").as_deref() == Some("True") {
                let tail = sp.take("SubAbility");
                if !sp.exhausted() {
                    restore(chain);
                    return None;
                }
                *sub_out = tail;
                break;
            }
            let Some((on_sac, check)) = self.take_remembered_sac_cond(&mut sp, count) else {
                restore(chain);
                return None;
            };
            if let Some(prev) = polarity {
                if prev != on_sac || check_valid != check {
                    restore(chain);
                    return None;
                }
            } else {
                polarity = Some(on_sac);
                check_valid = check;
            }
            if sp.has("UnlessCost") || sp.has("AtEOT") {
                restore(chain);
                return None;
            }
            let mut step_targets_here = false;
            if sp.has("ValidTgts") {
                if chain.target.is_some() || self.line_is_gated_on_kicking(&sp) {
                    restore(chain);
                    return None;
                }
                sp.claim_label("ValidTgtsDesc", "ValidTgts");
                let Some(valid) = sp.take("ValidTgts") else {
                    restore(chain);
                    return None;
                };
                if self
                    .chain_target(&api, &mut sp, &valid, chain, graveyard_desc.as_deref())
                    .is_none()
                    || chain.second_target.is_some()
                {
                    restore(chain);
                    return None;
                }
                step_targets_here = true;
            }
            let step_target = if step_targets_here {
                Self::effect_target(chain)
            } else {
                target.map(str::to_string)
            };
            let step_targets_player = step_targets_here
                && step_target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
            let next = sp.take("SubAbility");
            let effs = if allow_sac_lki && on_sac && count == 1 {
                let saved_sp = sp.clone();
                if let Some(lki_effs) = self.try_sacrificed_lki_effect(
                    &api,
                    &mut sp,
                    step_target.as_deref(),
                    step_targets_here,
                    step_targets_player,
                ) {
                    Some(lki_effs)
                } else {
                    sp = saved_sp;
                    self.effect_of(
                        &api,
                        &mut sp,
                        step_target.as_deref(),
                        step_targets_here,
                        step_targets_player,
                    )
                }
            } else {
                self.effect_of(
                    &api,
                    &mut sp,
                    step_target.as_deref(),
                    step_targets_here,
                    step_targets_player,
                )
            };
            let Some(effs) = effs else {
                restore(chain);
                return None;
            };
            if !sp.exhausted() || effs.is_empty() {
                restore(chain);
                return None;
            }
            body_effects.extend(effs);
            cursor = next;
        }
        let Some(on_sac) = polarity else {
            restore(chain);
            return None;
        };
        if body_effects.is_empty() {
            restore(chain);
            return None;
        }
        Some((on_sac, check_valid, body_effects))
    }

    fn remembered_sac_lki_amount(&self, raw: &str) -> Option<(String, bool)> {
        let trimmed = raw.trim();
        let neg = trimmed.starts_with('-');
        let key = trimmed.trim_start_matches(['+', '-']);
        let def = match self.svars.get(key) {
            Some(d) => d.trim(),
            None if key.contains('$') => key,
            None => return None,
        };
        let (head, suffix) = def.split_once('/').map_or((def, ""), |(h, s)| (h, s));
        let base = match head {
            "RememberedLKI$CardPower" | "Remembered$CardPower" => {
                "Amount::SacrificedPower".to_string()
            }
            "RememberedLKI$CardToughness" | "Remembered$CardToughness" => {
                "Amount::SacrificedToughness".to_string()
            }
            "RememberedLKI$CardManaCost" | "Remembered$CardManaCost" => {
                "Amount::SacrificedManaValue".to_string()
            }
            _ => return None,
        };
        let mag = Self::wrap_amount_suffix(base, suffix)?;
        Some((mag, neg))
    }

    /// Lowers a single follow-up effect whose amount reads the sacrificed
    /// permanent's LKI power, toughness, or mana value (`Amount::Sacrificed*`).
    #[allow(clippy::too_many_lines)]
    fn try_sacrificed_lki_effect(
        &mut self,
        api: &str,
        sp: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if sp.has("UnlessCost")
            || sp.has("AtEOT")
            || sp.has("ReplaceDyingDefined")
            || sp.entries.iter().any(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        let eff = match api {
            "GainLife" => {
                let raw = sp.take("LifeAmount")?;
                let (amt, false) = self.remembered_sac_lki_amount(&raw)? else {
                    return None;
                };
                let who = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                if !sp.exhausted() {
                    return None;
                }
                if who == "PlayerRel::You" {
                    format!("Effect::GainLife {{ amount: {amt} }}")
                } else {
                    format!("Effect::GainLifeFor {{ amount: {amt}, who: {who} }}")
                }
            }
            "LoseLife" => {
                sp.take("IsCurse");
                let raw = sp.take("LifeAmount")?;
                let (amt, false) = self.remembered_sac_lki_amount(&raw)? else {
                    return None;
                };
                let who = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                if !sp.exhausted() {
                    return None;
                }
                format!("Effect::LoseLife {{ amount: {amt}, target: {who} }}")
            }
            "Draw" => {
                let raw = sp.take("NumCards")?;
                let (amt, false) = self.remembered_sac_lki_amount(&raw)? else {
                    return None;
                };
                let who = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    target,
                    targets_a_player,
                )?;
                if !sp.exhausted() {
                    return None;
                }
                if who == "PlayerRel::You" {
                    format!("Effect::DrawCards {{ amount: {amt} }}")
                } else {
                    format!("Effect::DrawCardsFor {{ amount: {amt}, who: {who} }}")
                }
            }
            "DealDamage" => {
                sp.take("IsCurse");
                let raw = sp.take("NumDmg")?;
                let (amt, false) = self.remembered_sac_lki_amount(&raw)? else {
                    return None;
                };
                let to = match sp.take("Defined").as_deref() {
                    None if targets_here => target?.to_string(),
                    Some("Targeted") if target.is_some() => target?.to_string(),
                    Some("You") => "TargetSpec::Player(PlayerRel::You)".to_string(),
                    Some(def) => {
                        let who =
                            self.wave11_player_of_line(Some(def), target, targets_a_player)?;
                        format!("TargetSpec::Player({who})")
                    }
                    None => return None,
                };
                if !sp.exhausted() {
                    return None;
                }
                format!("Effect::DealDamage {{ amount: {amt}, target: {to} }}")
            }
            "PutCounter" => {
                sp.take("IsCurse");
                let raw = sp.take("CounterNum")?;
                let (amt, false) = self.remembered_sac_lki_amount(&raw)? else {
                    return None;
                };
                let code = sp.take("CounterType")?;
                let counter = super::counter_kind(&code)?;
                let on_self = match sp.take("Defined").as_deref() {
                    None | Some("Self") if !self.on_a_spell && !targets_here => true,
                    None | Some("Targeted")
                        if target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
                    {
                        false
                    }
                    _ => return None,
                };
                if !sp.exhausted() {
                    return None;
                }
                if on_self {
                    format!(
                        "Effect::AddCounterFilter {{ filter: &Filter::This, kind: {counter}, amount: {amt} }}"
                    )
                } else {
                    format!("Effect::AddCounter {{ kind: {counter}, amount: {amt} }}")
                }
            }
            "Token" => {
                let raw = sp.take("TokenAmount")?;
                let (amt, false) = self.remembered_sac_lki_amount(&raw)? else {
                    return None;
                };
                let owner = match sp.take("TokenOwner").as_deref() {
                    None | Some("You") => "PlayerRel::You",
                    other => self.wave11_player_of_line(other, target, targets_a_player)?,
                };
                let tapped = match sp.take("TokenTapped").as_deref() {
                    None => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let stem = sp.take("TokenScript")?;
                if !sp.exhausted() || (owner != "PlayerRel::You" && tapped) {
                    return None;
                }
                let body = self.tokens?.body(&stem, self.cats)?;
                let token = format!("&generated_tokens::{}", body.constant);
                if owner != "PlayerRel::You" {
                    format!(
                        "Effect::CreateTokensFor {{ player: {owner}, token: {token}, amount: {amt} }}"
                    )
                } else if tapped {
                    format!(
                        "Effect::CreateTokenModified {{ token: {token}, amount: {amt}, tapped: true, haste: false }}"
                    )
                } else {
                    format!("Effect::CreateTokenN {{ token: {token}, amount: {amt} }}")
                }
            }
            _ => return None,
        };
        Some(vec![eff])
    }

    fn try_unconditional_sac_lki_followup(
        &mut self,
        first_sub: &str,
        target: Option<&str>,
        sub_out: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let body = self.svars.get(first_sub.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        sp.drop_prose();
        let saved_target = chain.target.clone();
        let saved_up_to = chain.target_up_to;
        let saved_exact = chain.target_exact;
        let saved_is_x = chain.target_is_x;
        let saved_second = chain.second_target.clone();
        let saved_second_up_to = chain.second_target_up_to;
        let saved_unique = chain.target_unique;
        let restore = |c: &mut Chain| {
            c.target.clone_from(&saved_target);
            c.target_up_to = saved_up_to;
            c.target_exact = saved_exact;
            c.target_is_x = saved_is_x;
            c.second_target.clone_from(&saved_second);
            c.second_target_up_to = saved_second_up_to;
            c.target_unique = saved_unique;
        };
        let mut step_targets_here = false;
        if sp.has("ValidTgts") {
            if chain.target.is_some() || self.line_is_gated_on_kicking(&sp) {
                return None;
            }
            sp.claim_label("ValidTgtsDesc", "ValidTgts");
            let valid = sp.take("ValidTgts")?;
            if self
                .chain_target(&api, &mut sp, &valid, chain, None)
                .is_none()
                || chain.second_target.is_some()
            {
                restore(chain);
                return None;
            }
            step_targets_here = true;
        }
        let step_target = if step_targets_here {
            Self::effect_target(chain)
        } else {
            target.map(str::to_string)
        };
        let step_targets_player = step_targets_here
            && step_target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
        let next = sp.take("SubAbility");
        let Some(effs) = self.try_sacrificed_lki_effect(
            &api,
            &mut sp,
            step_target.as_deref(),
            step_targets_here,
            step_targets_player,
        ) else {
            restore(chain);
            return None;
        };
        if !sp.exhausted() || effs.is_empty() {
            restore(chain);
            return None;
        }
        if let Some(cname) = next {
            if self.sub_is_api(Some(&cname), "Cleanup") {
                if self
                    .consume_optional_remember_cleanup(cname, sub_out)
                    .is_none()
                {
                    restore(chain);
                    return None;
                }
            } else {
                *sub_out = Some(cname);
            }
        }
        Some(effs)
    }

    /// `Sacrifice | RememberSacrificed$ True` followed by `ConditionDefined$
    /// Remembered` (`PlayerMayPayCostThen`, `PlayerMayPayCostOr`,
    fn emit_remembered_sac_optional(
        who: &str,
        sac_filter: &str,
        valid: &str,
        count: u8,
        on_sac: bool,
        check_valid: Option<&str>,
        body_one: &str,
    ) -> Option<Vec<String>> {
        if check_valid.is_some_and(|c| c != valid) {
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
        if count > 1 {
            let variant = if on_sac {
                "PlayerMaySacrificeNThen"
            } else {
                "PlayerMaySacrificeNOr"
            };
            return Some(vec![format!(
                "Effect::{variant} {{ player: {who}, filter: {sac_filter}, count: {count}, \
                 effect: &{body_one} }}"
            )]);
        }
        let variant = if on_sac {
            "PlayerMayPayCostThen"
        } else {
            "PlayerMayPayCostOr"
        };
        Some(vec![format!(
            "Effect::{variant} {{ player: {who}, cost: &CostPart::Sacrifice({sac_filter}), \
             effect: &{body_one} }}"
        )])
    }

    /// `PlayerMaySacrificeNThen`, `PlayerMaySacrificeNOr`, `SacrificeAndIf`,
    /// `SacrificeNOrElse`, `SacrificeOneThen`, or `SacrificeSelfThen`).
    #[allow(clippy::too_many_lines)]
    pub(super) fn sacrifice_remembered_effect(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        targets_a_player: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if p.take("RememberSacrificed").as_deref() != Some("True")
            || !matches!(
                p.take("ForgetOtherRemembered").as_deref(),
                None | Some("True")
            )
            || !matches!(p.take("ChangeNum").as_deref(), None | Some("1"))
        {
            return None;
        }
        let (count, optional, sac_valid, who) = if api == "SacrificeAll" {
            if let Some(ctrl) = p.take("Controller")
                && ctrl != "You"
            {
                return None;
            }
            if p.take("Defined").as_deref() != Some("Self")
                || p.has("ValidCards")
                || self.on_a_spell
            {
                return None;
            }
            (1u8, false, Some("Self".to_string()), "PlayerRel::You")
        } else {
            let count: u8 = match p.take("Amount").as_deref().map(str::trim) {
                None | Some("1") => 1,
                Some(raw) => raw.parse::<u8>().ok().filter(|&n| n >= 2)?,
            };
            if let Some(strict) = p.take("StrictAmount")
                && (strict != "True" || count < 2)
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
            let who =
                self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
            (count, optional, sac_valid, who)
        };
        if !p.exhausted() {
            return None;
        }
        let first_sub = sub.take()?;
        let allow_sac_lki = api == "Sacrifice"
            && count == 1
            && !optional
            && who == "PlayerRel::You"
            && sac_valid.as_deref().is_some_and(|v| v != "Self");
        let collected = self.collect_remembered_sac_body(
            first_sub.clone(),
            target,
            sub,
            count,
            allow_sac_lki,
            chain,
        );
        let Some((on_sac, check_valid, body_effects)) = collected else {
            if allow_sac_lki
                && let Some(valid) = sac_valid
                && let Some(lki_effs) =
                    self.try_unconditional_sac_lki_followup(&first_sub, target, sub, chain)
            {
                let sac_filter = if valid == "Permanent" {
                    "&Filter::Any".to_string()
                } else {
                    let expr = self.filter_expr(&valid)?;
                    format!("&{}", self.body.filter_static("SAC", &expr))
                };
                return Some(vec![format!(
                    "Effect::SacrificeOneThen {{ filter: {sac_filter}, if_it_was: &Filter::Any, \
                     then: &[{}] }}",
                    lki_effs.join(", ")
                )]);
            }
            return None;
        };
        let body_one = match body_effects.as_slice() {
            [one] => one.clone(),
            many => format!("Effect::Sequence(&[{}])", many.join(", ")),
        };
        if sac_valid.as_deref().is_none_or(|v| v == "Self") {
            return (count == 1
                && !optional
                && on_sac
                && check_valid.is_none()
                && who == "PlayerRel::You"
                && !self.on_a_spell)
                .then(|| {
                    vec![format!(
                        "Effect::SacrificeSelfThen {{ then: &[{}] }}",
                        body_effects.join(", ")
                    )]
                });
        }
        let valid = sac_valid?;
        let sac_filter = if valid == "Permanent" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            format!("&{}", self.body.filter_static("SAC", &expr))
        };
        if optional {
            return Self::emit_remembered_sac_optional(
                who,
                &sac_filter,
                &valid,
                count,
                on_sac,
                check_valid.as_deref(),
                &body_one,
            );
        }
        if who != "PlayerRel::You" {
            return None;
        }
        if count == 1 && on_sac {
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
        if !on_sac && check_valid.is_none() {
            if count > 1 {
                return Some(vec![format!(
                    "Effect::SacrificeNOrElse {{ filter: {sac_filter}, count: {count}, \
                     otherwise: &[{}] }}",
                    body_effects.join(", ")
                )]);
            }
            return Some(vec![format!(
                "Effect::IfCondition {{ condition: Condition::CanSacrifice({sac_filter}), \
                 then: &[Effect::SacrificeFilter {{ who: PlayerRel::You, filter: {sac_filter} }}], \
                 otherwise: &[{}] }}",
                body_effects.join(", ")
            )]);
        }
        None
    }

    /// Wave-7 fallback for `DigUntil`: handles `NoPutDesc$` prose and
    /// `RevealedDestination$ Library | RevealedLibraryPosition$ -1 | Shuffle$ True`
    /// (putting the rest into the library and immediately shuffling).
    #[allow(clippy::too_many_lines)]
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
        if player == "PlayerRel::You"
            && found_raw.as_deref() == Some("Battlefield")
            && p.peek("GainControl") == Some("True")
        {
            p.take("GainControl");
        }
        if !p.entries.iter().all(|(k, _)| k.starts_with("Condition")) {
            return None;
        }
        if matches!(valid.trim(), "Card.NamedCard" | "NamedCard") {
            if !Self::is_single_player_rel(player) || random_order {
                return None;
            }
            if shuffle
                && revealed_raw.as_deref() == Some("Library")
                && matches!(pos.as_deref(), None | Some("-1" | "0"))
            {
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
                        "Effect::RevealUntilNamed {{ player: {player}, matched: {matched}, \
                         otherwise: SearchDest::TopOfLibrary }}"
                    ),
                    format!("Effect::ShuffleLibrary {{ who: {player} }}"),
                ]);
            }
            if !shuffle && pos.is_none() {
                let otherwise = match revealed_raw.as_deref() {
                    Some("Library") => "SearchDest::TopOfLibrary",
                    Some("Graveyard") => "SearchDest::Graveyard",
                    Some("Exile") => "SearchDest::Exile",
                    Some("Hand") => "SearchDest::Hand",
                    _ => return None,
                };
                let matched = match found_raw.as_deref() {
                    Some("Battlefield") => "SearchDest::Battlefield",
                    Some("Hand") => "SearchDest::Hand",
                    Some("Graveyard") => "SearchDest::Graveyard",
                    Some("Exile") => "SearchDest::Exile",
                    Some("Library") => "SearchDest::TopOfLibrary",
                    None if matches!(
                        otherwise,
                        "SearchDest::Graveyard" | "SearchDest::Exile" | "SearchDest::Hand"
                    ) =>
                    {
                        otherwise
                    }
                    _ => return None,
                };
                return Some(vec![format!(
                    "Effect::RevealUntilNamed {{ player: {player}, matched: {matched}, \
                     otherwise: {otherwise} }}"
                )]);
            }
            return None;
        }
        let expr = self.filter_expr_in_zone(&valid, true)?;
        let name = self.body.filter_static("REVEAL_UNTIL", &expr);
        // Putting the rest into the library at any position and immediately
        // shuffling is identical to leaving them in library + shuffling.
        if shuffle
            && revealed_raw.as_deref() == Some("Library")
            && matches!(pos.as_deref(), None | Some("-1" | "0"))
        {
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
                Some("Hand") => "SearchDest::Hand",
                _ => return None,
            };
            let matched = match found_raw.as_deref() {
                Some("Battlefield") => "SearchDest::Battlefield",
                Some("Hand") => "SearchDest::Hand",
                Some("Graveyard") => "SearchDest::Graveyard",
                Some("Exile") => "SearchDest::Exile",
                Some("Library") => "SearchDest::TopOfLibrary",
                None if matches!(
                    otherwise,
                    "SearchDest::Graveyard" | "SearchDest::Exile" | "SearchDest::Hand"
                ) =>
                {
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
                        || t.starts_with("TargetSpec::ObjectOfEachOpponent(")
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
            self.chain_target("Protection", sp, &valid, chain, None)?;
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
            if api == "Play"
                && scope == Wave8Scope::ChosenObject
                && sp.peek("Valid") == Some("Card.ChosenCard")
                && !sp.has("Defined")
            {
                sp.take("Valid");
                if sp.peek("ValidZone") == Some("Exile") {
                    sp.take("ValidZone");
                }
                sp.entries
                    .push(("Defined".to_string(), "DelayTriggerRemembered".to_string()));
                uses_event_obj = true;
            }
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
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let repeat_domain = match chooser {
            "PlayerRel::EachPlayer" => Some("RepeatDomain::EachPlayer"),
            "PlayerRel::EachOpponent" | "PlayerRel::Opponent" => Some("RepeatDomain::EachOpponent"),
            _ if Self::is_single_seat_rel(chooser) => None,
            _ => return self.deny(format!("`ChooseCard` for multi-seat `{chooser}`")),
        };
        let eff_chooser = if repeat_domain.is_some() {
            "PlayerRel::EventPlayer"
        } else {
            chooser
        };
        let controlled_by_you = match p.peek("ControlledByPlayer") {
            Some("You") => {
                p.take("ControlledByPlayer");
                true
            }
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
        let choices_effective = if controlled_by_you {
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
            Some("Graveyard") if !controlled_by_you => {
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
            None | Some("False") => false,
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
            sp.take("IsCurse");
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
                Some("Exile") if !tapped && !gain => {
                    if self.has_linked_exile_reader() {
                        return self.deny(
                            "`ChooseMove::Exile` on a card referencing `ExiledWith`".to_string(),
                        );
                    }
                    "ChooseMove::Exile".to_string()
                }
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
            let trailing = self.strip_chosen_card_cleanup_tail(after);
            return Some((movement, Vec::new(), trailing));
        }
        let then_effects =
            self.compile_wave8_scoped_chain(sub_name, Wave8Scope::ChosenObject, None, true)?;
        Some(("ChooseMove::None".to_string(), then_effects, None))
    }

    fn strip_chosen_card_cleanup_tail(&self, after: Option<String>) -> Option<String> {
        let name = after?;
        if self.is_wave8_cleanup_svar(&name, true) {
            return None;
        }
        let Some(body) = self.svars.get(name.trim()) else {
            return Some(name);
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return Some(name);
        };
        p.drop_prose();
        if api != "Cleanup" {
            return Some(name);
        }
        let mut any = false;
        if let Some(v) = p.take("ClearChosenCard") {
            if v != "True" {
                return Some(name);
            }
            any = true;
        }
        if let Some(v) = p.take("ClearRemembered") {
            if v != "True" {
                return Some(name);
            }
            any = true;
        }
        let next = p.take("SubAbility");
        if any && p.exhausted() {
            next
        } else {
            Some(name)
        }
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
                        "Effect::AddCounterFilter {{ filter: &Filter::This, kind: {kind}, amount: {amt} }}"
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
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
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
            if self
                .chain_target(&api, &mut sp, &valid, chain, None)
                .is_none()
            {
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
                    format!("Effect::GainLifeFor {{ amount: {amt}, who: {who} }}")
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
                sp.take("IsCurse");
                let duration = match sp.take("Duration").as_deref() {
                    None => "Duration::UntilEndOfTurn",
                    Some("Permanent") => "Duration::Indefinitely",
                    Some(_) => {
                        chain.target = saved_target;
                        return None;
                    }
                };
                let defined = sp.take("Defined");
                let is_targeted = match (defined.as_deref(), target.as_deref()) {
                    (None | Some("Targeted"), Some(t)) if t.starts_with("TargetSpec::Object(") => {
                        Some(true)
                    }
                    (None | Some("Self"), None) if !self.on_a_spell => Some(false),
                    _ => None,
                };
                let Some(is_targeted) = is_targeted else {
                    chain.target = saved_target;
                    return None;
                };
                if !sp.exhausted() {
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
                let (Some(att), Some(def)) = (side(att_raw.as_deref()), side(def_raw.as_deref()))
                else {
                    chain.target = saved_target;
                    return None;
                };
                if att == "Amount::Fixed(0)" && def == "Amount::Fixed(0)" {
                    chain.target = saved_target;
                    return None;
                }
                if is_targeted {
                    format!(
                        "Effect::PumpTarget {{ power: {att}, toughness: {def}, \
                         keywords: KeywordSet::EMPTY, duration: {duration} }}"
                    )
                } else {
                    format!(
                        "Effect::PumpFilter {{ filter: &Filter::This, controlled_by: None, \
                         power: {att}, toughness: {def}, keywords: KeywordSet::EMPTY, \
                         duration: {duration} }}"
                    )
                }
            }
            _ => {
                chain.target = saved_target;
                return None;
            }
        };
        Some((vec![eff], trailing))
    }

    /// `AB$/SP$/DB$ MoveCounter` moving one counter (CR 122.5) ->
    /// `Effect::MoveCounter { from, to, kind }` or
    /// `Effect::MoveCounterToEvent { kind }`.
    pub(super) fn move_counter_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        second_target_here: bool,
    ) -> Option<Vec<String>> {
        if !matches!(p.take("CounterNum").as_deref(), None | Some("1")) {
            return None;
        }
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let wrap = |eff: String| {
            if optional {
                format!("Effect::MayDo {{ effects: &[{eff}] }}")
            } else {
                eff
            }
        };
        p.take("IsCurse");
        let raw_kind = p.take("CounterType")?;
        let kind_opt = if raw_kind.eq_ignore_ascii_case("Any") {
            None
        } else {
            Some(super::counter_kind(&raw_kind)?)
        };
        let source = p.take("Source");
        let defined = p.take("Defined");
        if second_target_here {
            if source.as_deref() != Some("ParentTarget")
                || !matches!(defined.as_deref(), None | Some("Targeted"))
                || !target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            {
                return None;
            }
            let kind_expr = kind_opt.map_or_else(|| "None".to_string(), |k| format!("Some({k})"));
            return Some(vec![wrap(format!(
                "Effect::MoveCounter {{ from: TargetSlot::First, to: TargetSlot::Second, kind: {kind_expr} }}"
            ))]);
        }
        if source.as_deref() == Some("Self")
            && !self.on_a_spell
            && !targets_here
            && target.is_none()
            && defined
                .as_deref()
                .is_some_and(|d| self.is_event_object_defined(d))
        {
            let kind = kind_opt?;
            return Some(vec![wrap(format!(
                "Effect::MoveCounterToEvent {{ kind: {kind} }}"
            ))]);
        }
        let is_obj_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        let (from, to) = match (source.as_deref(), defined.as_deref()) {
            (Some("Self"), None | Some("Targeted"))
                if !self.on_a_spell && targets_here && is_obj_target =>
            {
                ("TargetSlot::This", "TargetSlot::First")
            }
            (Some("Self"), Some("Targeted" | "ParentTarget"))
                if !self.on_a_spell && !targets_here && is_obj_target =>
            {
                ("TargetSlot::This", "TargetSlot::First")
            }
            (None | Some("Targeted"), Some("Self"))
                if !self.on_a_spell && targets_here && is_obj_target =>
            {
                ("TargetSlot::First", "TargetSlot::This")
            }
            (Some("Targeted" | "ParentTarget"), Some("Self"))
                if !self.on_a_spell && !targets_here && is_obj_target =>
            {
                ("TargetSlot::First", "TargetSlot::This")
            }
            _ => return None,
        };
        let kind_expr = kind_opt.map_or_else(|| "None".to_string(), |k| format!("Some({k})"));
        Some(vec![wrap(format!(
            "Effect::MoveCounter {{ from: {from}, to: {to}, kind: {kind_expr} }}"
        ))])
    }

    pub(super) fn incubate_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let count = self.amount_or_count_ext(&p.take("Amount")?)?;
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let times: usize = match p.take("Times").as_deref().map(str::trim) {
            None => 1,
            Some(raw) => raw
                .parse::<usize>()
                .ok()
                .filter(|&n| (1..=10).contains(&n))?,
        };
        let one = if who == "PlayerRel::You"
            && let Some(fixed) = count
                .strip_prefix("Amount::Fixed(")
                .and_then(|s| s.strip_suffix(')'))
        {
            format!("Effect::incubate({fixed})")
        } else {
            format!("Effect::Incubate {{ who: {who}, count: {count} }}")
        };
        Some(vec![one; times])
    }

    pub(super) fn multiply_counter_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        if !matches!(
            p.take("Multiplier").as_deref().map(str::trim),
            None | Some("2")
        ) {
            return None;
        }
        let code = p.take("CounterType")?;
        let Some(kind) = super::counter_kind(&code) else {
            return self.deny(format!("counter `{code}`"));
        };
        let is_single_obj =
            single_target && target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        let filter = match p.take("Defined").as_deref() {
            None if targets_here && is_single_obj => "&Filter::Reference".to_string(),
            Some("Targeted" | "ParentTarget") if !targets_here && is_single_obj => {
                "&Filter::Reference".to_string()
            }
            Some("Self") if !self.on_a_spell => "&Filter::This".to_string(),
            Some("TriggeredAttackerLKICopy" | "TriggeredAttacker")
                if (matches!(
                    self.trigger_mode.as_deref(),
                    Some("Attacks" | "AttackerUnblocked" | "AttackerBlocked")
                ) || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                    && self
                        .block_line
                        .as_ref()
                        .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocks)))
                    && target.is_none() =>
            {
                "&Filter::Reference".to_string()
            }
            Some("TriggeredBlockerLKICopy" | "TriggeredBlocker")
                if (self.trigger_mode.as_deref() == Some("Blocks")
                    || (self.trigger_mode.as_deref() == Some("AttackerBlockedByCreature")
                        && self
                            .block_line
                            .as_ref()
                            .is_some_and(|(r, _, _)| *r == super::BlockRole::Blocked)))
                    && target.is_none() =>
            {
                "&Filter::Reference".to_string()
            }
            Some(
                "TriggeredCardLKICopy"
                | "TriggeredCard"
                | "TriggeredNewCardLKICopy"
                | "TriggeredNewCard",
            ) if self.trigger_mode.as_deref() == Some("ChangesZone") && target.is_none() => {
                "&Filter::Reference".to_string()
            }
            Some(def)
                if (target.is_none() || prior_target_used)
                    && def.starts_with("Valid ")
                    && !def.contains("Remembered")
                    && !def.contains("Targeted") =>
            {
                let valid = def.strip_prefix("Valid ")?.trim();
                let expr = self.filter_expr(valid)?;
                format!("&{}", self.body.filter_static("DOUBLE", &expr))
            }
            _ => return None,
        };
        Some(vec![format!(
            "Effect::DoubleCountersFilter {{ filter: {filter}, kind: {kind} }}"
        )])
    }

    pub(super) fn change_targets_target(
        &mut self,
        valid: &str,
        target_type: Option<&str>,
    ) -> Option<String> {
        let raw = target_type?.trim();
        let any_source = matches!(valid, "Card" | "Card,Emblem" | "Emblem,Card");
        match raw {
            "Spell.singleTarget" | "Spell.numTargets EQ1" => {
                if valid == "Card" {
                    Some("TargetSpec::Spell(&Filter::WithSingleTarget)".to_string())
                } else {
                    let inner = self.filter_expr_in_zone(valid, true)?;
                    let expr = format!("Filter::And(&[{inner}, Filter::WithSingleTarget])");
                    let name = self.body.filter_static("TARGET", &expr);
                    Some(format!("TargetSpec::Spell(&{name})"))
                }
            }
            "SpellAbility.singleTarget"
            | "Spell.singleTarget,Activated.singleTarget,Triggered.singleTarget"
                if any_source =>
            {
                Some("TargetSpec::SpellOrAbility(&Filter::WithSingleTarget)".to_string())
            }
            "Activated.singleTarget" if any_source => {
                Some("TargetSpec::ActivatedAbility(&Filter::WithSingleTarget)".to_string())
            }
            "Instant.singleTarget,Sorcery.singleTarget"
            | "Sorcery.singleTarget,Instant.singleTarget"
                if valid == "Card" =>
            {
                let expr = "Filter::And(&[Filter::Or(&[Filter::INSTANT, Filter::SORCERY]), Filter::WithSingleTarget])";
                let name = self.body.filter_static("TARGET", expr);
                Some(format!("TargetSpec::Spell(&{name})"))
            }
            "SpellAbility" if any_source => {
                Some("TargetSpec::SpellOrAbility(&Filter::Any)".to_string())
            }
            "Spell" | "Spell,Activated,Triggered" | "Activated,Triggered" => {
                self.counter_or_stack_target(valid, Some(raw), false)
            }
            _ => None,
        }
    }

    pub(super) fn change_targets_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        if (targets_here && !single_target)
            || p.has("RandomTarget")
            || p.has("Chooser")
            || p.has("TargetValidTargeting")
            || p.has("TargetRestriction")
            || p.has("ConditionTargetValidTargeting")
            || p.has("ConditionTargetsSingleTarget")
        {
            return None;
        }
        match p.take("Defined").as_deref() {
            None if targets_here => {}
            Some("Targeted" | "ParentTarget") if !targets_here => {}
            _ => return None,
        }
        let target_spec = target?;
        let is_stack_target = target_spec.starts_with("TargetSpec::Spell(")
            || target_spec.starts_with("TargetSpec::SpellOrAbility(")
            || target_spec.starts_with("TargetSpec::AbilityOnStack(")
            || target_spec.starts_with("TargetSpec::ActivatedAbility(");
        if !is_stack_target {
            return None;
        }
        let raw_type = p.take("TargetType");
        let single_target_stack = target_spec.contains("Filter::WithSingleTarget")
            || raw_type
                .as_deref()
                .is_some_and(|t| t.contains("singleTarget") || t.contains("numTargets EQ1"));
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let magnet = p.take("DefinedMagnet");
        let change_single = match p.take("ChangeSingleTarget").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        if optional && magnet.is_none() && !change_single {
            return Some(vec!["Effect::ChooseNewTargets".to_string()]);
        }
        if !optional && single_target_stack {
            return match magnet.as_deref() {
                None if !change_single => Some(vec![
                    "Effect::ChangeTarget { to: &Filter::Any }".to_string(),
                ]),
                Some("Self") if !self.on_a_spell => Some(vec![
                    "Effect::ChangeTarget { to: &Filter::This }".to_string(),
                ]),
                _ => None,
            };
        }
        None
    }

    pub(super) fn monstrosity_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if self.on_a_spell
            || target.is_some()
            || !matches!(p.take("Defined").as_deref(), None | Some("Self"))
        {
            return None;
        }
        let raw = p.take("Num").or_else(|| p.take("Amount"))?;
        let n = self.amount_or_count_ext(&raw)?;
        Some(vec![format!("Effect::Monstrosity({n})")])
    }

    pub(super) fn suspect_api_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let unsuspect = match p.take("Activate").as_deref() {
            None | Some("True") => false,
            Some("False") => true,
            Some(_) => return None,
        };
        let defined = p.take("Defined");
        let object_target = target.is_some_and(|t| t.starts_with("TargetSpec::Object("));
        let target_spec =
            self.resolve_suspect_target(defined.as_deref(), target, targets_here, object_target)?;
        let variant = if unsuspect { "Unsuspect" } else { "Suspect" };
        let effect = format!("Effect::{variant} {{ target: {target_spec} }}");
        Some(vec![if optional {
            format!("Effect::MayDo {{ effects: &[{effect}] }}")
        } else {
            effect
        }])
    }

    pub(super) fn radiation_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let raw = p.take("Num").or_else(|| p.take("Amount"))?;
        let amount = self.amount_or_count_ext(&raw)?;
        let who =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        Some(vec![format!(
            "Effect::Rad {{ who: {who}, amount: {amount} }}"
        )])
    }

    pub(super) fn exchange_control_single_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        single_target: bool,
    ) -> Option<Vec<String>> {
        if !targets_here
            || !single_target
            || !target.is_some_and(|t| t.starts_with("TargetSpec::Object("))
            || p.has("TargetingPlayer")
            || p.has("TargetsWithDefinedController")
            || p.has("TargetsWithSharedCardType")
            || p.has("TargetsWithSameCardType")
            || p.has("TargetsWithDifferentControllers")
            || p.has("RememberExchanged")
        {
            return None;
        }
        let defined = p.take("Defined")?;
        if self.is_event_object_defined(&defined) {
            return Some(vec!["Effect::ExchangeControlOfEventAndTarget".to_string()]);
        }
        None
    }

    pub(super) fn skip_phase_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        p.take("IsCurse");
        if p.has("Step") || p.has("Duration") || p.has("NumPhases") {
            return None;
        }
        if p.take("Phase").as_deref() != Some("BeginCombat") {
            return None;
        }
        let player =
            self.wave11_player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        Some(vec![format!(
            "Effect::SkipNextCombatPhase {{ player: {player} }}"
        )])
    }

    // -------------------------------------------------------------------------
    // Wave 18b: Clash, Detain, AddPhase, ChooseNumber, ChoosePlayer,
    // Goblin Charbelcher (`DigUntil` + `RememberRevealed`), and clean
    // `Remember*` -> `ThisWay` chains.
    // -------------------------------------------------------------------------

    /// `AB$/SP$/DB$ Clash` -> `Effect::Clash { on_win, on_loss }` (CR 701.30).
    pub(super) fn clash_effect(
        &mut self,
        p: &mut Params,
        targets_here: bool,
        chain: &Chain,
    ) -> Option<Vec<String>> {
        if targets_here || p.has("Defined") || p.has("RememberClasher") {
            return None;
        }
        let prior_target_used = !chain.effects.is_empty();
        let on_win = match p.take("WinSubAbility") {
            None => Vec::new(),
            Some(name) => self.parse_branch_svar_with_prior(
                &name,
                chain.target.as_deref(),
                prior_target_used,
            )?,
        };
        let on_loss = match p.take("OtherwiseSubAbility") {
            None => Vec::new(),
            Some(name) => self.parse_branch_svar_with_prior(
                &name,
                chain.target.as_deref(),
                prior_target_used,
            )?,
        };
        Some(vec![format!(
            "Effect::Clash {{ on_win: &[{}], on_loss: &[{}] }}",
            on_win.join(", "),
            on_loss.join(", ")
        )])
    }

    /// Parses a branch `SVar` (`WinSubAbility$` / `OtherwiseSubAbility$`) while
    /// preserving `prior_target_used` when an earlier effect in the parent chain
    /// already consumed `parent_target`.
    fn parse_branch_svar_with_prior(
        &mut self,
        name: &str,
        parent_target: Option<&str>,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        let body = self.svars.get(name.trim())?.clone();
        let seed_sentinel = prior_target_used && parent_target.is_some();
        let mut branch = Chain {
            target: parent_target.map(str::to_string),
            effects: if seed_sentinel {
                vec![String::new()]
            } else {
                Vec::new()
            },
            ..Chain::default()
        };
        self.chain(&body, &mut branch)?;
        if seed_sentinel {
            if branch.effects.first().map(String::as_str) != Some("") {
                return None;
            }
            branch.effects.remove(0);
        }
        if branch.target.as_deref() != parent_target
            || branch.second_target.is_some()
            || branch.effects.is_empty()
        {
            return None;
        }
        Some(branch.effects)
    }

    /// `AB$/SP$/DB$ Detain` -> `Effect::Detain { target }` or
    /// `Effect::DetainFilter { filter }` (CR 701.35a).
    pub(super) fn detain_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        p.take("IsCurse");
        let defined = p.take("Defined");
        if !targets_here
            && (target.is_none() || prior_target_used)
            && let Some(raw) = defined.as_deref()
            && let Some(valid) = raw.strip_prefix("Valid ").map(str::trim)
        {
            if valid.is_empty()
                || valid.contains("Remembered")
                || valid.contains("Targeted")
                || valid.contains("Triggered")
            {
                return None;
            }
            let expr = self.filter_expr(valid)?;
            let name = self.body.filter_static("DETAIN", &expr);
            return Some(vec![format!("Effect::DetainFilter {{ filter: &{name} }}")]);
        }
        if defined.as_deref() == Some("Self")
            && !self.on_a_spell
            && !targets_here
            && (target.is_none() || prior_target_used)
        {
            return Some(vec![
                "Effect::Detain { target: TargetSpec::ThisObject }".to_string(),
            ]);
        }
        if !targets_here
            && target.is_none()
            && defined
                .as_deref()
                .is_some_and(|d| self.is_event_object_defined(d))
        {
            return Some(vec![
                "Effect::Detain { target: TargetSpec::EventObject }".to_string(),
            ]);
        }
        let targeted_ok = match defined.as_deref() {
            None => targets_here,
            Some("Targeted" | "ParentTarget") => !targets_here,
            Some(_) => false,
        };
        if targeted_ok
            && let Some(tgt) = target
            && (tgt.starts_with("TargetSpec::Object(")
                || tgt.starts_with("TargetSpec::ObjectOfEachOpponent("))
        {
            return Some(vec![format!("Effect::Detain {{ target: {tgt} }}")]);
        }
        None
    }

    /// `AB$/SP$/DB$ AddPhase` -> `Effect::AdditionalCombatPhase` (CR 500.8).
    pub(super) fn add_phase_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        if targets_here || (target.is_some() && !prior_target_used) || p.has("Defined") {
            return None;
        }
        if p.take("ExtraPhase").as_deref() != Some("Combat") {
            return None;
        }
        let followed_by_main_phase = match p.take("FollowedBy").as_deref() {
            None => false,
            Some("Main2") => true,
            Some(_) => return None,
        };
        let num_phases: usize = match p.take("NumPhases").as_deref() {
            None | Some("1") => 1,
            Some(raw) => {
                let n = self.plain_num(raw)?;
                usize::try_from(n).ok().filter(|&k| (1..=4).contains(&k))?
            }
        };
        let after_end_combat = match p.take("AfterPhase").as_deref() {
            None => false,
            Some("EndCombat") => true,
            Some(_) => return None,
        };
        let mut conds: Vec<String> = Vec::new();
        match p.take("ConditionPhases").as_deref() {
            Some("Main1,Main2" | "Main2,Main1") => conds.push(
                "Condition::Not(&Condition::Any(&[Condition::DuringCombat, \
                 Condition::DuringStep(StepKind::Upkeep), \
                 Condition::DuringStep(StepKind::Draw), \
                 Condition::DuringStep(StepKind::End)]))"
                    .to_string(),
            ),
            Some("BeginCombat->EndCombat") => conds.push("Condition::DuringCombat".to_string()),
            None => {
                let in_combat_context =
                    matches!(
                        self.trigger_mode.as_deref(),
                        Some(
                            "Attacks"
                                | "AttackerBlocked"
                                | "AttackerUnblocked"
                                | "Blocks"
                                | "DamageDone"
                                | "DamageDoneOnce"
                                | "Exerted"
                                | "AttackersDeclared"
                                | "AttackersDeclaredOneTarget"
                        )
                    ) || self.oracle.to_ascii_lowercase().contains("during combat")
                        || self
                            .oracle
                            .to_ascii_lowercase()
                            .contains("after this phase");
                if after_end_combat && !in_combat_context {
                    conds.push("Condition::DuringCombat".to_string());
                }
            }
            Some(_) => return None,
        }
        match p.take("ConditionPlayerTurn").as_deref() {
            None => {}
            Some("True") => conds.push("Condition::YourTurn".to_string()),
            Some(_) => return None,
        }
        let step = format!(
            "Effect::AdditionalCombatPhase {{ followed_by_main_phase: {followed_by_main_phase} }}"
        );
        let effs = vec![step; num_phases];
        if conds.is_empty() {
            return Some(effs);
        }
        let cond = match conds.as_slice() {
            [one] => one.clone(),
            many => format!("Condition::All(&[{}])", many.join(", ")),
        };
        Some(vec![format!(
            "Effect::IfCondition {{ condition: {cond}, then: &[{}], otherwise: &[] }}",
            effs.join(", ")
        )])
    }

    /// `AB$/DB$ ChooseNumber` -> `Effect::ChooseNumber { min, max }`.
    ///
    /// In the engine, `Effect::ChooseNumber` stores `Rider::ChosenNumber` on
    /// `res.source` while it is on the battlefield, so spells and extra
    /// modifiers (`Secretly$`, `Random$`, `ChooseAnyNumber$`, `Highest$`,
    /// dynamic bounds) are explicitly refused.
    pub(super) fn choose_number_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        prior_target_used: bool,
    ) -> Option<Vec<String>> {
        if self.on_a_spell
            || targets_here
            || (target.is_some() && !prior_target_used)
            || !matches!(p.take("Defined").as_deref(), None | Some("You"))
        {
            return None;
        }
        let min_raw = p.take("Min")?;
        let max_raw = p.take("Max")?;
        let min: u8 = self.plain_num(&min_raw)?.try_into().ok()?;
        let max: u8 = self.plain_num(&max_raw)?.try_into().ok()?;
        if min > max || !p.exhausted() {
            return None;
        }
        Some(vec![format!(
            "Effect::ChooseNumber {{ min: {min}, max: {max} }}"
        )])
    }

    /// General `ChoosePlayer` resolution chain -> `Effect::ChoosePlayerFrom { among }`
    /// followed by one or more steps consuming `ChosenPlayer` / `Player.Chosen`.
    ///
    /// Scopes `ChosenPlayer` / `Player.Chosen` strictly to this chain so
    /// `EnterModifier::ChooseOpponent` permanents elsewhere are never
    /// mis-lowered to `PlayerRel::Chosen`.
    #[allow(clippy::too_many_lines)]
    pub(super) fn choose_player_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        targets_here: bool,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let player_target_in_use = chain.target.as_deref().is_some_and(|t| {
            matches!(
                t,
                "TargetSpec::AnyPlayer"
                    | "TargetSpec::AnyOpponent"
                    | "TargetSpec::PlayerWhoAttackedThisTurn"
                    | "TargetSpec::Player(PlayerRel::Chosen)"
            )
        });
        if targets_here
            || player_target_in_use
            || p.has("ValidTgts")
            || p.has("Random")
            || p.has("Secretly")
            || p.has("RememberChosen")
            || !matches!(p.take("Defined").as_deref(), None | Some("You"))
        {
            return None;
        }
        let among = match p.take("Choices").as_deref() {
            None | Some("Player") => "PlayerRel::EachPlayer",
            Some("Opponent" | "Player.Opponent") => "PlayerRel::EachOpponent",
            Some("Player.Other") => "PlayerRel::EachOtherPlayer",
            Some("Player.withMostLife") => "PlayerRel::MostLifeOrTied",
            Some("Player.withLowestLife") => "PlayerRel::LeastLifeOrTied",
            Some(_) => return None,
        };
        p.take("ChoiceTitle");
        if !p.exhausted() {
            return None;
        }
        let mut cursor = sub.take()?;
        let mut out = vec![format!("Effect::ChoosePlayerFrom {{ among: {among} }}")];
        let mut consumed = false;
        let mut steps = 0usize;
        loop {
            steps += 1;
            if steps > 6 {
                return None;
            }
            let body = self.svars.get(cursor.trim())?.clone();
            let (s_api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if s_api == "Cleanup" {
                let cp = sp.take("ClearChosenPlayer");
                let rem = sp.take("ClearRemembered");
                let next = sp.take("SubAbility");
                if cp.as_deref() != Some("True")
                    || rem.as_deref().is_some_and(|v| v != "True")
                    || !sp.exhausted()
                {
                    return None;
                }
                *sub = next;
                break;
            }
            let next_sub = sp.take("SubAbility");
            // The Black Gate: `Effect | ValidTgts$ Creature | RememberObjects$ ChosenPlayer & Targeted | StaticAbilities$ CantBlock`.
            if s_api == "Effect"
                && chain.target.is_none()
                && sp.peek("ValidTgts") == Some("Creature")
                && matches!(
                    sp.peek("RememberObjects"),
                    Some("ChosenPlayer & Targeted" | "Targeted & ChosenPlayer")
                )
                && matches!(sp.peek("ExileOnMoved"), None | Some("Battlefield"))
                && matches!(sp.peek("Duration"), None | Some("EndOfTurn"))
                && let Some(sa_name) = sp.peek("StaticAbilities")
                && let Some(sa_body) = self.svars.get(sa_name.trim())
                && Self::is_black_gate_cant_block(sa_body)
            {
                sp.take("ValidTgts");
                sp.take("RememberObjects");
                sp.take("ExileOnMoved");
                sp.take("Duration");
                sp.take("StaticAbilities");
                if !sp.exhausted() {
                    return None;
                }
                chain.target = Some("TargetSpec::Object(&Filter::CREATURE)".to_string());
                out.push(
                    "Effect::CantBeBlockedByPlayerThisTurn { by: PlayerRel::Chosen }".to_string(),
                );
                consumed = true;
            } else if s_api == "GainControl"
                && !self.on_a_spell
                && matches!(sp.peek("Defined"), None | Some("Self"))
                && matches!(
                    sp.peek("NewController"),
                    Some("ChosenPlayer" | "Player.Chosen")
                )
            {
                sp.take("Defined");
                sp.take("NewController");
                if !sp.exhausted() {
                    return None;
                }
                out.push("Effect::ChangeController { new_controller: PlayerRel::Chosen }".into());
                consumed = true;
            } else {
                let uses_chosen = sp.entries.iter().any(|(_, v)| {
                    v == "ChosenPlayer" || v == "Player.Chosen" || v.contains("ChosenPlayer")
                });
                if !uses_chosen {
                    *sub = Some(cursor);
                    break;
                }
                if sp.has("ValidTgts")
                    || sp.has("UnlessCost")
                    || sp.has("AtEOT")
                    || sp.has("ReplaceDyingDefined")
                    || sp.entries.iter().any(|(k, v)| {
                        k.starts_with("Condition")
                            || v.contains("Targeted")
                            || v.contains("ParentTarget")
                    })
                {
                    return None;
                }
                let mut rewrote = false;
                for (k, v) in &mut sp.entries {
                    if matches!(k.as_str(), "Defined" | "DefinedPlayer" | "TokenOwner")
                        && matches!(v.as_str(), "ChosenPlayer" | "Player.Chosen")
                    {
                        *v = "Targeted".to_string();
                        rewrote = true;
                    }
                }
                if !rewrote
                    || sp
                        .entries
                        .iter()
                        .any(|(_, v)| v.contains("ChosenPlayer") || v.contains("Player.Chosen"))
                {
                    return None;
                }
                let step_effects = self.effect_of(
                    &s_api,
                    &mut sp,
                    Some("TargetSpec::Player(PlayerRel::Chosen)"),
                    false,
                    false,
                )?;
                if !sp.exhausted() || step_effects.is_empty() {
                    return None;
                }
                out.extend(step_effects);
                consumed = true;
            }
            match next_sub {
                Some(next) => cursor = next,
                None => break,
            }
        }
        consumed.then_some(out)
    }

    fn is_black_gate_cant_block(body: &str) -> bool {
        let Some((mode, mut p)) = Params::parse(body) else {
            return false;
        };
        p.drop_prose();
        mode == "CantBlockBy"
            && p.take("ValidAttacker").as_deref() == Some("Creature.IsRemembered")
            && p.take("ValidBlocker").as_deref() == Some("Creature.RememberedPlayerCtrl")
            && p.exhausted()
    }

    /// Goblin Charbelcher: `DigUntil | Valid$ Land | RememberRevealed$ True |
    /// RevealedDestination$ Library | RevealedLibraryPosition$ -1 | SubAbility$ DBBelch`
    /// -> `Effect::RevealUntilLandDealDamage { target: TargetSpec::AnyTarget }`.
    pub(super) fn goblin_charbelcher_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if chain.target.is_some()
            || !matches!(p.take("Defined").as_deref(), None | Some("You"))
            || !matches!(p.take("Amount").as_deref(), None | Some("1"))
            || p.take("RememberRevealed").as_deref() != Some("True")
            || p.take("RevealedDestination").as_deref() != Some("Library")
            || p.take("RevealedLibraryPosition").as_deref() != Some("-1")
        {
            return None;
        }
        p.claim_label("ValidDescription", "Valid");
        if p.take("Valid").as_deref() != Some("Land") || !p.exhausted() {
            return None;
        }
        let s1_name = sub.take()?;
        let s1_body = self.svars.get(s1_name.trim())?.clone();
        let (s1_api, mut p1) = Params::parse(&s1_body)?;
        p1.drop_prose();
        if s1_api != "DealDamage"
            || p1.take("ValidTgts").as_deref() != Some("Any")
            || p1.take("ConditionSVarCompare").as_deref() != Some("EQ0")
        {
            return None;
        }
        let dmg_var = p1.take("NumDmg")?;
        let mtn_var = p1.take("ConditionCheckSVar")?;
        let s2_name = p1.take("SubAbility")?;
        if !p1.exhausted()
            || self.svars.get(dmg_var.trim()).map(String::as_str)
                != Some("Remembered$Valid Card.nonLand")
            || self.svars.get(mtn_var.trim()).map(String::as_str)
                != Some("Remembered$Valid Card.Mountain")
        {
            return None;
        }
        let s2_body = self.svars.get(s2_name.trim())?.clone();
        let (s2_api, mut p2) = Params::parse(&s2_body)?;
        p2.drop_prose();
        if s2_api != "DealDamage"
            || !matches!(
                p2.take("Defined").as_deref(),
                Some("Targeted" | "ParentTarget")
            )
            || p2.take("ConditionSVarCompare").as_deref() != Some("EQ1")
            || p2.take("ConditionCheckSVar").as_deref() != Some(mtn_var.as_str())
        {
            return None;
        }
        let double_var = p2.take("NumDmg")?;
        let cleanup_name = p2.take("SubAbility");
        if !p2.exhausted()
            || self.svars.get(double_var.trim()).map(String::as_str)
                != Some("Remembered$Valid Card.nonLand/Times.2")
        {
            return None;
        }
        if let Some(cname) = cleanup_name {
            self.consume_optional_remember_cleanup(cname, sub)?;
        }
        chain.target = Some("TargetSpec::AnyTarget".to_string());
        Some(vec![
            "Effect::RevealUntilLandDealDamage { target: TargetSpec::AnyTarget }".to_string(),
        ])
    }

    /// Clean `RememberDestroyed` / `RememberMilled` / `RememberDiscarded` /
    /// `RememberChanged` chains whose `SubAbility$` steps count or test the
    /// affected objects via `Amount::ThisWay(ThisWay::...)` (or optional
    /// discard-then-draw `Effect::DiscardUpToThenDraw` / `Effect::DiscardAnyThenDraw`).
    #[allow(clippy::too_many_lines)]
    pub(super) fn remembered_this_way_chain(
        &mut self,
        api: &str,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
        flags: super::chains::LineFlags,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        let super::chains::LineFlags {
            targets_here,
            targets_a_player,
            single_target,
            ..
        } = flags;
        if api == "Discard"
            && !targets_here
            && p.peek("Mode") == Some("TgtChoose")
            && p.peek("Optional") == Some("True")
            && p.peek("RememberDiscarded") == Some("True")
            && matches!(p.peek("Defined"), None | Some("You"))
            && !p.has("DiscardValid")
            && let Some(effs) = self.optional_discard_then_draw_chain(p, sub)
        {
            return Some(effs);
        }
        let (kind, flag_key, is_single_action) = match api {
            "Destroy" if p.peek("RememberDestroyed") == Some("True") => (
                RememberedThisWay::Destroyed,
                "RememberDestroyed",
                single_target,
            ),
            "DestroyAll" if p.peek("RememberDestroyed") == Some("True") => {
                (RememberedThisWay::Destroyed, "RememberDestroyed", false)
            }
            "Mill" if p.peek("RememberMilled") == Some("True") => {
                let who =
                    self.wave11_player_of_line(p.peek("Defined"), target, targets_a_player)?;
                (
                    RememberedThisWay::Milled,
                    "RememberMilled",
                    Self::is_single_player_rel(who)
                        && matches!(p.peek("NumCards"), None | Some("1")),
                )
            }
            "Discard"
                if matches!(p.peek("Mode"), Some("Hand" | "TgtChoose" | "Random"))
                    && p.peek("RememberDiscarded") == Some("True") =>
            {
                let who =
                    self.wave11_player_of_line(p.peek("Defined"), target, targets_a_player)?;
                if !Self::is_single_player_rel(who) {
                    return None;
                }
                let single = p.peek("Mode") != Some("Hand")
                    && matches!(p.peek("NumCards"), None | Some("1"));
                (RememberedThisWay::Discarded, "RememberDiscarded", single)
            }
            "ChangeZone" | "ChangeZoneAll" if p.peek("RememberChanged") == Some("True") => {
                let single_cz = api == "ChangeZone"
                    && single_target
                    && !p.has("Hidden")
                    && matches!(p.peek("ChangeNum"), None | Some("1"));
                match (p.peek("Origin"), p.peek("Destination")) {
                    (Some("Battlefield"), Some("Exile")) => (
                        RememberedThisWay::ExiledFromBattlefield,
                        "RememberChanged",
                        single_cz,
                    ),
                    (Some("Battlefield"), Some("Hand")) => (
                        RememberedThisWay::ReturnedToHand,
                        "RememberChanged",
                        single_cz,
                    ),
                    (Some("Graveyard"), Some("Exile")) => (
                        RememberedThisWay::ExiledFromGraveyard,
                        "RememberChanged",
                        single_cz,
                    ),
                    (
                        Some("Stack" | "Stack,Battlefield" | "Battlefield,Stack" | "Hand"),
                        Some("Exile"),
                    ) => (RememberedThisWay::Exiled, "RememberChanged", single_cz),
                    (Some("Library"), Some("Exile"))
                        if api == "ChangeZone"
                            && p.peek("Defined") == Some("TopOfLibrary")
                            && !p.has("ExileFaceDown") =>
                    {
                        let who =
                            self.wave11_player_of_line(p.peek("DefinedPlayer"), target, false)?;
                        (
                            RememberedThisWay::Exiled,
                            "RememberChanged",
                            Self::is_single_player_rel(who)
                                && matches!(p.peek("ChangeNum"), None | Some("1")),
                        )
                    }
                    (Some("Hand"), Some("Library")) if api == "ChangeZoneAll" => {
                        let who = self.wave11_player_of_line(
                            p.peek("Defined"),
                            target,
                            targets_a_player,
                        )?;
                        if !Self::is_single_player_rel(who) {
                            return None;
                        }
                        (
                            RememberedThisWay::PutIntoLibraryFromHand,
                            "RememberChanged",
                            false,
                        )
                    }
                    _ => return None,
                }
            }
            "Dig"
                if (p.peek("ChangeNum") == Some("All")
                    || (p.peek("DigNum") == Some("1")
                        && matches!(p.peek("ChangeNum"), None | Some("1"))))
                    && p.peek("DestinationZone") == Some("Exile")
                    && p.peek("RememberChanged") == Some("True")
                    && matches!(p.peek("ChangeValid"), None | Some("Card")) =>
            {
                let who =
                    self.wave11_player_of_line(p.peek("Defined"), target, targets_a_player)?;
                if !Self::is_single_player_rel(who) {
                    return None;
                }
                (
                    RememberedThisWay::Exiled,
                    "RememberChanged",
                    p.peek("DigNum") == Some("1"),
                )
            }
            "Dig"
                if !targets_here
                    && p.peek("ChangeNum") == Some("All")
                    && p.peek("Reveal") == Some("True")
                    && p.peek("DestinationZone") == Some("Hand")
                    && p.peek("ChangeValid") == Some("Card")
                    && matches!(p.peek("Defined"), None | Some("You"))
                    && p.peek("RememberChanged") == Some("True") =>
            {
                (
                    RememberedThisWay::RevealedTopToHand,
                    "RememberChanged",
                    p.peek("DigNum") == Some("1"),
                )
            }
            _ => return None,
        };
        if p.take(flag_key).as_deref() != Some("True") {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        if api == "Dig" && kind == RememberedThisWay::Exiled {
            p.take("ChangeValid");
            p.take("IsCurse");
        }
        let mut out = self.effect_of_with_prior(api, p, target, flags)?;
        if !p.exhausted() {
            return None;
        }
        let mut cursor = sub.take();
        let mut consumed = false;
        let mut pending_cond: Option<String> = None;
        let mut pending_body: Vec<String> = Vec::new();
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 6 {
                return None;
            }
            let body = self.svars.get(name.trim())?.clone();
            let (s_api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if s_api == "Cleanup" && sp.take("ClearRemembered").as_deref() == Some("True") {
                *sub = sp.take("SubAbility");
                if !sp.exhausted() {
                    return None;
                }
                break;
            }
            let cond_next = sp.peek("SubAbility").map(str::to_string);
            if let Some(cond) =
                self.take_remembered_this_way_condition(&mut sp, kind, is_single_action)
            {
                if sp
                    .entries
                    .iter()
                    .any(|(_, v)| v.contains("Remembered") || v.contains("IsRemembered"))
                    || sp.has("UnlessCost")
                    || sp.has("AtEOT")
                {
                    return None;
                }
                sp.take("SubAbility");
                let step_effects = if sp.has("ValidTgts") {
                    if chain.target.is_some() || self.line_is_gated_on_kicking(&sp) {
                        return None;
                    }
                    sp.claim_label("ValidTgtsDesc", "ValidTgts");
                    let valid = sp.take("ValidTgts")?;
                    self.chain_target(&s_api, &mut sp, &valid, chain, None)?;
                    if chain.second_target.is_some() {
                        return None;
                    }
                    let step_target = Self::effect_target(chain);
                    let step_targets_player =
                        step_target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
                    self.effect_of(
                        &s_api,
                        &mut sp,
                        step_target.as_deref(),
                        true,
                        step_targets_player,
                    )?
                } else {
                    self.effect_of_followup(&s_api, &mut sp, target)?
                };
                if !sp.exhausted() || step_effects.is_empty() {
                    return None;
                }
                if pending_cond.as_deref() == Some(cond.as_str()) {
                    pending_body.extend(step_effects);
                } else {
                    if let Some(prev) = pending_cond.take() {
                        out.push(format!(
                            "Effect::IfCondition {{ condition: {prev}, then: &[{}], otherwise: &[] }}",
                            pending_body.join(", ")
                        ));
                        pending_body.clear();
                    }
                    pending_cond = Some(cond);
                    pending_body.extend(step_effects);
                }
                consumed = true;
                cursor = cond_next;
                continue;
            }
            let next_sub = sp.take("SubAbility");
            if let Some(step_effects) =
                self.try_remembered_this_way_counted(&s_api, &mut sp, kind, chain)
            {
                if let Some(prev) = pending_cond.take() {
                    out.push(format!(
                        "Effect::IfCondition {{ condition: {prev}, then: &[{}], otherwise: &[] }}",
                        pending_body.join(", ")
                    ));
                    pending_body.clear();
                }
                out.extend(step_effects);
                consumed = true;
                cursor = next_sub;
                continue;
            }
            // Non-remembered follow-up step: leave it for the normal chain walker
            // if we already consumed the remembered value.
            *sub = Some(name);
            break;
        }
        if let Some(prev) = pending_cond.take() {
            out.push(format!(
                "Effect::IfCondition {{ condition: {prev}, then: &[{}], otherwise: &[] }}",
                pending_body.join(", ")
            ));
        }
        consumed.then_some(out)
    }

    fn optional_discard_then_draw_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
    ) -> Option<Vec<String>> {
        let mut probe = p.clone();
        probe.take("Mode");
        probe.take("Defined");
        probe.take("Optional");
        probe.take("RememberDiscarded");
        if let Some(forget) = probe.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        let any_number = match probe.take("AnyNumber").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let up_to_count = if any_number {
            None
        } else {
            let n = self.plain_num(probe.take("NumCards").as_deref().unwrap_or("1"))?;
            Some(u8::try_from(n).ok().filter(|&k| k > 0)?)
        };
        if !probe.exhausted() {
            return None;
        }
        let next_name = sub.as_deref()?.to_string();
        let next_body = self.svars.get(next_name.trim())?.clone();
        let (next_api, mut dp) = Params::parse(&next_body)?;
        dp.drop_prose();
        if next_api != "Draw" || !matches!(dp.take("Defined").as_deref(), None | Some("You")) {
            return None;
        }
        let raw_num = dp.take("NumCards")?;
        let def = self
            .svars
            .get(raw_num.trim())
            .map_or(raw_num.trim(), String::as_str)
            .trim();
        let plus_one = match def {
            "Remembered$Amount" | "Count$RememberedSize" => false,
            "Remembered$Amount/Plus.1" | "Count$RememberedSize/Plus.1" => true,
            _ => return None,
        };
        let after_draw = dp.take("SubAbility");
        if !dp.exhausted() {
            return None;
        }
        *p = probe;
        *sub = None;
        if let Some(aname) = after_draw {
            if self.sub_is_api(Some(&aname), "Cleanup") {
                self.consume_optional_remember_cleanup(aname, sub)?;
            } else {
                *sub = Some(aname);
            }
        }
        let mut out = vec![match up_to_count {
            None => "Effect::DiscardAnyThenDraw".to_string(),
            Some(count) => format!("Effect::DiscardUpToThenDraw {{ count: {count} }}"),
        }];
        if plus_one {
            out.push("Effect::draw(1)".to_string());
        }
        Some(out)
    }

    fn take_remembered_this_way_condition(
        &mut self,
        sp: &mut Params,
        kind: RememberedThisWay,
        is_single_action: bool,
    ) -> Option<String> {
        if sp.peek("ConditionDefined") == Some("Remembered")
            && sp.peek("ConditionPresent").is_some()
        {
            let cmp = sp.peek("ConditionCompare");
            let positive_n = match cmp {
                None | Some("GE1" | "GT0") => Some(1u32),
                Some("EQ1") if is_single_action => Some(1u32),
                Some("EQ0" | "LT1" | "LE0") => Some(0u32),
                Some(c) if c.starts_with("GE") => c[2..].parse::<u32>().ok().filter(|&n| n >= 1),
                Some(_) => None,
            }?;
            sp.take("ConditionDefined");
            let valid = sp.take("ConditionPresent")?;
            sp.take("ConditionCompare");
            sp.take("ConditionDescription");
            let amt = self.this_way_amount_for_filter(kind, Some(&valid))?;
            return Some(if positive_n == 0 {
                format!("Condition::Not(&Condition::AmountAtLeast({amt}, 1))")
            } else {
                format!("Condition::AmountAtLeast({amt}, {positive_n})")
            });
        }
        if let Some(var) = sp.peek("ConditionCheckSVar").map(str::to_string)
            && let Some((amt, false)) = self.remembered_this_way_amount(&var, kind)
        {
            let cmp = sp.peek("ConditionSVarCompare");
            let positive_n = match cmp {
                None | Some("GE1" | "GT0") => Some(1u32),
                Some("EQ1") if is_single_action => Some(1u32),
                Some("EQ0" | "LT1" | "LE0") => Some(0u32),
                Some(c) if c.starts_with("GE") => c[2..].parse::<u32>().ok().filter(|&n| n >= 1),
                Some(_) => None,
            }?;
            sp.take("ConditionCheckSVar");
            sp.take("ConditionSVarCompare");
            sp.take("ConditionDescription");
            return Some(if positive_n == 0 {
                format!("Condition::Not(&Condition::AmountAtLeast({amt}, 1))")
            } else {
                format!("Condition::AmountAtLeast({amt}, {positive_n})")
            });
        }
        None
    }

    fn strip_redundant_in_zone(valid: &str, expected_atom: &str) -> String {
        valid
            .split(',')
            .map(|alt| {
                let alt = alt.trim();
                let Some((base, rest)) = alt.split_once('.') else {
                    return alt.to_string();
                };
                let kept: Vec<&str> = rest
                    .split('+')
                    .map(str::trim)
                    .filter(|a| !a.is_empty() && *a != expected_atom)
                    .collect();
                if kept.is_empty() {
                    base.trim().to_string()
                } else {
                    format!("{}.{}", base.trim(), kept.join("+"))
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    fn this_way_amount_for_filter(
        &mut self,
        kind: RememberedThisWay,
        valid: Option<&str>,
    ) -> Option<String> {
        let redundant_zone = match kind {
            RememberedThisWay::Milled
            | RememberedThisWay::Destroyed
            | RememberedThisWay::Discarded => Some("inZoneGraveyard"),
            RememberedThisWay::Exiled
            | RememberedThisWay::ExiledFromBattlefield
            | RememberedThisWay::ExiledFromGraveyard => Some("inZoneExile"),
            RememberedThisWay::ReturnedToHand | RememberedThisWay::RevealedTopToHand => {
                Some("inZoneHand")
            }
            RememberedThisWay::PutIntoLibraryFromHand => None,
        };
        let cleaned = valid.map(|v| {
            let v = v.trim();
            redundant_zone.map_or_else(|| v.to_string(), |z| Self::strip_redundant_in_zone(v, z))
        });
        let is_unrestricted = cleaned
            .as_deref()
            .is_none_or(|v| matches!(v, "Card" | "Permanent"));
        match kind {
            RememberedThisWay::Discarded => {
                if is_unrestricted {
                    Some("Amount::ThisWay(ThisWay::Discarded)".to_string())
                } else {
                    let expr = self.filter_expr_in_zone(cleaned.as_deref()?, true)?;
                    let name = self.body.filter_static("DISCARDED", &expr);
                    Some(format!(
                        "Amount::ThisWay(ThisWay::DiscardedMatching(&{name}))"
                    ))
                }
            }
            RememberedThisWay::PutIntoLibraryFromHand => {
                is_unrestricted.then(|| "Amount::ThisWay(ThisWay::PutIntoLibraryFromHand)".into())
            }
            RememberedThisWay::Destroyed
            | RememberedThisWay::Milled
            | RememberedThisWay::Exiled
            | RememberedThisWay::ExiledFromBattlefield
            | RememberedThisWay::ReturnedToHand
            | RememberedThisWay::ExiledFromGraveyard
            | RememberedThisWay::RevealedTopToHand => {
                let (variant, card_zone, prefix) = match kind {
                    RememberedThisWay::Destroyed => ("Destroyed", false, "DESTROYED"),
                    RememberedThisWay::Milled => ("Milled", true, "MILLED"),
                    RememberedThisWay::Exiled => ("Exiled", true, "EXILED"),
                    RememberedThisWay::ExiledFromBattlefield => {
                        ("ExiledFromBattlefield", false, "EXILED")
                    }
                    RememberedThisWay::ReturnedToHand => ("ReturnedToHand", false, "RETURNED"),
                    RememberedThisWay::ExiledFromGraveyard => {
                        ("ExiledFromGraveyard", true, "EXILED_GY")
                    }
                    RememberedThisWay::RevealedTopToHand => ("Revealed", true, "REVEALED"),
                    _ => return None,
                };
                let filter_ref = if is_unrestricted {
                    "&Filter::Any".to_string()
                } else {
                    let expr = self.filter_expr_in_zone(cleaned.as_deref()?, card_zone)?;
                    let name = self.body.filter_static(prefix, &expr);
                    format!("&{name}")
                };
                Some(format!("Amount::ThisWay(ThisWay::{variant}({filter_ref}))"))
            }
        }
    }

    fn remembered_this_way_amount(
        &mut self,
        raw: &str,
        kind: RememberedThisWay,
    ) -> Option<(String, bool)> {
        let trimmed = raw.trim();
        let neg = trimmed.starts_with('-');
        let key = trimmed.trim_start_matches(['+', '-']);
        let def = match self.svars.get(key) {
            Some(d) => d.trim().to_string(),
            None if key.contains('$') => key.to_string(),
            None => return None,
        };
        let (head, suffix) = def
            .split_once('/')
            .map_or((def.as_str(), ""), |(h, s)| (h, s));
        let base = if matches!(head, "Remembered$Amount" | "Count$RememberedSize") {
            self.this_way_amount_for_filter(kind, None)?
        } else if kind == RememberedThisWay::RevealedTopToHand
            && matches!(
                head,
                "Remembered$CardManaCost"
                    | "RememberedLKI$CardManaCost"
                    | "Remembered$TotalManaCost"
            )
        {
            "Amount::ThisWay(ThisWay::RevealedManaValue)".to_string()
        } else if let Some(valid) = head
            .strip_prefix("RememberedLKI$Valid ")
            .or_else(|| head.strip_prefix("Remembered$Valid "))
        {
            self.this_way_amount_for_filter(kind, Some(valid.trim()))?
        } else if let Some(valid) = head.strip_prefix("Count$ValidGraveyard ")
            && matches!(
                kind,
                RememberedThisWay::Milled
                    | RememberedThisWay::Discarded
                    | RememberedThisWay::Destroyed
            )
            && let Some(stripped) = Self::strip_remembered_filter(valid.trim())
        {
            self.this_way_amount_for_filter(kind, Some(&stripped))?
        } else if let Some(valid) = head.strip_prefix("Count$ValidExile ")
            && matches!(
                kind,
                RememberedThisWay::Exiled
                    | RememberedThisWay::ExiledFromBattlefield
                    | RememberedThisWay::ExiledFromGraveyard
            )
            && let Some(stripped) = Self::strip_remembered_filter(valid.trim())
        {
            self.this_way_amount_for_filter(kind, Some(&stripped))?
        } else {
            return None;
        };
        let mag = Self::wrap_amount_suffix(base, suffix)?;
        Some((mag, neg))
    }

    #[allow(clippy::too_many_lines)]
    fn try_remembered_this_way_counted(
        &mut self,
        api: &str,
        sp: &mut Params,
        kind: RememberedThisWay,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if sp.has("UnlessCost")
            || sp.has("AtEOT")
            || sp.has("ReplaceDyingDefined")
            || sp.entries.iter().any(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        let saved_target = chain.target.clone();
        let saved_up_to = chain.target_up_to;
        let saved_exact = chain.target_exact;
        let saved_is_x = chain.target_is_x;
        sp.claim_label("ValidTgtsDesc", "ValidTgts");
        let targets_here = if let Some(valid) = sp.take("ValidTgts") {
            if chain.target.is_some() || self.chain_target(api, sp, &valid, chain, None).is_none() {
                chain.target = saved_target;
                chain.target_up_to = saved_up_to;
                chain.target_exact = saved_exact;
                chain.target_is_x = saved_is_x;
                return None;
            }
            true
        } else {
            false
        };
        let restore = |c: &mut Chain| {
            c.target.clone_from(&saved_target);
            c.target_up_to = saved_up_to;
            c.target_exact = saved_exact;
            c.target_is_x = saved_is_x;
        };
        let target = Self::effect_target(chain);
        let targets_a_player =
            targets_here && target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
        let eff = match api {
            "GainLife" => {
                let Some(raw) = sp.take("LifeAmount") else {
                    restore(chain);
                    return None;
                };
                let Some((amt, false)) = self.remembered_this_way_amount(&raw, kind) else {
                    restore(chain);
                    return None;
                };
                let Some(who) = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    target.as_deref(),
                    targets_a_player,
                ) else {
                    restore(chain);
                    return None;
                };
                if !sp.exhausted() {
                    restore(chain);
                    return None;
                }
                if who == "PlayerRel::You" {
                    format!("Effect::GainLife {{ amount: {amt} }}")
                } else {
                    format!("Effect::GainLifeFor {{ amount: {amt}, who: {who} }}")
                }
            }
            "LoseLife" => {
                sp.take("IsCurse");
                let Some(raw) = sp.take("LifeAmount") else {
                    restore(chain);
                    return None;
                };
                let Some((amt, false)) = self.remembered_this_way_amount(&raw, kind) else {
                    restore(chain);
                    return None;
                };
                let Some(who) = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    target.as_deref(),
                    targets_a_player,
                ) else {
                    restore(chain);
                    return None;
                };
                if !sp.exhausted() {
                    restore(chain);
                    return None;
                }
                format!("Effect::LoseLife {{ amount: {amt}, target: {who} }}")
            }
            "Draw" => {
                let Some(raw) = sp.take("NumCards") else {
                    restore(chain);
                    return None;
                };
                let Some((amt, false)) = self.remembered_this_way_amount(&raw, kind) else {
                    restore(chain);
                    return None;
                };
                let Some(who) = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    target.as_deref(),
                    targets_a_player,
                ) else {
                    restore(chain);
                    return None;
                };
                if !sp.exhausted() {
                    restore(chain);
                    return None;
                }
                if who == "PlayerRel::You" {
                    format!("Effect::DrawCards {{ amount: {amt} }}")
                } else {
                    format!("Effect::DrawCardsFor {{ amount: {amt}, who: {who} }}")
                }
            }
            "Mill" => {
                sp.take("IsCurse");
                let Some(raw) = sp.take("NumCards") else {
                    restore(chain);
                    return None;
                };
                let Some((amt, false)) = self.remembered_this_way_amount(&raw, kind) else {
                    restore(chain);
                    return None;
                };
                let Some(who) = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    target.as_deref(),
                    targets_a_player,
                ) else {
                    restore(chain);
                    return None;
                };
                if !sp.exhausted() {
                    restore(chain);
                    return None;
                }
                format!("Effect::Mill {{ amount: {amt}, target: {who} }}")
            }
            "DealDamage" => {
                sp.take("IsCurse");
                let Some(raw) = sp.take("NumDmg") else {
                    restore(chain);
                    return None;
                };
                let Some((amt, false)) = self.remembered_this_way_amount(&raw, kind) else {
                    restore(chain);
                    return None;
                };
                let to = match sp.take("Defined").as_deref() {
                    None if targets_here => target.clone()?,
                    Some("You") => "TargetSpec::Player(PlayerRel::You)".to_string(),
                    Some(def) => {
                        let Some(who) = self.wave11_player_of_line(
                            Some(def),
                            target.as_deref(),
                            targets_a_player,
                        ) else {
                            restore(chain);
                            return None;
                        };
                        format!("TargetSpec::Player({who})")
                    }
                    None => {
                        restore(chain);
                        return None;
                    }
                };
                if !sp.exhausted() {
                    restore(chain);
                    return None;
                }
                format!("Effect::DealDamage {{ amount: {amt}, target: {to} }}")
            }
            "PutCounter" => {
                sp.take("IsCurse");
                let Some(raw) = sp.take("CounterNum") else {
                    restore(chain);
                    return None;
                };
                let Some((amt, false)) = self.remembered_this_way_amount(&raw, kind) else {
                    restore(chain);
                    return None;
                };
                let Some(code) = sp.take("CounterType") else {
                    restore(chain);
                    return None;
                };
                let Some(counter) = super::counter_kind(&code) else {
                    restore(chain);
                    return None;
                };
                let defined = sp.take("Defined");
                let on_self = match defined.as_deref() {
                    None | Some("Self") if !self.on_a_spell && !targets_here => true,
                    None if targets_here
                        && target
                            .as_deref()
                            .is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
                    {
                        false
                    }
                    _ => {
                        restore(chain);
                        return None;
                    }
                };
                if !sp.exhausted() {
                    restore(chain);
                    return None;
                }
                if on_self {
                    format!(
                        "Effect::AddCounterFilter {{ filter: &Filter::This, kind: {counter}, amount: {amt} }}"
                    )
                } else {
                    format!("Effect::AddCounter {{ kind: {counter}, amount: {amt} }}")
                }
            }
            "Mana" => {
                let Some(raw) = sp.take("Amount") else {
                    restore(chain);
                    return None;
                };
                let Some((amt, false)) = self.remembered_this_way_amount(&raw, kind) else {
                    restore(chain);
                    return None;
                };
                let Some(color) = sp.take("Produced").and_then(|c| match c.trim() {
                    "W" => Some("ManaColor::White"),
                    "U" => Some("ManaColor::Blue"),
                    "B" => Some("ManaColor::Black"),
                    "R" => Some("ManaColor::Red"),
                    "G" => Some("ManaColor::Green"),
                    "C" => Some("ManaColor::Colorless"),
                    _ => None,
                }) else {
                    restore(chain);
                    return None;
                };
                let Some(who) = self.wave11_player_of_line(
                    sp.take("Defined").as_deref(),
                    target.as_deref(),
                    targets_a_player,
                ) else {
                    restore(chain);
                    return None;
                };
                if !sp.exhausted() {
                    restore(chain);
                    return None;
                }
                format!("Effect::AddManaAmountFor {{ who: {who}, color: {color}, amount: {amt} }}")
            }
            "Pump" => {
                let att_raw = sp.take("NumAtt");
                let def_raw = sp.take("NumDef");
                let kw_raw = sp.take("KW");
                let defined = sp.take("Defined");
                sp.take("IsCurse");
                if sp.has("Duration") || !sp.exhausted() {
                    restore(chain);
                    return None;
                }
                let on_self = match defined.as_deref() {
                    None | Some("Self") if !self.on_a_spell && !targets_here => true,
                    None if targets_here
                        && target
                            .as_deref()
                            .is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
                    {
                        false
                    }
                    Some("Targeted")
                        if target
                            .as_deref()
                            .is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
                    {
                        false
                    }
                    _ => {
                        restore(chain);
                        return None;
                    }
                };
                let keywords = match kw_raw.as_deref() {
                    None => "KeywordSet::EMPTY".to_string(),
                    Some(raw) => {
                        let Some(bits) = raw
                            .split('&')
                            .map(|k| keyword_const(k.trim()))
                            .collect::<Option<Vec<_>>>()
                        else {
                            restore(chain);
                            return None;
                        };
                        bits.into_iter()
                            .map(str::to_string)
                            .reduce(|acc, bit| format!("{acc}.union({bit})"))
                            .unwrap_or_else(|| "KeywordSet::EMPTY".to_string())
                    }
                };
                let mut used = false;
                let mut side = |raw: Option<&str>| -> Option<String> {
                    match raw {
                        None => Some("Amount::Fixed(0)".to_string()),
                        Some(r) => {
                            if let Some((mag, neg)) = self.remembered_this_way_amount(r, kind) {
                                used = true;
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
                let (Some(att), Some(def)) = (side(att_raw.as_deref()), side(def_raw.as_deref()))
                else {
                    restore(chain);
                    return None;
                };
                if !used {
                    restore(chain);
                    return None;
                }
                if on_self {
                    format!(
                        "Effect::PumpFilter {{ filter: &Filter::This, controlled_by: None, \
                         power: {att}, toughness: {def}, keywords: {keywords}, \
                         duration: Duration::UntilEndOfTurn }}"
                    )
                } else {
                    format!(
                        "Effect::PumpTarget {{ power: {att}, toughness: {def}, \
                         keywords: {keywords}, duration: Duration::UntilEndOfTurn }}"
                    )
                }
            }
            "PumpAll" => {
                let att_raw = sp.take("NumAtt");
                let def_raw = sp.take("NumDef");
                let Some(valid) = sp.take("ValidCards") else {
                    restore(chain);
                    return None;
                };
                sp.take("IsCurse");
                if sp.has("KW") || sp.has("Duration") || !sp.exhausted() {
                    restore(chain);
                    return None;
                }
                let mut used = false;
                let mut side = |raw: Option<&str>| -> Option<String> {
                    match raw {
                        None => Some("Amount::Fixed(0)".to_string()),
                        Some(r) => {
                            if let Some((mag, neg)) = self.remembered_this_way_amount(r, kind) {
                                used = true;
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
                let (Some(att), Some(def)) = (side(att_raw.as_deref()), side(def_raw.as_deref()))
                else {
                    restore(chain);
                    return None;
                };
                if !used {
                    restore(chain);
                    return None;
                }
                let Some(expr) = self.filter_expr(&valid) else {
                    restore(chain);
                    return None;
                };
                let fname = self.body.filter_static("PUMP", &expr);
                format!(
                    "Effect::PumpFilter {{ filter: &{fname}, controlled_by: None, power: {att}, \
                     toughness: {def}, keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }}"
                )
            }
            "Token" => {
                let Some(raw) = sp.take("TokenAmount") else {
                    restore(chain);
                    return None;
                };
                let Some((amt, false)) = self.remembered_this_way_amount(&raw, kind) else {
                    restore(chain);
                    return None;
                };
                let raw_owner = sp.take("TokenOwner");
                let Some(owner) = (match raw_owner.as_deref() {
                    None | Some("You") => Some("PlayerRel::You"),
                    other => self.wave11_player_of_line(other, target.as_deref(), targets_a_player),
                }) else {
                    restore(chain);
                    return None;
                };
                let tapped = match sp.take("TokenTapped").as_deref() {
                    None => false,
                    Some("True") => true,
                    Some(_) => {
                        restore(chain);
                        return None;
                    }
                };
                let Some(stem) = sp.take("TokenScript") else {
                    restore(chain);
                    return None;
                };
                if !sp.exhausted() || (owner != "PlayerRel::You" && tapped) {
                    restore(chain);
                    return None;
                }
                let Some(tokens) = self.tokens else {
                    restore(chain);
                    return None;
                };
                let Some(body) = tokens.body(&stem, self.cats) else {
                    restore(chain);
                    return None;
                };
                let token = format!("&generated_tokens::{}", body.constant);
                if owner != "PlayerRel::You" {
                    format!(
                        "Effect::CreateTokensFor {{ player: {owner}, token: {token}, amount: {amt} }}"
                    )
                } else if tapped {
                    format!(
                        "Effect::CreateTokenModified {{ token: {token}, amount: {amt}, tapped: true, haste: false }}"
                    )
                } else {
                    format!("Effect::CreateTokenN {{ token: {token}, amount: {amt} }}")
                }
            }
            _ => {
                restore(chain);
                return None;
            }
        };
        Some(vec![eff])
    }

    // -------------------------------------------------------------------------
    // Wave 20b: RepeatProcess, ExileTargetsCreateTokens, ThisWay::Exiled,
    // hand-to-bottom remembered chains, and DB$ ManaReflected.
    // -------------------------------------------------------------------------

    /// `DB$ ManaReflected | ColorOrType$ Type | ReflectProperty$ Produced`
    /// under a `T:Mode$ TapsForMana` trigger -> `Effect::AddManaLikeEvent`
    /// (Mana Flare, Mirari's Wake, Kinnan, Zendikar Resurgent, Extraplanar Lens).
    pub(super) fn mana_reflected_db_effect(
        &self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
    ) -> Option<Vec<String>> {
        if self.trigger_mode.as_deref() != Some("TapsForMana")
            || self.in_delayed
            || targets_here
            || target.is_some()
        {
            return self.deny("`ManaReflected` outside a `TapsForMana` trigger".to_string());
        }
        if p.take("ColorOrType").as_deref() != Some("Type")
            || p.take("ReflectProperty").as_deref() != Some("Produced")
        {
            return self.deny("`ManaReflected` not reflecting produced mana type".to_string());
        }
        let amount: u16 = match p.take("Amount") {
            None => 1,
            Some(raw) => self
                .plain_num(&raw)
                .and_then(|n| u16::try_from(n).ok())
                .filter(|&n| n >= 1)?,
        };
        match p.take("Defined").as_deref() {
            Some("TriggeredActivator" | "TriggeredCardController") => {}
            None | Some("You") => {
                let oracle_lower = self.oracle.to_ascii_lowercase();
                let trigger_filters_you = self
                    .body
                    .statics
                    .rsplit("static TRIGGER")
                    .next()
                    .filter(|_| self.body.statics.contains("static TRIGGER"))
                    .is_some_and(|s| s.contains("ControlledBy(PlayerRel::You)"));
                if !trigger_filters_you
                    && !oracle_lower.contains("whenever you tap ")
                    && !oracle_lower.contains("you control is tapped for mana")
                {
                    return self.deny(
                        "`ManaReflected` with `Defined$ You` on a non-self mana trigger"
                            .to_string(),
                    );
                }
            }
            Some(other) => {
                return self.deny(format!("`ManaReflected` with `Defined$ {other}`"));
            }
        }
        Some(vec![format!(
            "Effect::AddManaLikeEvent {{ amount: {amount} }}"
        )])
    }

    /// `ChangeZone | Origin$ Battlefield | Destination$ Exile | RememberLKI$ True`
    /// (or `RememberChanged$ True`) chained into `DB$ RepeatEach` creating one
    /// token per exiled target for that permanent's controller ->
    /// `Effect::ExileTargetsCreateTokens` (Curse of the Swine, Hour of Need).
    pub(super) fn exile_targets_create_tokens_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if !target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) {
            return self.deny(
                "`ChangeZone` exile-for-tokens without a battlefield object target".to_string(),
            );
        }
        if p.take("Origin").as_deref() != Some("Battlefield")
            || p.take("Destination").as_deref() != Some("Exile")
        {
            return None;
        }
        let has_lki = p.take("RememberLKI").as_deref() == Some("True");
        let has_changed = p.take("RememberChanged").as_deref() == Some("True");
        if !has_lki && !has_changed {
            return None;
        }
        if let Some(forget) = p.take("ForgetOtherRemembered")
            && forget != "True"
        {
            return None;
        }
        p.take("IsCurse");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChangeZone.{key}`"));
        }
        let rep_name = sub.take()?;
        let rep_body = self.svars.get(rep_name.trim())?.clone();
        let (rep_api, mut rp) = Params::parse(&rep_body)?;
        rp.drop_prose();
        if rep_api != "RepeatEach" {
            return None;
        }
        if !matches!(
            rp.take("DefinedCards").as_deref(),
            Some("DirectRemembered" | "Remembered" | "RememberedLKI")
        ) {
            return self
                .deny("`RepeatEach` after exile not iterating remembered cards".to_string());
        }
        let use_imprinted = match rp.take("UseImprinted").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`RepeatEach` with `UseImprinted$ {other}`"));
            }
        };
        if let Some(czt) = rp.take("ChangeZoneTable")
            && czt != "True"
        {
            return self.deny(format!("`RepeatEach` with `ChangeZoneTable$ {czt}`"));
        }
        let tok_name = rp.take("RepeatSubAbility")?;
        let after_rep = rp.take("SubAbility");
        if let Some(key) = rp.first_key() {
            return self.deny(format!("unclaimed parameter `RepeatEach.{key}`"));
        }
        let tok_body = self.svars.get(tok_name.trim())?.clone();
        let (tok_api, mut tp) = Params::parse(&tok_body)?;
        tp.drop_prose();
        if tok_api != "Token" {
            return self.deny("`RepeatEach` after exile not creating a token".to_string());
        }
        let expected_owner = if use_imprinted {
            "ImprintedController"
        } else {
            "RememberedController"
        };
        if tp.take("TokenOwner").as_deref() != Some(expected_owner) {
            return self.deny(
                "`Token` in exile-for-tokens not owned by exiled permanent's controller"
                    .to_string(),
            );
        }
        if !matches!(tp.take("TokenAmount").as_deref(), None | Some("1")) {
            return self
                .deny("`Token` in exile-for-tokens creating more than one token".to_string());
        }
        let stem = tp.take("TokenScript")?;
        if let Some(key) = tp.first_key() {
            return self.deny(format!("unclaimed parameter `Token.{key}`"));
        }
        let body = self.tokens?.body(&stem, self.cats)?;
        let token = format!("&generated_tokens::{}", body.constant);
        if let Some(after) = after_rep {
            if self.sub_is_api(Some(&after), "Cleanup") {
                self.consume_optional_remember_cleanup(after, sub)?;
            } else {
                *sub = Some(after);
            }
        }
        Some(vec![format!(
            "Effect::ExileTargetsCreateTokens {{ token: {token} }}"
        )])
    }

    /// `ChangeZone | Origin$ Graveyard | Destination$ Exile | TargetMin$ X | TargetMax$ X`
    /// followed by `DB$ Token | TokenAmount$ X` when Oracle text says
    /// "for each ... exiled this way" (Midnight Ritual; CR 608.2b).
    pub(super) fn midnight_ritual_exile_tokens_effect(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let target = target.filter(|t| t.starts_with("TargetSpec::CardInGraveyard"))?;
        let oracle_lower = self.oracle.to_ascii_lowercase();
        if !oracle_lower.contains("for each") || !oracle_lower.contains("exiled this way") {
            return None;
        }
        if p.take("Origin").as_deref() != Some("Graveyard")
            || p.take("Destination").as_deref() != Some("Exile")
            || !p.exhausted()
        {
            return None;
        }
        let tok_name = sub.take()?;
        let tok_body = self.svars.get(tok_name.trim())?.clone();
        let (tok_api, mut tp) = Params::parse(&tok_body)?;
        tp.drop_prose();
        if tok_api != "Token" || !matches!(tp.take("TokenOwner").as_deref(), None | Some("You")) {
            return None;
        }
        let raw_amt = tp.take("TokenAmount")?;
        if self.svars.get(raw_amt.trim()).map(|s| s.trim()) != Some("Count$xPaid") {
            return None;
        }
        let tapped = match tp.take("TokenTapped").as_deref() {
            None => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let stem = tp.take("TokenScript")?;
        let next_sub = tp.take("SubAbility");
        if !tp.exhausted() {
            return None;
        }
        let body = self.tokens?.body(&stem, self.cats)?;
        let token = format!("&generated_tokens::{}", body.constant);
        *sub = next_sub;
        let create = if tapped {
            format!(
                "Effect::CreateTokenModified {{ token: {token}, \
                 amount: Amount::ThisWay(ThisWay::Exiled(&Filter::Any)), \
                 tapped: true, haste: false }}"
            )
        } else {
            format!(
                "Effect::CreateTokenN {{ token: {token}, \
                 amount: Amount::ThisWay(ThisWay::Exiled(&Filter::Any)) }}"
            )
        };
        Some(vec![format!("Effect::exile({target})"), create])
    }

    /// `ChangeZone | Origin$ Hand | Destination$ Library | LibraryPosition$ -1 |
    /// RememberChanged$ True` followed by either:
    /// - "If you do, [then]" (`Effect::BottomCardFromHand`, Volcanic Spite), or
    /// - "put any number of cards ... then draw that many ..." (`Effect::PutAnyFromHandOnBottom`
    ///   + counted `ThisWay::PutIntoLibraryFromHand`, Valakut Awakening).
    #[allow(clippy::too_many_lines)]
    pub(super) fn hand_to_bottom_remembered_chain(
        &mut self,
        p: &mut Params,
        sub: &mut Option<String>,
        chain: &mut Chain,
    ) -> Option<Vec<String>> {
        if p.take("Origin").as_deref() != Some("Hand")
            || p.take("Destination").as_deref() != Some("Library")
            || p.take("LibraryPosition").as_deref() != Some("-1")
            || p.take("RememberChanged").as_deref() != Some("True")
        {
            return None;
        }
        if !matches!(p.take("DefinedPlayer").as_deref(), None | Some("You"))
            || !matches!(p.take("Hidden").as_deref(), None | Some("True"))
            || !matches!(
                p.take("ForgetOtherRemembered").as_deref(),
                None | Some("True")
            )
            || p.has("Mandatory")
        {
            return None;
        }
        let prompt_may = p.take("SelectPrompt").is_some_and(|s| {
            let lower = s.to_ascii_lowercase();
            lower.starts_with("you may")
                || lower.starts_with("select up to ")
                || lower.starts_with("choose up to ")
        });
        let opt_raw = p.take("Optional");
        let change_num = p.take("ChangeNum");
        let change_type = p.take("ChangeType");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChangeZone.{key}`"));
        }
        // Form B (Valakut Awakening): put any number of cards from hand on bottom,
        // then run counted `ThisWay::PutIntoLibraryFromHand` follow-ups.
        if let Some(ref num_var) = change_num
            && self
                .svars
                .get(num_var.trim())
                .is_some_and(|def| def.trim() == "Count$ValidHand Card.YouOwn")
        {
            if !matches!(change_type.as_deref(), None | Some("Card"))
                || !matches!(opt_raw.as_deref(), None | Some("True"))
            {
                return None;
            }
            let mut out = vec!["Effect::PutAnyFromHandOnBottom".to_string()];
            let mut cursor = sub.take();
            let mut consumed = false;
            let mut steps = 0usize;
            while let Some(name) = cursor.take() {
                steps += 1;
                if steps > 4 {
                    return None;
                }
                let body = self.svars.get(name.trim())?.clone();
                let (s_api, mut sp) = Params::parse(&body)?;
                sp.drop_prose();
                if s_api == "Cleanup" && sp.take("ClearRemembered").as_deref() == Some("True") {
                    *sub = sp.take("SubAbility");
                    if !sp.exhausted() {
                        return None;
                    }
                    break;
                }
                let next_sub = sp.take("SubAbility");
                if let Some(step_effs) = self.try_remembered_this_way_counted(
                    &s_api,
                    &mut sp,
                    RememberedThisWay::PutIntoLibraryFromHand,
                    chain,
                ) {
                    out.extend(step_effs);
                    consumed = true;
                    cursor = next_sub;
                    continue;
                }
                *sub = Some(name);
                break;
            }
            return consumed.then_some(out);
        }
        // Form A (Volcanic Spite): optionally put 1 card from hand on bottom;
        // if you do, run follow-up steps.
        if !matches!(change_num.as_deref(), None | Some("1")) {
            return None;
        }
        let optional = match opt_raw.as_deref() {
            Some("True") => true,
            None => prompt_may || self.oracle.to_ascii_lowercase().contains("you may put "),
            Some(_) => return None,
        };
        if !optional {
            return None;
        }
        let filter = match change_type.as_deref() {
            None | Some("Card") => "Filter::Any".to_string(),
            Some(valid) => {
                let expr = self.filter_expr_in_zone(valid, true)?;
                self.body.filter_static("BOTTOM", &expr)
            }
        };
        let target = Self::effect_target(chain);
        let mut cursor = sub.take();
        let mut then_effects = Vec::new();
        let mut steps = 0usize;
        while let Some(name) = cursor.take() {
            steps += 1;
            if steps > 6 {
                return None;
            }
            let body = self.svars.get(name.trim())?.clone();
            let (s_api, mut sp) = Params::parse(&body)?;
            sp.drop_prose();
            if s_api == "Cleanup" && sp.take("ClearRemembered").as_deref() == Some("True") {
                *sub = sp.take("SubAbility");
                if !sp.exhausted() {
                    return None;
                }
                break;
            }
            let cond_ok = if sp.peek("ConditionDefined") == Some("Remembered")
                && matches!(sp.peek("ConditionPresent"), Some("Card" | "Permanent"))
                && matches!(
                    sp.peek("ConditionCompare"),
                    None | Some("GE1" | "EQ1" | "GT0")
                ) {
                sp.take("ConditionDefined");
                sp.take("ConditionPresent");
                sp.take("ConditionCompare");
                sp.take("ConditionDescription");
                true
            } else if let Some(var) = sp.peek("ConditionCheckSVar")
                && self.svars.get(var.trim()).is_some_and(|def| {
                    matches!(def.trim(), "Remembered$Amount" | "Count$RememberedSize")
                })
                && matches!(
                    sp.peek("ConditionSVarCompare"),
                    None | Some("GE1" | "EQ1" | "GT0")
                )
            {
                sp.take("ConditionCheckSVar");
                sp.take("ConditionSVarCompare");
                sp.take("ConditionDescription");
                true
            } else {
                false
            };
            if !cond_ok || sp.has("ValidTgts") || sp.has("UnlessCost") || sp.has("AtEOT") {
                return None;
            }
            let next = sp.take("SubAbility");
            let step_effs = self.effect_of_followup(&s_api, &mut sp, target.as_deref())?;
            if !sp.exhausted() || step_effs.is_empty() {
                return None;
            }
            then_effects.extend(step_effs);
            cursor = next;
        }
        if then_effects.is_empty() {
            return None;
        }
        Some(vec![format!(
            "Effect::BottomCardFromHand {{ player: PlayerRel::You, filter: &{filter}, \
             then: &[{}] }}",
            then_effects.join(", ")
        )])
    }

    fn repeat_present_condition(&mut self, valid: &str, cmp: Option<&str>) -> Option<String> {
        if valid.contains("Remembered") || valid.contains("Imprinted") || valid.contains("Targeted")
        {
            return None;
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("REPEAT_COND", &expr);
        let count = format!("Amount::CountOf {{ filter: &{name}, zone: ZoneSel::Battlefield }}");
        match cmp.unwrap_or("GE1") {
            "EQ0" | "LT1" | "LE0" => Some(format!(
                "Condition::Not(&Condition::AmountAtLeast({count}, 1))"
            )),
            s if s.starts_with("GE") => {
                let n: u32 = s[2..].parse().ok().filter(|&n| n >= 1)?;
                Some(format!("Condition::AmountAtLeast({count}, {n})"))
            }
            s if s.starts_with("GT") => {
                let n: u32 = s[2..].parse().ok()?;
                Some(format!(
                    "Condition::AmountAtLeast({count}, {})",
                    n.checked_add(1)?
                ))
            }
            s if s.starts_with("LT") => {
                let n: u32 = s[2..].parse().ok().filter(|&n| n >= 2)?;
                Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast({count}, {n}))"
                ))
            }
            s if s.starts_with("LE") => {
                let n: u32 = s[2..].parse().ok().filter(|&n| n >= 1)?;
                Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast({count}, {}))",
                    n.checked_add(1)?
                ))
            }
            s if s.starts_with("EQ") => {
                let n: u32 = s[2..].parse().ok().filter(|&n| n >= 1)?;
                Some(format!(
                    "Condition::AmountCompare {{ left: &{count}, op: AmountCmp::Equal, right: &Amount::Fixed({n}) }}"
                ))
            }
            _ => None,
        }
    }

    fn repeat_this_way_pass_kind(
        &self,
        api: &str,
        rp: &Params,
        target: Option<&str>,
    ) -> Option<(RememberedThisWay, &'static str, bool)> {
        match api {
            "Destroy" if rp.peek("RememberDestroyed") == Some("True") => {
                Some((RememberedThisWay::Destroyed, "RememberDestroyed", true))
            }
            "DestroyAll" if rp.peek("RememberDestroyed") == Some("True") => {
                Some((RememberedThisWay::Destroyed, "RememberDestroyed", false))
            }
            "Mill" if rp.peek("RememberMilled") == Some("True") => {
                let who = self.wave11_player_of_line(rp.peek("Defined"), target, false)?;
                Self::is_single_player_rel(who).then_some((
                    RememberedThisWay::Milled,
                    "RememberMilled",
                    matches!(rp.peek("NumCards"), None | Some("1")),
                ))
            }
            "Discard"
                if matches!(rp.peek("Mode"), Some("Hand" | "TgtChoose" | "Random"))
                    && rp.peek("RememberDiscarded") == Some("True") =>
            {
                let who = self.wave11_player_of_line(rp.peek("Defined"), target, false)?;
                let single = rp.peek("Mode") != Some("Hand")
                    && matches!(rp.peek("NumCards"), None | Some("1"));
                Self::is_single_player_rel(who).then_some((
                    RememberedThisWay::Discarded,
                    "RememberDiscarded",
                    single,
                ))
            }
            "ChangeZone" | "ChangeZoneAll" if rp.peek("RememberChanged") == Some("True") => {
                let single_cz = api == "ChangeZone"
                    && !rp.has("Hidden")
                    && matches!(rp.peek("ChangeNum"), None | Some("1"));
                match (rp.peek("Origin"), rp.peek("Destination")) {
                    (Some("Battlefield"), Some("Exile")) => Some((
                        RememberedThisWay::ExiledFromBattlefield,
                        "RememberChanged",
                        single_cz,
                    )),
                    (Some("Battlefield"), Some("Hand")) => Some((
                        RememberedThisWay::ReturnedToHand,
                        "RememberChanged",
                        single_cz,
                    )),
                    (Some("Graveyard"), Some("Exile")) => Some((
                        RememberedThisWay::ExiledFromGraveyard,
                        "RememberChanged",
                        single_cz,
                    )),
                    (
                        Some("Stack" | "Stack,Battlefield" | "Battlefield,Stack" | "Hand"),
                        Some("Exile"),
                    ) => Some((RememberedThisWay::Exiled, "RememberChanged", single_cz)),
                    _ => None,
                }
            }
            "Dig"
                if rp.peek("ChangeNum") == Some("All")
                    && rp.peek("DestinationZone") == Some("Exile")
                    && rp.peek("RememberChanged") == Some("True")
                    && matches!(rp.peek("ChangeValid"), None | Some("Card")) =>
            {
                let who = self.wave11_player_of_line(rp.peek("Defined"), target, false)?;
                Self::is_single_player_rel(who).then_some((
                    RememberedThisWay::Exiled,
                    "RememberChanged",
                    rp.peek("DigNum") == Some("1"),
                ))
            }
            _ => None,
        }
    }

    fn format_this_way_cmp(amt: &str, cmp: Option<&str>, single_action: bool) -> Option<String> {
        match cmp {
            None | Some("GE1" | "GT0") => Some(format!("Condition::AmountAtLeast({amt}, 1)")),
            Some("EQ1") if single_action => Some(format!("Condition::AmountAtLeast({amt}, 1)")),
            Some("EQ0" | "LT1" | "LE0") => Some(format!(
                "Condition::Not(&Condition::AmountAtLeast({amt}, 1))"
            )),
            Some(c) if c.starts_with("GE") => {
                let n: u32 = c[2..].parse().ok().filter(|&n| n >= 1)?;
                Some(format!("Condition::AmountAtLeast({amt}, {n})"))
            }
            Some(c) if c.starts_with("GT") => {
                let n: u32 = c[2..].parse().ok()?;
                Some(format!(
                    "Condition::AmountAtLeast({amt}, {})",
                    n.checked_add(1)?
                ))
            }
            Some(c) if c.starts_with("LT") => {
                let n: u32 = c[2..].parse().ok().filter(|&n| n >= 2)?;
                Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast({amt}, {n}))"
                ))
            }
            Some(c) if c.starts_with("LE") => {
                let n: u32 = c[2..].parse().ok().filter(|&n| n >= 1)?;
                Some(format!(
                    "Condition::Not(&Condition::AmountAtLeast({amt}, {}))",
                    n.checked_add(1)?
                ))
            }
            Some(_) => None,
        }
    }

    /// `AB$/SP$/DB$ Repeat` -> `Effect::RepeatProcess` (CR 608.2c):
    /// bounded `MaxRepeat$`, optional `"you may repeat this process"`,
    /// pre-condition, and post-condition (`RepeatPresent$`, `RepeatCheckSVar$`,
    /// or per-pass `RepeatDefined$ Remembered`).
    #[allow(clippy::too_many_lines)]
    pub(super) fn repeat_process_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<()> {
        p.take("IsCurse");
        let Some(repeat_sub) = p.take("RepeatSubAbility") else {
            return self.deny("`Repeat` with no `RepeatSubAbility$`".to_string());
        };
        let optional = match p.take("RepeatOptional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`Repeat` with `RepeatOptional$ {other}`"));
            }
        };
        if let Some(decider) = p.take("RepeatOptionalDecider")
            && (!optional || decider != "You")
        {
            return self.deny(format!("`Repeat` with `RepeatOptionalDecider$ {decider}`"));
        }
        if let Some(mand) = p.take("Mandatory")
            && (optional || mand != "True")
        {
            return self.deny(format!("`Repeat` with `Mandatory$ {mand}`"));
        }
        let max_times = match p.take("MaxRepeat") {
            None => None,
            Some(raw) => {
                let amt = self.amount_or_count_ext(&raw)?;
                if amt == "Amount::Fixed(0)" {
                    return self.deny("`Repeat` with zero `MaxRepeat$`".to_string());
                }
                Some(amt)
            }
        };
        let pre_condition = match self.take_wave8_line_condition(&mut p) {
            Ok(Some(cond)) => Some(cond),
            Ok(None) => None,
            Err(()) => {
                let var = p.take("ConditionCheckSVar")?;
                let cmp = p
                    .take("ConditionSVarCompare")
                    .unwrap_or_else(|| "GE1".to_string());
                p.take("ConditionDescription");
                Some(
                    self.wave11_amount_compare_condition(&var, &cmp, true)
                        .or_else(|| Self::wave16_svar_condition(var.trim(), &cmp))?,
                )
            }
        };
        let repeat_defined = p.take("RepeatDefined");
        let repeat_present = p.take("RepeatPresent");
        let repeat_compare = p.take("RepeatCompare");
        let repeat_check_svar = p.take("RepeatCheckSVar");
        let repeat_svar_compare = p.take("RepeatSVarCompare");
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Repeat.{key}`"));
        }
        let Some(repeat_body) = self.svars.get(repeat_sub.trim()).cloned() else {
            return self.deny(format!("`RepeatSubAbility$ {repeat_sub}` names no SVar"));
        };
        let wants_remembered_post = repeat_defined.as_deref() == Some("Remembered")
            || repeat_check_svar.as_deref().is_some_and(|v| {
                self.svars.get(v.trim()).is_some_and(|def| {
                    def.contains("Remembered$")
                        || def.contains("RememberedLKI$")
                        || def.contains("RememberedSize")
                })
            });
        let mut pass_chain = Chain {
            target: chain.target.clone(),
            target_up_to: chain.target_up_to,
            target_exact: chain.target_exact,
            target_is_x: chain.target_is_x,
            ..Chain::default()
        };
        let per_pass_this_way;
        let post_condition = if wants_remembered_post {
            let (raw_api, mut rp) = Params::parse(&repeat_body)?;
            rp.drop_prose();
            if rp.has("ValidTgts") || rp.has("SubAbility") {
                return self
                    .deny("`Repeat` remembered post-condition on multi-step pass".to_string());
            }
            let Some((kind, flag_key, single_action)) =
                self.repeat_this_way_pass_kind(&raw_api, &rp, chain.target.as_deref())
            else {
                return self.deny("`Repeat` with unsupported remembered pass action".to_string());
            };
            rp.take(flag_key);
            if let Some(forget) = rp.take("ForgetOtherRemembered")
                && forget != "True"
            {
                return None;
            }
            if raw_api == "Dig" {
                rp.take("ChangeValid");
                rp.take("IsCurse");
            }
            let single_target = chain.target_exact.is_none()
                && !chain.target_is_x
                && chain.target_up_to.is_none_or(|n| n == 1);
            let step_effs = self.effect_of_with_prior(
                &raw_api,
                &mut rp,
                chain.target.as_deref(),
                super::chains::LineFlags {
                    targets_here: false,
                    targets_a_player: false,
                    single_target,
                    prior_target_used: false,
                },
            )?;
            if !rp.exhausted() || step_effs.is_empty() {
                return self.deny("`Repeat` pass action with unclaimed parameters".to_string());
            }
            pass_chain.effects = step_effs;
            per_pass_this_way = true;
            if repeat_defined.as_deref() == Some("Remembered") {
                if repeat_check_svar.is_some() || repeat_svar_compare.is_some() {
                    return self.deny(
                        "`Repeat` combining `RepeatDefined$` and `RepeatCheckSVar$`".to_string(),
                    );
                }
                let valid = repeat_present?;
                let amt = self.this_way_amount_for_filter(kind, Some(&valid))?;
                Some(Self::format_this_way_cmp(
                    &amt,
                    repeat_compare.as_deref(),
                    single_action,
                )?)
            } else {
                if repeat_present.is_some() || repeat_compare.is_some() {
                    return self.deny(
                        "`Repeat` combining `RepeatCheckSVar$` and `RepeatPresent$`".to_string(),
                    );
                }
                let var = repeat_check_svar?;
                let (amt, false) = self.remembered_this_way_amount(&var, kind)? else {
                    return None;
                };
                Some(Self::format_this_way_cmp(
                    &amt,
                    repeat_svar_compare.as_deref(),
                    single_action,
                )?)
            }
        } else {
            self.chain(&repeat_body, &mut pass_chain)?;
            if pass_chain.effects.is_empty()
                || pass_chain.target != chain.target
                || pass_chain.second_target.is_some()
            {
                return self.deny(
                    "`RepeatSubAbility$` declaring nested targets or no effects".to_string(),
                );
            }
            per_pass_this_way = pass_chain.effects.iter().any(|e| e.contains("ThisWay::"));
            match (repeat_defined, repeat_present, repeat_check_svar) {
                (None, None, None) => {
                    if repeat_compare.is_some() || repeat_svar_compare.is_some() {
                        return self.deny(
                            "`Repeat` comparator without `RepeatPresent$` or `RepeatCheckSVar$`"
                                .to_string(),
                        );
                    }
                    None
                }
                (None, Some(valid), None) => {
                    if repeat_svar_compare.is_some() {
                        return self.deny(
                            "`Repeat` with `RepeatSVarCompare$` beside `RepeatPresent$`"
                                .to_string(),
                        );
                    }
                    Some(self.repeat_present_condition(&valid, repeat_compare.as_deref())?)
                }
                (None, None, Some(var)) => {
                    if repeat_compare.is_some() {
                        return self.deny(
                            "`Repeat` with `RepeatCompare$` beside `RepeatCheckSVar$`".to_string(),
                        );
                    }
                    let cmp = repeat_svar_compare.unwrap_or_else(|| "GE1".to_string());
                    let cond = self
                        .svar_condition(&var, &cmp)
                        .or_else(|| self.extra_svar_condition_clause(&var, &cmp))
                        .or_else(|| self.wave7_svar_condition(&var, &cmp))
                        .or_else(|| Self::wave16_svar_condition(var.trim(), &cmp))
                        .or_else(|| self.wave11_amount_compare_condition(&var, &cmp, true))?;
                    Some(cond)
                }
                _ => {
                    return self.deny("`Repeat` with unsupported post-condition".to_string());
                }
            }
        };
        if max_times.is_none() && !optional && post_condition.is_none() {
            return self
                .deny("unbounded `Repeat` with no cap, choice, or stop condition".to_string());
        }
        let optional_before_first = optional
            && self
                .oracle
                .to_ascii_lowercase()
                .contains("as many times as you choose");
        chain.could_add_mana |= pass_chain.could_add_mana;
        chain.moves_library |= pass_chain.moves_library;
        let max_expr = max_times.map_or_else(|| "None".to_string(), |m| format!("Some({m})"));
        let pre_expr = pre_condition.map_or_else(|| "None".to_string(), |c| format!("Some({c})"));
        let post_expr = post_condition.map_or_else(|| "None".to_string(), |c| format!("Some({c})"));
        chain.effects.push(format!(
            "Effect::RepeatProcess {{ effects: &[{}], max_times: {max_expr}, \
             pre_condition: {pre_expr}, post_condition: {post_expr}, \
             optional: {optional}, optional_before_first: {optional_before_first}, \
             per_pass_this_way: {per_pass_this_way} }}",
            pass_chain.effects.join(", ")
        ));
        let trailing = sub.map(str::to_string);
        if let Some(ref cname) = trailing
            && self.sub_is_api(Some(cname), "Cleanup")
        {
            let mut next_after = None;
            if self
                .consume_optional_remember_cleanup(cname.clone(), &mut next_after)
                .is_some()
            {
                return self.chain_sub_ability(next_after, chain);
            }
        }
        self.chain_sub_ability(trailing, chain)
    }

    // -------------------------------------------------------------------------
    // Wave 21b: Effect::Play, Effect::Vote / VoteObjects, Effect::ReduceNextSpellCost,
    // and Effect::SetPTByCountFilter.
    // -------------------------------------------------------------------------

    fn strip_exiled_with_source_filter(valid: &str) -> Option<String> {
        let mut out = Vec::new();
        for alt in valid.split(',').map(str::trim) {
            if alt.is_empty() {
                return None;
            }
            let (base, atoms_raw) = alt.split_once('.').unwrap_or((alt, ""));
            let mut linked = false;
            let mut kept = Vec::new();
            for atom in atoms_raw
                .split('+')
                .map(str::trim)
                .filter(|a| !a.is_empty())
            {
                match atom {
                    "ExiledWithSource" | "ExiledWith" | "IsImprinted" => linked = true,
                    "IsRemembered" => return None,
                    other => kept.push(other),
                }
            }
            if !linked {
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

    fn parse_play_sa_filter(&self, raw_sa: &str) -> Option<(Option<&'static str>, Option<String>)> {
        let trimmed = raw_sa.trim();
        if trimmed == "Spell" {
            return Some((None, None));
        }
        let mut normalized_alts = Vec::new();
        for alt in trimmed.split(',').map(str::trim) {
            if alt.is_empty() || alt == "Land" || alt.starts_with("Land.") {
                return None;
            }
            let norm = if alt == "Spell" {
                "Card".to_string()
            } else if let Some(rest) = alt.strip_prefix("Spell.") {
                format!("Card.{rest}")
            } else {
                alt.to_string()
            };
            normalized_alts.push(norm);
        }
        let joined = normalized_alts.join(",");
        let (owner, cleaned) = Self::strip_graveyard_owner(&joined)?;
        let expr = if cleaned == "Card" {
            None
        } else {
            Some(self.filter_expr_in_zone(&cleaned, true)?)
        };
        Some((owner, expr))
    }

    fn parse_play_cost_mode(&self, p: &mut Params) -> Option<String> {
        let free = match p.take("WithoutManaCost").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                return self.deny(format!("`Play` with `WithoutManaCost$ {other}`"));
            }
        };
        let play_cost = p.take("PlayCost");
        let mana_conv = p.take("ManaConversion");
        match (free, play_cost, mana_conv) {
            (true, None, None) => Some("PlayCostMode::WithoutPayingManaCost".to_string()),
            (false, None, None) => Some("PlayCostMode::Normal".to_string()),
            (false, None, Some(conv)) if conv == "AnyType->AnyType" => {
                Some("PlayCostMode::AnyManaColor".to_string())
            }
            (false, Some(raw), None) => {
                let mana = super::printed_mana(&raw)?;
                Some(format!("PlayCostMode::Alternative(mana!(\"{mana}\"))"))
            }
            _ => self.deny("`Play` with unsupported cost combination".to_string()),
        }
    }

    fn combine_play_filters(
        &mut self,
        base: Option<String>,
        extra: Option<String>,
    ) -> Option<String> {
        let combined = match (base, extra) {
            (None, None) => "Filter::Any".to_string(),
            (Some(f), None) | (None, Some(f)) => f,
            (Some(a), Some(b)) if a == "Filter::Any" => b,
            (Some(a), Some(b)) if b == "Filter::Any" => a,
            (Some(a), Some(b)) => format!("Filter::And(&[{a}, {b}])"),
        };
        if !self.has_x
            && (combined.contains("Filter::CmcAtMostX") || combined.contains("Filter::CmcExactlyX"))
        {
            return self.deny("`Play` filter using `X` without announced `X`".to_string());
        }
        if combined == "Filter::Any" {
            Some("&Filter::Any".to_string())
        } else {
            let name = self.body.filter_static("PLAY", &combined);
            Some(format!("&{name}"))
        }
    }

    fn parse_play_defined_source(
        &self,
        def: &str,
        valid_zone: Option<&str>,
        target: Option<&str>,
    ) -> Option<String> {
        match def {
            "ExiledWith" if matches!(valid_zone, None | Some("Exile")) => {
                Some("PlaySource::LinkedExile".to_string())
            }
            "Imprinted" if !self.on_a_spell && matches!(valid_zone, None | Some("Exile")) => {
                Some("PlaySource::LinkedExile".to_string())
            }
            "Self"
                if !self.on_a_spell && matches!(valid_zone, None | Some("Exile" | "Graveyard")) =>
            {
                Some("PlaySource::Subject(TargetSpec::ThisObject)".to_string())
            }
            "Targeted" => {
                let tgt = target?;
                if !tgt.starts_with("TargetSpec::CardInGraveyard")
                    && !tgt.starts_with("TargetSpec::Object(")
                {
                    return None;
                }
                Some(format!("PlaySource::Subject({tgt})"))
            }
            "TopOfLibrary" if matches!(valid_zone, None | Some("Library")) => {
                Some("PlaySource::Zone { zone: ZoneRef::Library, owner: PlayerRel::You }".into())
            }
            other
                if self.is_event_object_defined(other)
                    && matches!(valid_zone, None | Some("Exile" | "Graveyard" | "Hand")) =>
            {
                Some("PlaySource::Subject(TargetSpec::EventObject)".to_string())
            }
            _ => None,
        }
    }

    fn parse_play_source_and_filter(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        sa_owner: Option<&'static str>,
        sa_filter: Option<String>,
    ) -> Option<(String, String)> {
        p.claim_label("DefinedDesc", "Defined");
        let defined = p.take("Defined");
        let valid = p.take("Valid");
        let valid_zone = p.take("ValidZone");
        if targets_here {
            if defined.is_some() || valid.is_some() || valid_zone.is_some() || sa_owner.is_some() {
                return None;
            }
            let tgt = target?;
            if !tgt.starts_with("TargetSpec::CardInGraveyard")
                && !tgt.starts_with("TargetSpec::Object(")
            {
                return None;
            }
            let filter_ref = self.combine_play_filters(None, sa_filter)?;
            return Some((format!("PlaySource::Subject({tgt})"), filter_ref));
        }
        if let Some(def) = defined.as_deref() {
            if valid.is_some() || sa_owner.is_some() {
                return None;
            }
            let source = self.parse_play_defined_source(def, valid_zone.as_deref(), target)?;
            let filter_ref = self.combine_play_filters(None, sa_filter)?;
            return Some((source, filter_ref));
        }
        let valid_raw = valid?;
        let zone_raw = valid_zone.unwrap_or_else(|| "Hand".to_string());
        if zone_raw == "Exile" && valid_raw == "Card.Self" && !self.on_a_spell && sa_owner.is_none()
        {
            let filter_ref = self.combine_play_filters(None, sa_filter)?;
            return Some((
                "PlaySource::Subject(TargetSpec::ThisObject)".to_string(),
                filter_ref,
            ));
        }
        if zone_raw == "Exile"
            && sa_owner.is_none()
            && let Some(stripped) = Self::strip_exiled_with_source_filter(&valid_raw)
        {
            let base = if stripped == "Card" {
                None
            } else {
                Some(self.filter_expr_in_zone(&stripped, true)?)
            };
            let filter_ref = self.combine_play_filters(base, sa_filter)?;
            return Some(("PlaySource::LinkedExile".to_string(), filter_ref));
        }
        if valid_raw.contains("IsRemembered")
            || valid_raw.contains("IsImprinted")
            || valid_raw.contains("ExiledWith")
            || valid_raw.contains("ChosenCard")
        {
            return None;
        }
        let zone_ref = match zone_raw.as_str() {
            "Hand" => "ZoneRef::Hand",
            "Graveyard" => "ZoneRef::Graveyard",
            "Exile" => "ZoneRef::Exile",
            "Command" => "ZoneRef::Command",
            "Library" if self.oracle.to_ascii_lowercase().contains("top card") => {
                "ZoneRef::Library"
            }
            _ => return None,
        };
        let (valid_owner, cleaned) = Self::strip_graveyard_owner(&valid_raw)?;
        let owner = match (valid_owner, sa_owner) {
            (Some(a), Some(b)) if a != b => return None,
            (Some(a), _) | (_, Some(a)) => a,
            (None, None) if zone_ref == "ZoneRef::Hand" => return None,
            (None, None) => "PlayerRel::EachPlayer",
        };
        if owner == "PlayerRel::Chosen" && target != Some("TargetSpec::Player(PlayerRel::Chosen)") {
            return None;
        }
        let base = if cleaned == "Card" {
            None
        } else {
            Some(self.filter_expr_in_zone(&cleaned, true)?)
        };
        let filter_ref = self.combine_play_filters(base, sa_filter)?;
        Some((
            format!("PlaySource::Zone {{ zone: {zone_ref}, owner: {owner} }}"),
            filter_ref,
        ))
    }

    /// `AB$/SP$/DB$ Play` -> `Effect::Play` (`CR 608.2g`, `CR 707.12`),
    /// preserving `Effect::MayCopyCastLinkedExile` for the legacy Isochron
    /// Scepter shape.
    pub(super) fn play_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_here: bool,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if target.is_none()
            && !targets_here
            && p.peek("Valid") == Some("Card.IsImprinted+ExiledWithSource")
            && p.peek("ValidZone") == Some("Exile")
            && p.peek("Amount") == Some("All")
            && p.peek("WithoutManaCost") == Some("True")
            && p.peek("ValidSA") == Some("Spell")
            && p.peek("Optional") == Some("True")
            && p.peek("CopyCard") == Some("True")
            && p.entries.len() == 7
        {
            for key in [
                "Valid",
                "ValidZone",
                "Amount",
                "WithoutManaCost",
                "ValidSA",
                "Optional",
                "CopyCard",
            ] {
                p.take(key);
            }
            return Some(vec!["Effect::MayCopyCastLinkedExile".to_string()]);
        }
        let cost = self.parse_play_cost_mode(p)?;
        let copy = match p.take("CopyCard").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Play` with `CopyCard$ {other}`")),
        };
        let exile_instead_of_graveyard = match p.take("ReplaceGraveyard").as_deref() {
            None => false,
            Some("Exile") => true,
            Some(other) => {
                return self.deny(format!("`Play` with `ReplaceGraveyard$ {other}`"));
            }
        };
        let optional = match p.take("Optional").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Play` with `Optional$ {other}`")),
        };
        let count = match p.take("Amount").as_deref().map(str::trim) {
            None | Some("1") => "PlayCount::One".to_string(),
            Some("All") => "PlayCount::All".to_string(),
            Some(raw) => {
                let n: u8 = self.plain_num(raw)?.try_into().ok().filter(|&n| n >= 2)?;
                format!("PlayCount::UpTo({n})")
            }
        };
        let controller =
            self.wave11_player_of_line(p.take("Controller").as_deref(), target, targets_a_player)?;
        if !Self::is_single_player_rel(controller) {
            return self.deny(format!("`Play` with non-single controller `{controller}`"));
        }
        let (cast_only, sa_owner, sa_filter) = match p.take("ValidSA") {
            None => (copy || exile_instead_of_graveyard, None, None),
            Some(raw_sa) => {
                let (owner, filter) = self.parse_play_sa_filter(&raw_sa)?;
                (true, owner, filter)
            }
        };
        let (source, filter) =
            self.parse_play_source_and_filter(p, target, targets_here, sa_owner, sa_filter)?;
        Some(vec![format!(
            "Effect::Play {{ source: {source}, filter: {filter}, controller: {controller}, \
             count: {count}, cost: {cost}, cast_only: {cast_only}, copy: {copy}, \
             exile_instead_of_graveyard: {exile_instead_of_graveyard}, optional: {optional} }}"
        )])
    }

    /// `AB$/SP$/DB$ Vote` -> `Effect::Vote` / `Effect::VoteObjects` (`CR 701.38`).
    pub(super) fn vote_chain(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
        chain: &mut Chain,
    ) -> Option<()> {
        if chain
            .target
            .as_deref()
            .is_some_and(|t| t.starts_with("TargetSpec::Spell"))
        {
            p.take("TargetType");
        }
        if p.take("Defined").as_deref() != Some("Player") {
            return self.deny("`Vote` without `Defined$ Player`".to_string());
        }
        let secret = match p.take("Secretly").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Vote` with `Secretly$ {other}`")),
        };
        let effect = if p.has("VoteCard") {
            self.vote_objects_effect(&mut p, secret)?
        } else if p.has("Choices") {
            self.vote_options_effect(&mut p, secret, chain)?
        } else {
            return self.deny("`Vote` with neither `Choices$` nor `VoteCard$`".to_string());
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Vote.{key}`"));
        }
        chain.effects.push(effect);
        self.chain_sub_ability(sub.map(str::to_string), chain)
    }

    fn vote_objects_if_no_votes(&mut self, mut cursor: Option<String>) -> Option<Vec<String>> {
        let mut if_no_votes = Vec::new();
        while let Some(cname) = cursor.take() {
            let body = self.svars.get(cname.trim())?.clone();
            let (c_api, mut cp) = Params::parse(&body)?;
            cp.drop_prose();
            if c_api == "Cleanup" && cp.take("ClearRemembered").as_deref() == Some("True") {
                if !cp.exhausted() {
                    return None;
                }
                break;
            }
            let zero_cond = if cp.peek("ConditionDefined") == Some("Remembered")
                && matches!(cp.peek("ConditionPresent"), Some("Card" | "Permanent"))
                && matches!(cp.peek("ConditionCompare"), Some("EQ0" | "LT1" | "LE0"))
            {
                cp.take("ConditionDefined");
                cp.take("ConditionPresent");
                cp.take("ConditionCompare");
                cp.take("ConditionDescription");
                true
            } else if let Some(var) = cp.peek("ConditionCheckSVar")
                && self.svars.get(var.trim()).is_some_and(|d| {
                    matches!(d.trim(), "Remembered$Amount" | "Count$RememberedSize")
                })
                && matches!(cp.peek("ConditionSVarCompare"), Some("EQ0" | "LT1" | "LE0"))
            {
                cp.take("ConditionCheckSVar");
                cp.take("ConditionSVarCompare");
                cp.take("ConditionDescription");
                true
            } else {
                false
            };
            if !zero_cond || cp.has("ValidTgts") || cp.has("UnlessCost") || cp.has("AtEOT") {
                return None;
            }
            let next = cp.take("SubAbility");
            let effs = self.effect_of_followup(&c_api, &mut cp, None)?;
            if !cp.exhausted() || effs.is_empty() {
                return None;
            }
            if_no_votes.extend(effs);
            cursor = next;
        }
        Some(if_no_votes)
    }

    fn vote_objects_effect(&mut self, p: &mut Params, secret: bool) -> Option<String> {
        p.claim_label("VoteMessage", "VoteCard");
        let valid = p.take("VoteCard")?;
        let optional = match p.take("UpTo").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Vote` with `UpTo$ {other}`")),
        };
        let zone = match p.take("Zone").as_deref() {
            None | Some("Battlefield") => "ZoneRef::Battlefield",
            Some("Graveyard") => "ZoneRef::Graveyard",
            Some(other) => return self.deny(format!("`Vote` in `Zone$ {other}`")),
        };
        let sub_name = p.take("VoteSubAbility")?;
        let sub_body = self.svars.get(sub_name.trim())?.clone();
        let (s_api, mut sp) = Params::parse(&sub_body)?;
        sp.drop_prose();
        let next_sub = sp.take("SubAbility");
        let action = match s_api.as_str() {
            "ChangeZone"
                if sp.take("Defined").as_deref() == Some("Remembered")
                    && ((zone == "ZoneRef::Battlefield"
                        && sp.take("Origin").as_deref() == Some("Battlefield"))
                        || (zone == "ZoneRef::Graveyard"
                            && sp.take("Origin").as_deref() == Some("Graveyard"))) =>
            {
                match sp.take("Destination").as_deref() {
                    Some("Exile") => "VoteObjectAction::Exile".to_string(),
                    Some("Hand") => "VoteObjectAction::ReturnToHand".to_string(),
                    _ => return self.deny("`Vote` object move to unsupported destination".into()),
                }
            }
            "DestroyAll" if zone == "ZoneRef::Battlefield" => {
                let vc = sp.take("ValidCards")?;
                let vc_ok = vc == "Card.IsRemembered"
                    || vc == "Permanent.IsRemembered"
                    || vc == format!("{valid}.IsRemembered")
                    || vc == format!("{valid}+IsRemembered");
                if !vc_ok {
                    return None;
                }
                let no_regen = match sp.take("NoRegen").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                format!("VoteObjectAction::Destroy {{ no_regen: {no_regen} }}")
            }
            "Destroy" if zone == "ZoneRef::Battlefield" => {
                if sp.take("Defined").as_deref() != Some("Remembered") {
                    return None;
                }
                let no_regen = match sp.take("NoRegen").as_deref() {
                    None | Some("False") => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                format!("VoteObjectAction::Destroy {{ no_regen: {no_regen} }}")
            }
            _ => return self.deny(format!("`Vote` object action `{s_api}`")),
        };
        if !sp.exhausted() {
            return None;
        }
        let if_no_votes = self.vote_objects_if_no_votes(next_sub)?;
        let expr = self.filter_expr_in_zone(&valid, zone == "ZoneRef::Graveyard")?;
        let filter_ref = if expr == "Filter::Any" {
            "&Filter::Any".to_string()
        } else {
            format!("&{}", self.body.filter_static("VOTE", &expr))
        };
        Some(format!(
            "Effect::VoteObjects {{ zone: {zone}, filter: {filter_ref}, optional: {optional}, \
             secret: {secret}, action: {action}, if_no_votes: &[{}] }}",
            if_no_votes.join(", ")
        ))
    }

    fn vote_count_amount(&self, raw: &str, option_idx: u8) -> Option<(String, bool)> {
        let trimmed = raw.trim();
        let neg = trimmed.starts_with('-');
        let key = trimmed.trim_start_matches(['+', '-']);
        let def = match self.svars.get(key) {
            Some(d) => d.trim(),
            None => key,
        };
        let (head, suffix) = def.split_once('/').map_or((def, ""), |(h, s)| (h, s));
        if !matches!(head, "VoteNum" | "Count$VoteNum") {
            return None;
        }
        let base = format!("Amount::ThisWay(ThisWay::VoteCount({option_idx}))");
        let mag = Self::wrap_amount_suffix(base, suffix)?;
        Some((mag, neg))
    }

    #[allow(clippy::too_many_lines)]
    fn try_vote_counted_step(
        &mut self,
        api: &str,
        sp: &mut Params,
        option_idx: u8,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if sp.has("ValidTgts")
            || sp.has("UnlessCost")
            || sp.has("AtEOT")
            || sp.entries.iter().any(|(k, _)| k.starts_with("Condition"))
        {
            return None;
        }
        let eff = match api {
            "PutCounter" => {
                sp.take("IsCurse");
                let raw = sp.take("CounterNum")?;
                let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
                    return None;
                };
                let counter = super::counter_kind(&sp.take("CounterType")?)?;
                let on_self = match sp.take("Defined").as_deref() {
                    None | Some("Self") if !self.on_a_spell => true,
                    Some("Targeted")
                        if target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
                    {
                        false
                    }
                    _ => return None,
                };
                if !sp.exhausted() {
                    return None;
                }
                if on_self {
                    format!(
                        "Effect::AddCounterFilter {{ filter: &Filter::This, kind: {counter}, amount: {amt} }}"
                    )
                } else {
                    format!("Effect::AddCounter {{ kind: {counter}, amount: {amt} }}")
                }
            }
            "Token" => {
                let raw = sp.take("TokenAmount")?;
                let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
                    return None;
                };
                let owner = match sp.take("TokenOwner").as_deref() {
                    None | Some("You") => "PlayerRel::You",
                    other => self.wave11_player_of_line(other, target, false)?,
                };
                let tapped = match sp.take("TokenTapped").as_deref() {
                    None => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let stem = sp.take("TokenScript")?;
                if !sp.exhausted() || (owner != "PlayerRel::You" && tapped) {
                    return None;
                }
                let body = self.tokens?.body(&stem, self.cats)?;
                let token = format!("&generated_tokens::{}", body.constant);
                if owner != "PlayerRel::You" {
                    format!(
                        "Effect::CreateTokensFor {{ player: {owner}, token: {token}, amount: {amt} }}"
                    )
                } else if tapped {
                    format!(
                        "Effect::CreateTokenModified {{ token: {token}, amount: {amt}, tapped: true, haste: false }}"
                    )
                } else {
                    format!("Effect::CreateTokenN {{ token: {token}, amount: {amt} }}")
                }
            }
            "Draw" => {
                let raw = sp.take("NumCards")?;
                let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
                    return None;
                };
                let who =
                    self.wave11_player_of_line(sp.take("Defined").as_deref(), target, false)?;
                if !sp.exhausted() {
                    return None;
                }
                if who == "PlayerRel::You" {
                    format!("Effect::DrawCards {{ amount: {amt} }}")
                } else {
                    format!("Effect::DrawCardsFor {{ amount: {amt}, who: {who} }}")
                }
            }
            "GainLife" => {
                let raw = sp.take("LifeAmount")?;
                let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
                    return None;
                };
                let who =
                    self.wave11_player_of_line(sp.take("Defined").as_deref(), target, false)?;
                if !sp.exhausted() {
                    return None;
                }
                if who == "PlayerRel::You" {
                    format!("Effect::GainLife {{ amount: {amt} }}")
                } else {
                    format!("Effect::GainLifeFor {{ amount: {amt}, who: {who} }}")
                }
            }
            "LoseLife" => {
                sp.take("IsCurse");
                let raw = sp.take("LifeAmount")?;
                let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
                    return None;
                };
                let who =
                    self.wave11_player_of_line(sp.take("Defined").as_deref(), target, false)?;
                if !sp.exhausted() {
                    return None;
                }
                format!("Effect::LoseLife {{ amount: {amt}, target: {who} }}")
            }
            "Mill" => {
                sp.take("IsCurse");
                let raw = sp.take("NumCards")?;
                let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
                    return None;
                };
                let who =
                    self.wave11_player_of_line(sp.take("Defined").as_deref(), target, false)?;
                if !sp.exhausted() {
                    return None;
                }
                format!("Effect::Mill {{ amount: {amt}, target: {who} }}")
            }
            "Scry" | "Surveil" => {
                let key = if api == "Scry" { "ScryNum" } else { "Amount" };
                let raw = sp.take(key)?;
                let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
                    return None;
                };
                if !matches!(sp.take("Defined").as_deref(), None | Some("You")) || !sp.exhausted() {
                    return None;
                }
                if api == "Scry" {
                    format!("Effect::Scry {{ amount: {amt} }}")
                } else {
                    format!("Effect::Surveil {{ amount: {amt} }}")
                }
            }
            "Sacrifice" => {
                sp.take("IsCurse");
                sp.claim_label("SacMessage", "SacValid");
                let raw = sp.take("Amount")?;
                let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
                    return None;
                };
                let who =
                    self.wave11_player_of_line(sp.take("Defined").as_deref(), target, false)?;
                let valid = sp.take("SacValid")?;
                if !sp.exhausted() {
                    return None;
                }
                let expr = self.filter_expr(&valid)?;
                let filter = if expr == "Filter::Any" {
                    "&Filter::Any".to_string()
                } else {
                    format!("&{}", self.body.filter_static("VOTE_SAC", &expr))
                };
                format!(
                    "Effect::SacrificeAmountForPlayers {{ who: {who}, filter: {filter}, amount: {amt} }}"
                )
            }
            "Discard" => {
                sp.take("IsCurse");
                let raw = sp.take("NumCards")?;
                let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
                    return None;
                };
                if sp.take("Mode").as_deref() != Some("TgtChoose") {
                    return None;
                }
                let who =
                    self.wave11_player_of_line(sp.take("Defined").as_deref(), target, false)?;
                if !sp.exhausted() {
                    return None;
                }
                format!("Effect::DiscardAmountForPlayers {{ who: {who}, amount: {amt} }}")
            }
            "Repeat" => {
                let raw = sp.take("MaxRepeat")?;
                let (amt, false) = self.vote_count_amount(&raw, option_idx)? else {
                    return None;
                };
                let rep_sub = sp.take("RepeatSubAbility")?;
                if !sp.exhausted() {
                    return None;
                }
                let rep_body = self.svars.get(rep_sub.trim())?.clone();
                let mut inner = Chain {
                    target: target.map(str::to_string),
                    ..Chain::default()
                };
                self.chain(&rep_body, &mut inner)?;
                if inner.effects.is_empty()
                    || inner.target.as_deref() != target
                    || inner.second_target.is_some()
                {
                    return None;
                }
                format!(
                    "Effect::RepeatProcess {{ effects: &[{}], max_times: Some({amt}), \
                     pre_condition: None, post_condition: None, optional: false, \
                     optional_before_first: false, per_pass_this_way: false }}",
                    inner.effects.join(", ")
                )
            }
            _ => return None,
        };
        Some(vec![eff])
    }

    fn compile_vote_option_effects(
        &mut self,
        option_idx: u8,
        svar_name: &str,
        per_option_count: bool,
        repeat_per_vote: bool,
        chain: &mut Chain,
    ) -> Option<(String, Vec<String>)> {
        let body = self.svars.get(svar_name.trim())?.clone();
        let (api, mut sp) = Params::parse(&body)?;
        let label = sp.take("SpellDescription")?.trim().to_string();
        if label.is_empty() {
            return None;
        }
        sp.drop_prose();
        if per_option_count {
            let next_sub = sp.take("SubAbility");
            let saved_sp = sp.clone();
            if let Some(mut effs) =
                self.try_vote_counted_step(&api, &mut sp, option_idx, chain.target.as_deref())
            {
                if let Some(tail) = next_sub {
                    let mut tail_chain = Chain {
                        target: chain.target.clone(),
                        ..Chain::default()
                    };
                    self.chain_sub_ability(Some(tail), &mut tail_chain)?;
                    if tail_chain.target != chain.target || tail_chain.second_target.is_some() {
                        return None;
                    }
                    chain.could_add_mana |= tail_chain.could_add_mana;
                    chain.moves_library |= tail_chain.moves_library;
                    effs.extend(tail_chain.effects);
                }
                return Some((label, effs));
            }
            sp = saved_sp;
            if let Some(tail) = next_sub {
                sp.entries.push(("SubAbility".to_string(), tail));
            }
        }
        let mut uses_voter = false;
        if repeat_per_vote {
            for (k, v) in &mut sp.entries {
                if matches!(k.as_str(), "Defined" | "TokenOwner" | "UnlessPayer")
                    && v == "RememberedPlayer"
                {
                    *v = "TriggeredPlayer".to_string();
                    uses_voter = true;
                }
            }
        }
        let spec = Self::reconstruct_spec(&api, &sp);
        let mut opt_chain = Chain {
            target: chain.target.clone(),
            target_up_to: chain.target_up_to,
            target_exact: chain.target_exact,
            target_is_x: chain.target_is_x,
            ..Chain::default()
        };
        let saved_mode = if uses_voter {
            self.trigger_mode.replace("LifeGained".into())
        } else {
            self.trigger_mode.clone()
        };
        let ok = self.chain(&spec, &mut opt_chain);
        self.trigger_mode = saved_mode;
        ok?;
        if opt_chain.effects.is_empty()
            || opt_chain.target != chain.target
            || opt_chain.second_target.is_some()
        {
            return None;
        }
        chain.could_add_mana |= opt_chain.could_add_mana;
        chain.moves_library |= opt_chain.moves_library;
        Some((label, opt_chain.effects))
    }

    fn vote_options_effect(
        &mut self,
        p: &mut Params,
        secret: bool,
        chain: &mut Chain,
    ) -> Option<String> {
        let choices_raw = p.take("Choices")?;
        let choice_names: Vec<&str> = choices_raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if choice_names.len() < 2 || choice_names.len() > usize::from(u8::MAX) {
            return None;
        }
        let tied_ability = p.take("VoteTiedAbility");
        let store_vote_num = match p.take("StoreVoteNum").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let each_vote = match p.take("EachVote").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let outcome = match (tied_ability.as_deref(), store_vote_num, each_vote) {
            (Some(tied), false, false) => {
                let idx = choice_names.iter().position(|&c| c == tied.trim())?;
                format!("VoteOutcome::MajorityOrTied {{ tied_option: {idx} }}")
            }
            (None, false, false) => "VoteOutcome::AllWinners".to_string(),
            (None, true, false) => "VoteOutcome::PerOptionCount".to_string(),
            (None, false, true) => "VoteOutcome::RepeatPerVote".to_string(),
            _ => return self.deny("`Vote` with conflicting outcome flags".to_string()),
        };
        let mut rendered = Vec::with_capacity(choice_names.len());
        for (idx, name) in choice_names.iter().enumerate() {
            let idx_u8 = u8::try_from(idx).ok()?;
            let (label, effs) =
                self.compile_vote_option_effects(idx_u8, name, store_vote_num, each_vote, chain)?;
            let escaped = label.replace('\\', "\\\\").replace('"', "\\\"");
            rendered.push(format!(
                "VoteOption {{ name: \"{escaped}\", effects: &[{}] }}",
                effs.join(", ")
            ));
        }
        Some(format!(
            "Effect::Vote {{ options: &[{}], outcome: {outcome}, secret: {secret} }}",
            rendered.join(", ")
        ))
    }

    fn forget_on_cast_matches_valid(forget: &str, valid_card: &str, who: &str) -> bool {
        let suffix = match who {
            "PlayerRel::You" => "YouCtrl",
            "PlayerRel::EachOpponent" => "OppCtrl",
            _ => return false,
        };
        let strip_one = |alt: &str| -> Option<String> {
            let alt = alt.trim();
            alt.strip_suffix(&format!("+{suffix}"))
                .or_else(|| alt.strip_suffix(&format!(".{suffix}")))
                .map(str::to_string)
        };
        let Some(stripped): Option<Vec<String>> = forget.split(',').map(strip_one).collect() else {
            return false;
        };
        let expected: Vec<String> = valid_card
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        stripped == expected
    }

    /// `AB$/SP$/DB$ Effect | StaticAbilities$ <ReduceCost>` lowering to
    /// `Effect::ReduceNextSpellCost` (`Kaza`, `Elminster`, `Will, Scion of Peace`,
    /// `Rowan, Scion of War`, `Will Kenrith`, `Draconic Debut`, colored reductions).
    /// Plain fixed-amount non-one-shot discounts remain on `wave9_effect_spell_cost`
    /// (`Modifier::SpellsCostLess`), and fixed-amount `ForgetOnCast$ Card.YouCtrl`
    /// without `SetChosenNumber$` stays deferred cleanly per Wave 21b scope.
    #[allow(clippy::too_many_lines)]
    fn try_reduce_next_spell_cost_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let statics_raw = p.take("StaticAbilities")?;
        if statics_raw.contains(',') {
            return None;
        }
        let body = self.svars.get(statics_raw.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "ReduceCost"
            || st.take("Type").as_deref() != Some("Spell")
            || !matches!(st.take("EffectZone").as_deref(), None | Some("Battlefield"))
        {
            return None;
        }
        let duration = match p.take("Duration").as_deref() {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some(_) => return None,
        };
        let forget_on_cast = p.take("ForgetOnCast");
        let set_chosen = p.take("SetChosenNumber");
        let remember_objs = p.take("RememberObjects");
        p.take("Name");
        if !p.exhausted() {
            return None;
        }
        let who = match (st.take("Activator").as_deref(), remember_objs.as_deref()) {
            (Some("You"), None) => "PlayerRel::You",
            (Some("Opponent" | "Player.Opponent"), None) => "PlayerRel::EachOpponent",
            (None | Some("Player"), None) => "PlayerRel::EachPlayer",
            (Some("Player.IsRemembered"), Some("Targeted"))
                if target == Some("TargetSpec::Player(PlayerRel::Chosen)") =>
            {
                "PlayerRel::Chosen"
            }
            (Some("Player.IsRemembered"), Some("TriggeredActivator" | "TriggeredPlayer"))
                if self.trigger_mode.is_some() =>
            {
                "PlayerRel::EventPlayer"
            }
            _ => return None,
        };
        let raw_valid = st.take("ValidCard").unwrap_or_else(|| "Card".to_string());
        if raw_valid.contains("IsRemembered")
            || raw_valid.contains("EffectSource")
            || raw_valid.contains("RememberedPlayer")
            || raw_valid.contains("sharesCreatureTypeWith")
            || raw_valid.contains("faceDown")
        {
            return None;
        }
        let next_only = match forget_on_cast.as_deref() {
            None => false,
            Some(f) => {
                if !Self::forget_on_cast_matches_valid(f, &raw_valid, who) {
                    return None;
                }
                true
            }
        };
        let colored_only = match st.take("ColorOnly").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(_) => return None,
        };
        let raw_amount = st.take("Amount");
        let raw_cost = st.take("Cost");
        if !st.exhausted() {
            return None;
        }
        let (generic, mana, is_plain_fixed_generic) =
            match (raw_amount.as_deref(), raw_cost.as_deref()) {
                (Some(amt_raw), None) if !colored_only => {
                    let resolved_chosen = amt_raw.trim() == "Count$ChosenNumber"
                        || self
                            .svars
                            .get(amt_raw.trim())
                            .is_some_and(|d| d.trim() == "Count$ChosenNumber");
                    if resolved_chosen {
                        let chosen_var = set_chosen.as_deref()?;
                        let amt = self.amount_or_count_ext(chosen_var)?;
                        if amt == "Amount::Fixed(0)" {
                            return None;
                        }
                        (amt, "ManaCost::ZERO".to_string(), false)
                    } else {
                        if set_chosen.is_some() {
                            return None;
                        }
                        if let Some(n) = self.plain_num(amt_raw).filter(|&n| n > 0) {
                            (
                                format!("Amount::Fixed({n})"),
                                "ManaCost::ZERO".to_string(),
                                true,
                            )
                        } else {
                            let amt = self.amount_or_count_ext(amt_raw)?;
                            if amt == "Amount::Fixed(0)" {
                                return None;
                            }
                            (amt, "ManaCost::ZERO".to_string(), false)
                        }
                    }
                }
                (None, Some(cost_raw)) => {
                    if set_chosen.is_some() {
                        return None;
                    }
                    let printed = super::printed_mana(cost_raw)?;
                    if let Some(n) = super::generic_mana(&printed) {
                        if colored_only || n == 0 {
                            return None;
                        }
                        (
                            format!("Amount::Fixed({n})"),
                            "ManaCost::ZERO".to_string(),
                            true,
                        )
                    } else {
                        (
                            "Amount::Fixed(0)".to_string(),
                            format!("mana!(\"{printed}\")"),
                            false,
                        )
                    }
                }
                _ => return None,
            };
        // Keep plain fixed-amount multi-spell discounts on `wave9_effect_spell_cost`
        // (`Modifier::SpellsCostLess`).
        if !next_only && is_plain_fixed_generic && remember_objs.is_none() {
            return None;
        }
        let filter_ref = if raw_valid == "Card" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr_in_zone(&raw_valid, true)?;
            format!("&{}", self.body.filter_static("REDUCE", &expr))
        };
        Some(vec![format!(
            "Effect::ReduceNextSpellCost {{ who: {who}, filter: {filter_ref}, generic: {generic}, \
             mana: {mana}, colored_only: {colored_only}, next_only: {next_only}, duration: {duration} }}"
        )])
    }

    /// One-shot `Effect | StaticAbilities$ ... | ForgetOnCast$ ...` spells/abilities
    /// backed directly by dedicated DSL primitives (`Effect::NextSpellThisTurnHasImprovise`
    /// for `Archway of Innovation`, `Modifier::OneSpellManaAnyType` for `North Star`).
    fn try_one_shot_spell_effect(
        &self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some() {
            return None;
        }
        let statics_raw = p.take("StaticAbilities")?;
        if statics_raw.contains(',') {
            return None;
        }
        if !matches!(p.take("Duration").as_deref(), None | Some("UntilEndOfTurn")) {
            return None;
        }
        let forget = p.take("ForgetOnCast")?;
        p.take("Name");
        if !p.exhausted() {
            return None;
        }
        let body = self.svars.get(statics_raw.trim())?;
        let (mode, mut st) = Params::parse(body)?;
        st.drop_prose();
        if !matches!(st.take("EffectZone").as_deref(), None | Some("Command")) {
            return None;
        }
        if mode == "Continuous"
            && forget == "Card.YouCtrl"
            && st.take("Affected").as_deref() == Some("Card.YouCtrl")
            && st.take("AffectedZone").as_deref() == Some("Stack")
            && st.take("AddKeyword").as_deref() == Some("Improvise")
            && st.exhausted()
        {
            return Some(vec!["Effect::NextSpellThisTurnHasImprovise".to_string()]);
        }
        if mode == "ManaConvert"
            && matches!(forget.as_str(), "Card.IsRemembered" | "Card.YouCtrl")
            && st.take("ValidPlayer").as_deref() == Some("You")
            && st.take("ValidSA").as_deref() == Some("Spell")
            && matches!(st.take("Optional").as_deref(), None | Some("True"))
            && st.take("ManaConversion").as_deref() == Some("AnyType->AnyType")
            && st.exhausted()
        {
            return Some(vec![
                "Effect::continuous(&Filter::Any, Modifier::OneSpellManaAnyType, Duration::UntilEndOfTurn)"
                    .to_string(),
            ]);
        }
        None
    }

    fn parse_pt_count_expr(&mut self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if let Some(n) = self.plain_num(trimmed) {
            let n = u16::try_from(n).ok()?;
            return Some(format!("PtCount::Fixed({n})"));
        }
        let def = self
            .svars
            .get(trimmed)
            .map_or(trimmed, String::as_str)
            .trim()
            .to_string();
        let (head, suffix) = def
            .split_once('/')
            .map_or((def.as_str(), ""), |(h, s)| (h, s));
        let base = match head {
            "Count$ValidHand Card.YouOwn" | "Count$ValidHand Card.YouCtrl" => {
                "PtCount::CardsInHand(PlayerRel::You)".to_string()
            }
            "Count$ValidHand Card.OppOwn" | "Count$ValidHand Card.OppCtrl" => {
                "PtCount::CardsInHand(PlayerRel::EachOpponent)".to_string()
            }
            "Count$ValidHand Card" => "PtCount::CardsInHand(PlayerRel::EachPlayer)".to_string(),
            "Count$Domain" => "PtCount::Domain(PlayerRel::You)".to_string(),
            "Count$YourLifeTotal" => "PtCount::LifeTotal(PlayerRel::You)".to_string(),
            "Count$PartySize" => "PtCount::PartySize(PlayerRel::You)".to_string(),
            "Count$CardTypesInAllGraveyards" => "PtCount::CardTypesInAllGraveyards".to_string(),
            "Count$CardTypesInOppGraveyards" => {
                "PtCount::CardTypesInOpponentsGraveyards".to_string()
            }
            "Count$ExiledWithThis" => "PtCount::ExiledWithThis".to_string(),
            _ if head.starts_with("Count$Valid ") => {
                let valid = head.strip_prefix("Count$Valid ")?.trim();
                if let Some(stripped) = valid.strip_suffix(".YouCtrl") {
                    let expr = self.filter_expr(stripped)?;
                    let f = if expr == "Filter::Any" {
                        "&Filter::Any".to_string()
                    } else {
                        format!("&{}", self.body.filter_static("PT_COUNT", &expr))
                    };
                    format!("PtCount::YouControl({f})")
                } else {
                    let expr = self.filter_expr(valid)?;
                    let f = if expr == "Filter::Any" {
                        "&Filter::Any".to_string()
                    } else {
                        format!("&{}", self.body.filter_static("PT_COUNT", &expr))
                    };
                    format!("PtCount::OnBattlefield({f})")
                }
            }
            _ if head.starts_with("Count$ValidGraveyard ") => {
                let valid = head.strip_prefix("Count$ValidGraveyard ")?.trim();
                let (owner, cleaned) = Self::strip_graveyard_owner(valid)?;
                let expr = self.filter_expr_in_zone(&cleaned, true)?;
                let f = if expr == "Filter::Any" {
                    "&Filter::Any".to_string()
                } else {
                    format!("&{}", self.body.filter_static("PT_COUNT", &expr))
                };
                match owner {
                    None => format!("PtCount::InAllGraveyards({f})"),
                    Some(who) => format!("PtCount::InGraveyards {{ who: {who}, filter: {f} }}"),
                }
            }
            _ if head.starts_with("Count$GreatestCardCMC ") => {
                let valid = head.strip_prefix("Count$GreatestCardCMC ")?.trim();
                let expr = self.filter_expr(valid)?;
                let f = if expr == "Filter::Any" {
                    "&Filter::Any".to_string()
                } else {
                    format!("&{}", self.body.filter_static("PT_COUNT", &expr))
                };
                format!("PtCount::GreatestManaValueAmong({f})")
            }
            _ => return None,
        };
        match suffix {
            "" => Some(base),
            "Twice" => Some(format!("PtCount::Twice(&{base})")),
            s if s.starts_with("Plus.") => {
                let offset: u16 = s.strip_prefix("Plus.")?.trim().parse().ok()?;
                Some(format!(
                    "PtCount::Plus {{ base: &{base}, offset: {offset} }}"
                ))
            }
            _ => None,
        }
    }

    /// `Animate` / `AnimateAll` setting base P/T to resolution-evaluated `PtCount`
    /// values (`Effect::SetPTByCountFilter`, `CR 608.2h`, `CR 611.2c`), both
    /// standalone (e.g. `Jolrael, Mwonvuli Recluse`) and composed with type /
    /// keyword animation (e.g. `Elvish Branchbender`, `Vastwood Animist`).
    pub(super) fn set_pt_by_count_filter_effect(
        &mut self,
        is_all: bool,
        p: &mut Params,
        target: Option<&str>,
        single_target: bool,
    ) -> Option<Vec<String>> {
        let power_raw = p.take("Power")?;
        let tough_raw = p.take("Toughness")?;
        if self.plain_num(&power_raw).is_some() && self.plain_num(&tough_raw).is_some() {
            return None;
        }
        let duration = match p.peek("Duration") {
            None | Some("UntilEndOfTurn") => "Duration::UntilEndOfTurn",
            Some("UntilYourNextTurn") => "Duration::UntilYourNextTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some(_) => return None,
        };
        let has_extra_animation = p.entries.iter().any(|(k, _)| {
            !matches!(
                k.as_str(),
                "Duration" | "IsCurse" | "ValidCards" | "Defined"
            )
        });
        let filter_ref = if is_all {
            if target.is_some() {
                return None;
            }
            let valid = p.peek("ValidCards")?.to_string();
            let expr = self.filter_expr(&valid)?;
            if expr == "Filter::Any" {
                "&Filter::Any".to_string()
            } else {
                format!("&{}", self.body.filter_static("PT_FILTER", &expr))
            }
        } else {
            match p.peek("Defined") {
                None | Some("Self") if !self.on_a_spell && target.is_none() => {
                    "&Filter::This".to_string()
                }
                None | Some("Targeted" | "ParentTarget" | "ParentTargeted")
                    if single_target
                        && target.is_some_and(|t| t.starts_with("TargetSpec::Object(")) =>
                {
                    "&Filter::This".to_string()
                }
                Some("Enchanted" | "Equipped") if target.is_none() => {
                    "&Filter::AttachedToBySource".to_string()
                }
                _ => return None,
            }
        };
        let power = self.parse_pt_count_expr(&power_raw)?;
        let toughness = self.parse_pt_count_expr(&tough_raw)?;
        let mut effs = if has_extra_animation {
            if is_all {
                self.animate_all_effect(p, target)?
            } else {
                self.animate_effect(p, target)?
            }
        } else {
            p.take("Duration");
            p.take("IsCurse");
            if is_all {
                p.take("ValidCards");
            } else {
                p.take("Defined");
            }
            Vec::new()
        };
        if !p.exhausted() {
            return None;
        }
        effs.push(format!(
            "Effect::SetPTByCountFilter {{ filter: {filter_ref}, power: {power}, \
             toughness: {toughness}, duration: {duration} }}"
        ));
        Some(effs)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RememberedThisWay {
    Destroyed,
    Milled,
    Discarded,
    Exiled,
    ExiledFromBattlefield,
    ReturnedToHand,
    ExiledFromGraveyard,
    PutIntoLibraryFromHand,
    RevealedTopToHand,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Wave8Scope {
    RepeatPlayer,
    RepeatPermanent,
    ChosenObject,
    RevealedObject,
    Times,
}

struct ExilePermissionSpec {
    window: &'static str,
    cast_only: bool,
    spell_types: Option<String>,
    free: bool,
    mana: &'static str,
}

impl ExilePermissionSpec {
    fn to_expr(&self) -> String {
        let mut expr = if self.cast_only || self.spell_types.is_some() {
            format!("ExilePermission::cast({})", self.window)
        } else {
            format!("ExilePermission::play({})", self.window)
        };
        if let Some(types) = &self.spell_types {
            expr = format!("{expr}.with_spell_types({types})");
        }
        if self.free {
            expr.push_str(".without_paying_mana_cost()");
        }
        match self.mana {
            "ExileMana::AnyColor" => expr.push_str(".spend_mana_as_any_color()"),
            "ExileMana::AnyType" => expr.push_str(".spend_mana_as_any_type()"),
            _ => {}
        }
        expr
    }
}
