//! Reading the effects that are more than one line: tokens, animation,
//! pumps, zone changes and library searches.

use super::{CLUE_TOKEN_SCRIPT, Params, Tx, keyword_const, plain_number, pump_amount};

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
        for key in ["Optional", "RememberSacrificed"] {
            if p.take(key).is_some() {
                return self.deny(format!("a sacrifice naming `{key}`"));
            }
        }
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
            if p.take("Defined").is_some() {
                return self.deny("a sacrifice naming `Defined`".to_string());
            }
            return Some(vec!["Effect::SacrificeSelf".to_string()]);
        };
        let who = self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
        let filter = if sac_valid == "Permanent" {
            "&Filter::Any".to_string()
        } else {
            let expr = self.filter_expr(&sac_valid)?;
            format!("&{}", self.body.filter_static("SAC", &expr))
        };
        match raw_amount.as_deref().map(str::trim) {
            None | Some("1") => Some(vec![format!(
                "Effect::SacrificeFilter {{ who: {who}, filter: {filter} }}"
            )]),
            Some(raw) => {
                let amount = self.amount_or_count(raw)?;
                Some(vec![format!(
                    "Effect::SacrificeAmountForPlayers {{ who: {who}, filter: {filter}, amount: \
                     {amount} }}"
                )])
            }
        }
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
        let owner = match p.take("TokenOwner").as_deref() {
            Some("You") => "PlayerRel::You",
            Some("TriggeredCardController") if self.trigger_is_self => "PlayerRel::You",
            Some("TargetedController")
                if target.is_some_and(|t| {
                    t.starts_with("TargetSpec::Object(")
                        || t.starts_with("TargetSpec::CardInGraveyard(")
                }) && matches!(p.peek("TokenAmount"), None | Some("1")) =>
            {
                p.take("TokenAmount");
                let Some(stem) = p.take("TokenScript") else {
                    return self.deny("a `Token` effect naming no `TokenScript$`".to_string());
                };
                let Some(tokens) = self.tokens else {
                    return self
                        .deny("`Token` with no token scripts to read it against".to_string());
                };
                let Some(body) = tokens.body(&stem, self.cats) else {
                    return self.deny(format!("token script `{stem}`"));
                };
                return Some(vec![format!(
                    "Effect::CreateTokenForTargetController {{ token: &generated_tokens::{} }}",
                    body.constant
                )]);
            }
            None => self.player_rel_of(None, targets_a_player)?,
            Some(who) => return self.deny(format!("token owner `{who}`")),
        };
        if owner != "PlayerRel::You" {
            return self.deny("a token created under another player's control".to_string());
        }
        let Some(stem) = p.take("TokenScript") else {
            return self.deny("a `Token` effect naming no `TokenScript$`".to_string());
        };
        self.create_tokens(&stem, p.take("TokenAmount").as_deref())
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
            Some(raw) => match self.amount_or_count(raw) {
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

    /// `Pump`: `NumAtt$ +2 | NumDef$ +2 | KW$ Trample`, the commonest
    /// effect in the whole script corpus.
    ///
    /// `Defined$ Self` and `Defined$ Targeted` are two different effects
    /// here, not one with a flag: `PumpFilter` binds `Filter::This` to the
    /// source, `PumpTarget` to what the spell targeted, and an ability can
    /// have both a target and a pump on itself.
    pub(super) fn pump_effect(
        &mut self,
        p: &mut Params,
        target: &str,
        second: bool,
    ) -> Option<Vec<String>> {
        let power = self.pump_side(p.take("NumAtt").as_deref().unwrap_or("0"))?;
        let toughness = self.pump_side(p.take("NumDef").as_deref().unwrap_or("0"))?;
        let keywords = match p.take("KW") {
            None => "KeywordSet::EMPTY".to_string(),
            Some(kw) if kw.trim() == "HIDDEN CARDNAME can't attack or block." => {
                "KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)".to_string()
            }
            Some(kw)
                if kw.trim() == "HIDDEN CARDNAME can't block."
                    && p.peek("Defined") == Some("Enchanted") =>
            {
                "KeywordSet::CANT_BLOCK".to_string()
            }
            Some(kw) => {
                let each: Option<Vec<&str>> =
                    kw.split('&').map(|k| keyword_const(k.trim())).collect();
                // A keyword the engine has no bit for is a whole sentence of
                // rules text ("can't block", "doesn't untap"), not a flag —
                // refuse rather than drop it.
                each?.join(".union(") + &")".repeat(kw.split('&').count().saturating_sub(1))
            }
        };
        // Every duration but the default is a lifetime the DSL spells
        // differently; none of them is "until end of turn" with a longer
        // name.
        let duration = match p.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("UntilUntaps") if !self.on_a_spell => {
                "Duration::WhileSourceTapped { control: false, power_limited: false }"
            }
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
    ) -> Option<Vec<String>> {
        if target.is_some() {
            return None;
        }
        let valid = p.take("ValidCards")?;
        let power = self.pump_all_side(p.take("NumAtt").as_deref().unwrap_or("0"))?;
        let toughness = self.pump_all_side(p.take("NumDef").as_deref().unwrap_or("0"))?;
        let keywords = match p.take("KW") {
            None => "KeywordSet::EMPTY".to_string(),
            Some(kw) if kw.trim() == "HIDDEN CARDNAME can't attack or block." => {
                "KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)".to_string()
            }
            Some(kw)
                if kw.trim() == "HIDDEN CARDNAME can't block."
                    && p.peek("Defined") == Some("Enchanted") =>
            {
                "KeywordSet::CANT_BLOCK".to_string()
            }
            Some(kw) => {
                let each: Option<Vec<&str>> =
                    kw.split('&').map(|k| keyword_const(k.trim())).collect();
                each?.join(".union(") + &")".repeat(kw.split('&').count().saturating_sub(1))
            }
        };
        if p.take("Duration").is_some() {
            return None;
        }
        p.take("IsCurse");
        if power == "Amount::Fixed(0)"
            && toughness == "Amount::Fixed(0)"
            && keywords == "KeywordSet::EMPTY"
        {
            return None;
        }
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("PUMP_ALL", &expr);
        Some(vec![format!(
            "Effect::PumpFilter {{ filter: &{filter}, controlled_by: None, \
             power: {power}, toughness: {toughness}, keywords: {keywords}, \
             duration: Duration::UntilEndOfTurn }}"
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
                (Some("Battlefield"), Some("Hand")) => {
                    let valid = p.take("ChangeType")?;
                    let expr = self.filter_expr(&valid)?;
                    let name = self.body.filter_static("BOUNCE_ALL", &expr);
                    Some(vec![format!(
                        "Effect::ReturnAllToHand {{ filter: &{name}, opponents_only: false }}"
                    )])
                }
                (Some("Battlefield"), Some("Exile")) => {
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
                (Some("Graveyard"), Some("Exile"))
                    if p.take("ChangeType").as_deref() == Some("Card") =>
                {
                    Some(vec![
                        "Effect::ExileGraveyard { player: PlayerRel::EachPlayer }".to_string(),
                    ])
                }
                (Some("Graveyard"), Some("Battlefield")) => {
                    let valid = p.take("ChangeType")?;
                    if valid.contains("Ctrl") || valid.contains("Own") {
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
        let defined = p.take("Defined");
        let who = match p.take("ChangeType").as_deref() {
            Some("Card.YouOwn") if defined.is_none() && target.is_none() => {
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
            Some("LeavesPlay") if !self.on_a_spell => "Duration::WhileYouControlSource",
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
            && p.has("Optional")
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
        // Without a target this would move nothing at all.
        if !itself && target == "TargetSpec::AnyPlayer" {
            return self.deny("`ChangeZone` with neither a target nor `Defined$`".to_string());
        }
        Some(match (origin.as_str(), destination.as_str(), itself) {
            ("Battlefield" | "Graveyard", "Hand", true) => {
                vec!["Effect::bounce(TargetSpec::ThisObject)".to_string()]
            }
            ("Graveyard", "Battlefield", true) => vec![
                "Effect::ReturnSourceFromGraveyard { face: None, tapped: false, \
                 owner_control: false, counters: None }"
                    .to_string(),
            ],
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
                vec![format!(
                    "Effect::GraveyardToBattlefield {{ target: {target}, owner_control: false, \
                     counters: None }}"
                )]
            }
            _ => return self.deny(format!("`ChangeZone` {origin} to {destination}")),
        })
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
            && (target != "TargetSpec::AnyPlayer"
                || (!self.on_a_spell
                    && ((origin == "Battlefield"
                        && (destination == "Hand"
                            || (destination == "Library"
                                && p.peek("LibraryPosition") == Some("0"))))
                        || (origin == "Graveyard"
                            && matches!(destination, "Battlefield" | "Hand")))
                    && !p.has("ChangeType")))
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
        let count = plain_number(p.take("ChangeNum").as_deref().unwrap_or("1"), self.svars)?;
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
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Tapped$ {other}`"));
                return None;
            }
        };
        let optional = match p.take("Optional").as_deref() {
            Some("You" | "True") => true,
            Some("False") => false,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Optional$ {other}`"));
                return None;
            }
            None => {
                self.note("`ChangeZone` from hand without `Optional$`".to_string());
                return None;
            }
        };
        let count = plain_number(p.take("ChangeNum").as_deref().unwrap_or("1"), self.svars)?;
        if count != 1 {
            self.note(format!("`ChangeZone` putting {count} cards from hand"));
            return None;
        }
        let filter = self.filter_expr_in_zone(&p.take("ChangeType")?, true)?;
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
        let any_number = self.any_number_artifact_exile(p, destination)?;
        let find = if any_number {
            "Find::EXILE.any_number()"
        } else {
            match (destination, tapped) {
                ("Battlefield", false) => "Find::BATTLEFIELD",
                ("Battlefield", true) => "Find::BATTLEFIELD_TAPPED",
                ("Hand", false) => "Find::HAND",
                ("Exile", false) => "Find::EXILE",
                ("Library", false) => "Find::TOP_OF_LIBRARY",
                _ => {
                    self.note(format!("`ChangeZone` searching into {destination}"));
                    return None;
                }
            }
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
        let count = plain_number(p.take("ChangeNum").as_deref().unwrap_or("1"), self.svars)?;
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
        let finds = vec![find; count].join(", ");
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

    fn any_number_artifact_exile(&mut self, p: &mut Params, destination: &str) -> Option<bool> {
        let any_number = destination == "Exile"
            && p.peek("ChangeNum") == Some("X")
            && self.svars.get("X").map(String::as_str)
                == Some("Count$ValidLibrary Artifact.YouOwn");
        if any_number {
            p.take("ChangeNum");
            for key in ["RememberChanged", "ForgetOtherRemembered"] {
                if p.take(key).as_deref() != Some("True") {
                    return self.deny(format!("an any-number exile search without `{key}$ True`"));
                }
            }
        }
        Some(any_number)
    }
}
