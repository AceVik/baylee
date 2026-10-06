//! Reading the effects that are more than one line: tokens, animation,
//! pumps, zone changes and library searches.

use super::{
    CLUE_TOKEN_SCRIPT, Params, Tx, amount, card_type_const, color_const, keyword_const,
    plain_number, pump_amount,
};

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
    pub(super) fn sacrifice_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        for key in [
            "Defined",
            "SacValid",
            "Amount",
            "Optional",
            "RememberSacrificed",
        ] {
            if p.take(key).is_some() {
                return self.deny(format!("a sacrifice naming `{key}`"));
            }
        }
        Some(vec!["Effect::SacrificeSelf".to_string()])
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
            Some(raw) => match amount(raw, self.svars, self.has_x) {
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

    /// `Animate`: the manland sentence — "until end of turn, this land
    /// becomes a 4/4 white and blue Elemental creature with flying and
    /// vigilance. It's still a land."
    ///
    /// One printed sentence, four continuous effects, because CR 613.1
    /// applies type, colour, ability and power/toughness in that order and
    /// each is its own layer. "It's still a land" is why the types are
    /// *added* rather than set — an animated Colonnade that stopped being a
    /// land would stop making mana.
    ///
    /// Two objects are read, each the one `Filter::This` binds to: the
    /// source (`Defined$ Self`) on a chain that targets nothing, and the
    /// target on a line that names no `Defined$` — the Laces' "target spell
    /// or permanent becomes red". `Filter::This` binds to the first target
    /// when there is one and to the source when there is not, so a chain
    /// with both would animate the wrong permanent, and is refused.
    ///
    /// `Duration$ Permanent` is "for the rest of the game"
    /// (`Duration::Indefinitely`), `UntilEndOfCombat` is until end of combat;
    /// without one it is until end of turn.
    ///
    /// `RemoveCreatureTypes$ True` is CR 205.1b's "becomes a [creature type]
    /// artifact creature": the first creature type named replaces the ones it
    /// had (`Modifier::ReplaceCreatureTypes`) and every other type is kept.
    ///
    /// `RemoveLandTypes$ True` is "target land becomes a Forest" (Gaea's
    /// Liege): CR 305.7's setting of a land's subtype, `Modifier::SetLandType`,
    /// for the one basic land type named; a second land type, or none, is
    /// refused. `Duration$ UntilHostLeavesPlay` is "until [this] leaves the
    /// battlefield", `Duration::WhileSourceOnBattlefield`.
    pub(super) fn animate_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        match (p.take("Defined").as_deref(), target) {
            (Some("Self"), None) | (None, Some(_)) => {}
            _ => {
                self.note("`Animate` of something other than the source".to_string());
                return None;
            }
        }
        let duration = match p.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilHostLeavesPlay") => "Duration::WhileSourceOnBattlefield",
            Some(other) => {
                self.note(format!("`Animate` lasting `{other}`"));
                return None;
            }
        };
        let replace_creature_types = match p.take("RemoveCreatureTypes").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`RemoveCreatureTypes$ {other}`")),
        };
        let set_land_type = match p.take("RemoveLandTypes").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`RemoveLandTypes$ {other}`")),
        };
        let mut out = Vec::new();
        let types = p.take("Types").unwrap_or_default();
        self.animate_types(
            &types,
            (replace_creature_types, set_land_type),
            duration,
            &mut out,
        )?;
        // Layer 5: colour. Without `OverwriteColors$ True` the card keeps
        // the colours it had, which is `AddColor` (CR 105.3).
        if let Some(raw) = p.take("Colors") {
            let overwrite = p.take("OverwriteColors").as_deref() == Some("True");
            let colors: Option<Vec<&str>> = raw.split(',').map(|c| color_const(c.trim())).collect();
            let Some(colors) = colors else {
                self.note(format!("`Animate` into colours `{raw}`"));
                return None;
            };
            let which = if overwrite { "SetColor" } else { "AddColor" };
            out.push(Self::animate_expr(
                &format!(
                    "Modifier::{which}(ColorSet::from_slice(&[{}]))",
                    colors.join(", ")
                ),
                duration,
            ));
        }
        // Layer 6: keywords it gains.
        if let Some(raw) = p.take("Keywords") {
            let each: Option<Vec<&str>> = raw.split('&').map(|k| keyword_const(k.trim())).collect();
            let Some(each) = each else {
                self.note(format!("`Animate` granting `{raw}`"));
                return None;
            };
            let joined =
                each.join(".union(") + &")".repeat(raw.split('&').count().saturating_sub(1));
            out.push(Self::animate_expr(
                &format!("Modifier::AddKeyword({joined})"),
                duration,
            ));
        }
        // Layer 7b: the printed P/T it takes on. Both halves or neither —
        // `SetPT` sets both, and half a set would invent the other.
        match (p.take("Power"), p.take("Toughness")) {
            (Some(power), Some(toughness)) => {
                let power: i16 = power.parse().ok()?;
                let toughness: i16 = toughness.parse().ok()?;
                out.push(Self::animate_expr(
                    &format!("Modifier::SetPT({power}, {toughness})"),
                    duration,
                ));
            }
            (None, None) => {}
            _ => {
                self.note("`Animate` setting only one of power and toughness".to_string());
                return None;
            }
        }
        if out.is_empty() {
            self.note("`Animate` that changes nothing".to_string());
            return None;
        }
        Some(out)
    }

    /// Layer 4 of an [`Self::animate_effect`]: the types it becomes. A word
    /// is either a card type or a subtype, and the corpus writes both in one
    /// list. `remove` is (`RemoveCreatureTypes$ True`, `RemoveLandTypes$
    /// True`), each of which needs the one word it replaces with.
    pub(super) fn animate_types(
        &mut self,
        types: &str,
        remove: (bool, bool),
        duration: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        let (replace_creature_types, set_land_type) = remove;
        let mut set = false;
        let mut replaced = false;
        for word in types.split(',').filter(|w| !w.trim().is_empty()) {
            let word = word.trim();
            let modifier = if let Some(types) = card_type_const(word) {
                format!("Modifier::AddType({types})")
            } else if set_land_type && let Some(path) = self.basic_land_type(word) {
                if set {
                    return self.deny("`RemoveLandTypes$` naming two land types".to_string());
                }
                set = true;
                format!("Modifier::SetLandType({path})")
            } else {
                if set_land_type
                    && self
                        .cats
                        .const_path_of(baylee_core::types::SubtypeKind::Land, word)
                        .is_some()
                {
                    return self.deny(format!("`RemoveLandTypes$` into `{word}`"));
                }
                let Some(path) = self.cats.const_path(word) else {
                    self.note(format!("`Animate` into `{word}`"));
                    return None;
                };
                let creature_type = self
                    .cats
                    .const_path_of(baylee_core::types::SubtypeKind::Creature, word)
                    .is_some();
                if replace_creature_types && creature_type && !replaced {
                    replaced = true;
                    format!("Modifier::ReplaceCreatureTypes({path})")
                } else {
                    format!("Modifier::AddSubtype({path})")
                }
            };
            out.push(Self::animate_expr(&modifier, duration));
        }
        if replace_creature_types && !replaced {
            return self.deny("`RemoveCreatureTypes$` naming no creature type".to_string());
        }
        if set_land_type && !set {
            return self.deny("`RemoveLandTypes$` naming no basic land type".to_string());
        }
        Some(())
    }

    /// One layer of an [`Self::animate_effect`], as the `Effect` expression.
    ///
    /// No layer is passed in because none is written out: `Effect::continuous`
    /// derives it from the modifier the way CR 613.1 does, so the emitter
    /// cannot name a layer that disagrees with what it is applying.
    pub(super) fn animate_expr(modifier: &str, duration: &str) -> String {
        format!("Effect::continuous(&Filter::This, {modifier}, {duration})")
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
    pub(super) fn pump_effect(&mut self, p: &mut Params, target: &str) -> Option<Vec<String>> {
        let power = self.pump_side(p.take("NumAtt").as_deref().unwrap_or("0"))?;
        let toughness = self.pump_side(p.take("NumDef").as_deref().unwrap_or("0"))?;
        let keywords = match p.take("KW") {
            None => "KeywordSet::EMPTY".to_string(),
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
        if p.take("Duration").is_some() {
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
            _ => None,
        };
        if let Some(whom) = whom {
            return Some(vec![format!(
                "Effect::PumpFilter {{ filter: &{whom}, controlled_by: None, \
                 power: {power}, toughness: {toughness}, keywords: {keywords}, \
                 duration: Duration::UntilEndOfTurn }}"
            )]);
        }
        Some(match defined.as_deref() {
            None | Some("Targeted") => {
                // Without a target this would pump nothing at all.
                if target == "TargetSpec::AnyPlayer" {
                    return None;
                }
                vec![format!(
                    "Effect::PumpTarget {{ power: {power}, toughness: {toughness}, \
                     keywords: {keywords}, duration: Duration::UntilEndOfTurn }}"
                )]
            }
            Some(_) => return None,
        })
    }

    /// An `Effect` line: a static ability that lasts the turn, read by the
    /// one sentence its static says. Every other static stays refused.
    pub(super) fn static_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        let body = self.svars.get(&statics)?.clone();
        let (mode, _) = Params::parse(&body)?;
        match mode.as_str() {
            "CantBlockBy" => self.unblockable_effect(p, target),
            "CantRegenerate" => self.cant_regenerate_effect(p, target),
            _ => None,
        }
    }

    /// "It can't be regenerated this turn" (Disintegrate, Carbonize): an
    /// `Effect` whose one static says no regeneration applies to what it
    /// remembers, which is the line's target, and which ends when that
    /// leaves the battlefield — `Effect::CantBeRegeneratedThisTurn`, kept
    /// for that object only (CR 400.7, 701.19c).
    ///
    /// "If it's a creature" (`ConditionDefined$ ParentTarget |
    /// ConditionPresent$ Creature`) is asked of the target as the line
    /// resolves. "A creature dealt damage this way" (`Remembered.Creature`
    /// after `RememberDamaged$`) asks whether damage was dealt, and is
    /// refused.
    pub(super) fn cant_regenerate_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let statics = p.take("StaticAbilities")?;
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CantRegenerate"
            || st.take("ValidCard").as_deref() != Some("Card.IsRemembered")
            || !st.exhausted()
        {
            return None;
        }
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if ends.as_deref() != Some("Battlefield") || p.take("Duration").is_some() {
            return None;
        }
        p.take("IsCurse");
        let target = target.filter(|t| {
            !matches!(
                *t,
                "TargetSpec::Player(PlayerRel::Chosen)" | "TargetSpec::AnyPlayer"
            )
        })?;
        if !matches!(
            p.take("RememberObjects").as_deref(),
            Some("Targeted" | "ParentTarget")
        ) {
            return None;
        }
        let effect = format!("Effect::CantBeRegeneratedThisTurn {{ target: {target} }}");
        let creature = match (p.peek("ConditionDefined"), p.peek("ConditionPresent")) {
            (None, None) => false,
            (Some("ParentTarget" | "Targeted"), Some("Creature")) => {
                p.take("ConditionDefined");
                p.take("ConditionPresent");
                true
            }
            _ => return None,
        };
        Some(vec![if creature {
            format!("Effect::IfTargetMatches {{ filter: &Filter::CREATURE, then: &[{effect}] }}")
        } else {
            effect
        }])
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
        if ends.as_deref() != Some("Battlefield") || p.take("Duration").is_some() {
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
    ) -> Option<Vec<String>> {
        let origin = p.take("Origin")?;
        let destination = p.take("Destination")?;
        p.claim_label("ChangeTypeDesc", "ChangeType");
        // A library search is a different effect, not a zone change with a
        // hidden target: a card in a library cannot be targeted at all
        // (CR 115.2 needs a visible object), so `Effect::SearchLibrary`
        // *finds* rather than moves, and carries the shuffle with it.
        if origin == "Library" {
            return self.search_library(p, &destination, target);
        }
        let target = target.unwrap_or("TargetSpec::AnyPlayer");
        let itself = match p.take("Defined").as_deref() {
            None => false,
            Some("Self") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` of `Defined$ {other}`"));
                return None;
            }
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
        // Without a target this would move nothing at all.
        if !itself && target == "TargetSpec::AnyPlayer" {
            self.note("`ChangeZone` with neither a target nor `Defined$`".to_string());
            return None;
        }
        Some(match (origin.as_str(), destination.as_str(), itself) {
            ("Battlefield", "Hand", false) => vec![format!("Effect::bounce({target})")],
            ("Battlefield", "Exile", false) => vec![format!("Effect::exile({target})")],
            ("Battlefield", "Exile", true) => vec!["Effect::ExileSource".to_string()],
            // "Return target card from your graveyard to your hand"
            // (Regrowth): the target is a `CardInGraveyard`, which the chain
            // read off `Origin$` before it read the valid-string.
            ("Graveyard", "Hand", false) if target.starts_with("TargetSpec::CardInGraveyard") => {
                vec![format!("Effect::GraveyardToHand {{ target: {target} }}")]
            }
            // "Return target creature card from your graveyard to the
            // battlefield" (Resurrection): it enters under the control of
            // the player whose effect put it there (CR 110.2a), which is
            // `owner_control: false`. A line that says otherwise carries a
            // key (`GainControl$`, `WithCountersType$`) this does not claim.
            ("Graveyard", "Battlefield", false)
                if target.starts_with("TargetSpec::CardInGraveyard") =>
            {
                vec![format!(
                    "Effect::GraveyardToBattlefield {{ target: {target}, owner_control: false, \
                     counters: None }}"
                )]
            }
            _ => {
                self.note(format!("`ChangeZone` {origin} to {destination}"));
                return None;
            }
        })
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
        let filter = self.filter_expr(&p.take("ChangeType")?)?;
        let name = self.body.filter_static("RETURN", &filter);
        Some(vec![format!(
            "Effect::ReturnChosenToHand {{ who: PlayerRel::You, filter: &{name} }}"
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
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some() {
            self.note("`ChangeZone` searching a library *and* targeting".to_string());
            return None;
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Tapped$ {other}`"));
                return None;
            }
        };
        let find = match (destination, tapped) {
            ("Battlefield", false) => "Find::BATTLEFIELD",
            ("Battlefield", true) => "Find::BATTLEFIELD_TAPPED",
            ("Hand", false) => "Find::HAND",
            _ => {
                self.note(format!("`ChangeZone` searching into {destination}"));
                return None;
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
        if count > 1 && !optional && !mandatory {
            self.note(
                "`ChangeZone` finding several cards without saying whether that is a maximum"
                    .to_string(),
            );
            return None;
        }
        let filter = self.filter_expr(&p.take("ChangeType")?)?;
        let name = self.body.filter_static("SEARCH", &filter);
        let finds = vec![find; count].join(", ");
        Some(vec![format!(
            "Effect::SearchLibrary {{ filter: &{name}, finds: &[{finds}], optional: {optional} }}"
        )])
    }
}
