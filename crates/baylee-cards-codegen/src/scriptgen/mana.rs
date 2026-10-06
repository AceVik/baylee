//! Reading mana, amounts and costs.

use super::{
    Params, Tx, amount, cost_parts, counter_kind, hybrid_pair, keyword_atom, mana_color_const,
    named_atom, object_cost, plain_number, stat_atom,
};

impl Tx<'_> {
    /// `Produced$ Combo W U | Amount$ 2` and friends.
    pub(super) fn mana_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        // "Its controller adds an additional {R}" (Gauntlet of Might): mana
        // in another player's pool, which is fixed mana or nothing.
        let defined = p.take("Defined");
        if let Some(who) = defined.as_deref().filter(|d| *d != "You") {
            let who = self.player_rel(Some(who))?;
            return self.mana_for(p, who);
        }
        let produced = p.take("Produced")?;
        let restrict = match p.take("RestrictValid") {
            None => None,
            Some(valid) => Some(self.spend_restriction(&valid)?),
        };
        let raw = p.take("Amount").unwrap_or_else(|| "1".to_string());
        // A literal first and always. The `Amount` this line can now carry is
        // the *dynamic* one, and reaching for it where a number would do
        // would rewrite half the lands in the pool on the next codegen run
        // while saying nothing new about any of them — `Effect::mana` and
        // `Effect::mana_dynamic` build the same `AddMana` and only one of
        // them is what the 508 machine-owned files already say.
        let fixed = plain_number(&raw, self.svars).and_then(|n| u32::try_from(n).ok());
        let color = |c: &str| {
            Some(match c {
                "W" => "ManaColor::White",
                "U" => "ManaColor::Blue",
                "B" => "ManaColor::Black",
                "R" => "ManaColor::Red",
                "G" => "ManaColor::Green",
                "C" => "ManaColor::Colorless",
                _ => return None,
            })
        };
        let effects = if produced == "Any" {
            match fixed {
                Some(1) => vec!["Effect::mana_of_any_color()".to_string()],
                // "Add three mana of any one color" — **one** pick for the
                // whole amount, which is the difference from `Combo` below
                // and the reason `mana_choice_dynamic` exists beside
                // `mana_combination`.
                _ => vec![format!(
                    "Effect::mana_choice_dynamic(ALL_MANA_COLORS, {})",
                    self.counted_amount(&raw)?
                )],
            }
        } else if produced == "Chosen" || produced == "ChosenColor" {
            // "Add one mana of the chosen color." The colour is on the
            // permanent, named as it entered — `EnterModifier::ChooseColor`
            // is the other half, and a card printing this line without it
            // would make nothing.
            (fixed == Some(1)).then(|| vec!["Effect::mana_chosen()".to_string()])?
        } else if let Some(list) = produced.strip_prefix("Combo ") {
            self.combo_mana(list, &raw, fixed)?
        } else {
            // `Produced$ W U` is "add {W}{U}" — two mana at once, not a
            // choice between them (that is `Combo`). One effect per colour,
            // which is how the bounce lands were already written by hand.
            let Some(colors) = produced
                .split_whitespace()
                .map(color)
                .collect::<Option<Vec<_>>>()
            else {
                // `Produced$ Special EachColorAmong_Valid …` is the whole of
                // what lands here, and it is one rule rather than an
                // unreadable word: a colour per colour among a filter.
                let head = produced.split_whitespace().next().unwrap_or(&produced);
                return self.deny(format!("mana source `{head}`"));
            };
            if let Some(n) = fixed {
                colors
                    .into_iter()
                    .map(|c| format!("Effect::mana({c}, {n})"))
                    .collect()
            } else {
                // "Add {B} for each Swamp you control." One colour only,
                // because a counted amount of *two* colours is a sentence no
                // card prints and `mana_dynamic` has no room for.
                let [only] = &colors[..] else {
                    return self.deny("a counted amount of more than one colour".to_string());
                };
                vec![format!(
                    "Effect::mana_dynamic({only}, {})",
                    self.counted_amount(&raw)?
                )]
            }
        };
        let Some(filter) = restrict else {
            return Some(effects);
        };
        // "Spend this mana only to cast a creature spell" is a rider on the
        // mana, so it can only hang on one effect — a line that made two
        // mana and restricted them would need the restriction twice, and
        // nothing in the corpus prints that.
        let [only] = &effects[..] else {
            self.note("`Mana.RestrictValid` on more than one mana".to_string());
            return None;
        };
        let name = self.body.filter_static("SPEND", &filter);
        Some(vec![format!(
            "{only}.restricted(&{name}, SpendRider::None)"
        )])
    }

    /// [`Self::mana_effect`] for a pool other than the controller's:
    /// `Effect::AddManaFor`, one per colour, of a fixed amount and with no
    /// restriction. Anything else is refused by name.
    pub(super) fn mana_for(&mut self, p: &mut Params, who: &str) -> Option<Vec<String>> {
        let produced = p.take("Produced")?;
        if p.peek("RestrictValid").is_some() {
            return self.deny("restricted mana for another player".to_string());
        }
        let raw = p.take("Amount").unwrap_or_else(|| "1".to_string());
        let Some(amount) = plain_number(&raw, self.svars).and_then(|n| u16::try_from(n).ok())
        else {
            return self.deny(format!("mana for another player of `Amount$ {raw}`"));
        };
        let mut out = Vec::new();
        for symbol in produced.split_whitespace() {
            let color = match symbol {
                "W" => "ManaColor::White",
                "U" => "ManaColor::Blue",
                "B" => "ManaColor::Black",
                "R" => "ManaColor::Red",
                "G" => "ManaColor::Green",
                "C" => "ManaColor::Colorless",
                other => return self.deny(format!("mana for another player of `{other}`")),
            };
            out.push(format!(
                "Effect::AddManaFor {{ who: {who}, color: {color}, amount: {amount} }}"
            ));
        }
        Some(out)
    }

    /// `Produced$ Combo …` — a choice among the colours listed, and how many.
    ///
    /// `Combo` is the corpus's word for "one of these", and the amount is
    /// what decides which of two different sentences it is: one mana picked
    /// from a list, or *n* mana each picked from it. "Add {G}{G}, {G}{W}, or
    /// {W}{W}" is the second — the filter cycle's whole point — and
    /// `combination: true` is where it lives (CR 608.2d: the choices are made
    /// as the effect is applied, and nothing in the rules numbers them).
    pub(super) fn combo_mana(
        &mut self,
        list: &str,
        raw: &str,
        fixed: Option<u32>,
    ) -> Option<Vec<String>> {
        // `ColorIdentity` is a source of its own rather than a colour list:
        // what a Command Tower makes is read off its controller's commanders
        // (CR 903.4), so no card can name the colours.
        if list.trim() == "ColorIdentity" {
            return (fixed == Some(1))
                .then(|| vec!["Effect::mana_commander_identity()".to_string()]);
        }
        // The chosen colour is one of the options rather than a colour of
        // its own: "Add {W} or one mana of the chosen color."
        let (chosen, rest): (Vec<&str>, Vec<&str>) = list
            .split_whitespace()
            .partition(|w| *w == "Chosen" || *w == "ChosenColor");
        // `Combo Any` is the five colours written as one word, and here it is
        // a *list* rather than a source — which is what makes "add four mana
        // in any combination of colors" the same rule as the filter lands'
        // two.
        let colors: Vec<String> = if rest == ["Any"] {
            Vec::new()
        } else {
            let mut out = Vec::new();
            for word in rest {
                let Some(color) = mana_color_const(word) else {
                    // `AnyDifferent` (two mana of *different* colours) and
                    // `NotedColors` (what was chosen while drafting) are each
                    // a rule of their own, and a refusal naming the line
                    // rather than the word would send a reader back to the
                    // script to find out which.
                    return self.deny(format!("`Mana` combination over `{word}`"));
                };
                out.push(color.to_string());
            }
            out
        };
        let listed = if colors.is_empty() {
            "ALL_MANA_COLORS".to_string()
        } else {
            format!("&[{}]", colors.join(", "))
        };
        if !chosen.is_empty() {
            return (fixed == Some(1)).then(|| vec![format!("Effect::mana_chosen_or({listed})")]);
        }
        Some(match fixed {
            Some(1) if colors.is_empty() => vec!["Effect::mana_of_any_color()".to_string()],
            Some(1) => vec![format!("Effect::mana_choice({listed})")],
            // `combination: true`: one pick per mana, and the answers may
            // differ. One pick for the whole amount is `Produced$ Any`, a
            // sentence away and a different card.
            _ => vec![format!(
                "Effect::mana_combination({listed}, {})",
                self.counted_amount(raw)?
            )],
        })
    }

    /// An `Amount$` value as an `Amount`, counts included.
    ///
    /// [`amount`] reads what a number can be without a board: a literal, an
    /// `SVar` that resolves to one, and the `X` a player announced. A mana
    /// line is where the corpus's *counted* amounts are commonest — "Add {B}
    /// for each Swamp you control" — and the DSL has been able to say that
    /// since `Amount::CountOf`; Gaea's Cradle is written with it by hand.
    /// Nothing read it.
    ///
    /// Two differences from [`Self::filter_expr`]'s other counting caller are
    /// deliberate and pull the opposite way from
    /// `a_clause_that_counts_needs_the_filter_to_say_whose`. That rule refuses
    /// a filter naming no controller because `Condition::ControlCount` means
    /// "you control" and would silently narrow it; `CountOf` over
    /// `ZoneSel::Battlefield` counts everything, which is what Cloudpost
    /// prints ("each Locus on the battlefield"). And it refuses `Other`
    /// because a condition has no room for the card asking; `eval::amount`
    /// hands `matches` the source object, so Baldur's Gate's "each **other**
    /// Gate you control" is `Filter::Another` and says what it means.
    pub(super) fn counted_amount(&mut self, raw: &str) -> Option<String> {
        if let Some(n) = amount(raw, self.svars, self.has_x) {
            return Some(n);
        }
        let Some(def) = self.svars.get(raw.trim()).cloned() else {
            return self.deny(format!("mana amount `{}`", raw.trim()));
        };
        match self.count_expr(&def) {
            Some(expr) => Some(expr),
            // Named by what it resolves *through* and never by its own
            // spelling: `Amount$ X` is one letter standing for thirty
            // different questions, and the definition is the one of them this
            // card is asking.
            None => self.deny(format!("count `{}`", def.trim())),
        }
    }

    /// A `Count$Valid …` definition as an [`Amount::CountOf`] expression.
    ///
    /// Silent on a definition it cannot read, because its two callers refuse
    /// in different words and a refusal is a worklist entry.
    /// [`Self::counted_amount`] is reading a mana line and [`Self::pump_side`]
    /// a pump; a pump filed under "mana amount" sends whoever reads the
    /// report to a rule the card never touched, and a wrong reason travels
    /// further than a wrong reading because nothing downstream can check it.
    ///
    /// The definition is passed in rather than the `SVar` name so that the
    /// caller decides what an *undefined* name is called as well.
    pub(super) fn count_expr(&mut self, def: &str) -> Option<String> {
        let valid = def.trim().strip_prefix("Count$Valid ")?;
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("COUNT", &expr);
        Some(format!(
            "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::Battlefield }}"
        ))
    }

    /// The atoms that are one word of printed text each, and the filter each
    /// is:
    ///
    /// - a name ("creatures named Plague Rats", [`named_atom`]);
    /// - a number the printed words compare against ("power 2 or less",
    ///   [`stat_atom`]), or "less than this creature's power" where `X` is
    ///   the source's power ([`Self::source_power_atom`]);
    /// - a mana value against a number or the announced X
    ///   ([`Self::cmc_atom`]);
    /// - "each creature with flying", "each creature without flying"
    ///   (Hurricane, Earthquake): a keyword the engine has a bit for
    ///   ([`keyword_atom`]). A keyword it has none for is a rule, not a flag,
    ///   and stays refused.
    pub(super) fn worded_atom(&self, atom: &str) -> Option<String> {
        if let Some(name) = named_atom(atom) {
            return Some(format!("Filter::Named({name:?})"));
        }
        stat_atom(atom)
            .or_else(|| self.source_power_atom(atom))
            .or_else(|| self.cmc_atom(atom))
            .or_else(|| keyword_atom(atom))
    }

    /// `powerLTX` and `toughnessLTX` where `X` is `Count$CardPower`, the
    /// source's own power: "with power less than this creature's power",
    /// Stone Giant's "with toughness less than Stone Giant's power". Any
    /// other comparison or `X` is refused.
    pub(super) fn source_power_atom(&self, atom: &str) -> Option<String> {
        if self.svars.get("X").map(|x| x.trim()) != Some("Count$CardPower") {
            return None;
        }
        match atom {
            "powerLTX" => Some("Filter::PowerLessThanSourcePower".to_string()),
            "toughnessLTX" => Some("Filter::ToughnessLessThanSourcePower".to_string()),
            _ => None,
        }
    }

    /// `cmcLE2`, `cmcGE4`, `cmcEQX`: a mana value against a fixed number,
    /// or against the X announced for this spell or ability — only where
    /// `X` is that announcement ([`amount`] gives the reason), since the
    /// filter reads it off the source and a trigger announced none.
    pub(super) fn cmc_atom(&self, atom: &str) -> Option<String> {
        let (cmp, number) = atom.strip_prefix("cmc")?.split_at_checked(2)?;
        if number == "X" {
            let announced =
                self.has_x && self.svars.get("X").map(String::as_str) == Some("Count$xPaid");
            return match cmp {
                "LE" if announced => Some("Filter::CmcAtMostX".to_string()),
                "EQ" if announced => Some("Filter::CmcExactlyX".to_string()),
                _ => None,
            };
        }
        let n: u32 = number.parse().ok()?;
        Some(match cmp {
            "LE" => format!("Filter::CmcAtMost({n})"),
            "LT" => format!("Filter::CmcAtMost({})", n.checked_sub(1)?),
            "GE" => format!("Filter::CmcAtLeast({n})"),
            "GT" => format!("Filter::CmcAtLeast({})", n.checked_add(1)?),
            "EQ" => format!("Filter::And(&[Filter::CmcAtMost({n}), Filter::CmcAtLeast({n})])"),
            _ => return None,
        })
    }

    /// [`amount`], or else a count of permanents ([`Self::count_expr`]):
    /// Karma's "damage equal to the number of Swamps they control". A count
    /// is read as the ability resolves, like every other `Amount`.
    pub(super) fn amount_or_count(&mut self, raw: &str) -> Option<String> {
        if let Some(n) = amount(raw, self.svars, self.has_x) {
            return Some(n);
        }
        let def = self.svars.get(raw.trim())?.clone();
        self.count_expr(&def)
    }

    /// `RestrictValid$ Spell.Creature` — what produced mana may be spent on.
    ///
    /// Every alternative has to be a *spell*: `Activated.Hero` restricts an
    /// ability activation instead, which is a second kind of restriction the
    /// `ManaRestriction` filter cannot say, and a card printing both means
    /// both.
    pub(super) fn spend_restriction(&self, valid: &str) -> Option<String> {
        let spells: Option<Vec<&str>> = valid
            .split(',')
            .map(|alt| alt.trim().strip_prefix("Spell."))
            .collect();
        let Some(spells) = spells else {
            self.note("`Mana.RestrictValid` beyond a spell".to_string());
            return None;
        };
        self.filter_expr(&spells.join(","))
    }

    /// A `Cost$` value as a `Cost` expression, plus whether it taps.
    ///
    /// An unreadable token names itself: a cost is where the widest variety
    /// of the corpus's syntax shows up — `Discard<…>`, `Exile<…>`,
    /// `tapXType<…>` — and a report saying "a cost" would send the reader
    /// back to the script to find out which.
    ///
    /// `&mut self` because one part carries a filter, and a filter is
    /// declared as a `static` above the card the way a target's is.
    pub(super) fn cost_expr(&mut self, raw: &str) -> Option<String> {
        let (mana, parts) = self.cost_pieces(raw)?;
        Some(crate::body::cost_literal(&mana, &parts))
    }

    /// The same reading, before [`crate::body::cost_literal`] joins it.
    ///
    /// Split out because one caller wants a *part* rather than a cost:
    /// "unless you return an untapped Plains" is priced by a single
    /// [`CostPart`], and re-reading that syntax beside this one would be two
    /// places for `Return<1/Plains>` to mean two things.
    pub(super) fn cost_pieces(&mut self, raw: &str) -> Option<(String, Vec<String>)> {
        let mut mana = String::new();
        let mut parts: Vec<String> = Vec::new();
        for token in cost_parts(raw) {
            let token = token.as_str();
            if token == "T" {
                parts.push("TapSelf".to_string());
            } else if token == "Q" {
                parts.push("UntapSelf".to_string());
            } else if token.starts_with("Sac<1/CARDNAME") {
                parts.push("SacrificeSelf".to_string());
            } else if let Some((n, kind)) = token
                .strip_prefix("AddCounter<")
                .and_then(|t| t.strip_suffix('>'))
                .and_then(|t| t.split_once('/'))
            {
                // "Put a -1/-1 counter on this creature" as a cost. The
                // counter noun goes through the same table the `PutCounter`
                // *effect* reads, so the two cannot come to disagree about
                // what `M1M1` is — and a noun the DSL has no kind for takes
                // the card off the list rather than guessing at one.
                let Ok(n) = n.parse::<u16>() else {
                    return self.deny(format!("counter count `{n}`"));
                };
                let Some(kind) = counter_kind(kind) else {
                    return self.deny(format!("counter `{kind}`"));
                };
                parts.push(format!("PutCounterSelf {{ kind: {kind}, n: {n} }}"));
            } else if let Some((n, kind)) = token
                .strip_prefix("SubCounter<")
                .and_then(|t| t.strip_suffix('>'))
                .and_then(|t| t.split_once('/'))
            {
                // "Remove a corpse counter from this creature" as a cost:
                // a fixed number of one kind, from the source. Refused by
                // name: a count that is not a number (`X` is announced, and
                // "any number" is chosen), a loyalty cost (a loyalty
                // ability's, CR 606.4, which only a main phase with an empty
                // stack may activate, once a turn, CR 606.3), and the longer
                // forms that name where the counters come from ("from a
                // creature you control").
                let Ok(n) = n.parse::<u16>() else {
                    return self.deny(format!("counter count `{n}`"));
                };
                if kind.contains('/') {
                    return self.deny(format!("a counter cost from `{kind}`"));
                }
                if kind == "LOYALTY" {
                    return self.deny("a loyalty cost".to_string());
                }
                let Some(kind) = counter_kind(kind) else {
                    return self.deny(format!("counter `{kind}`"));
                };
                parts.push(format!("RemoveCounterSelf {{ kind: {kind}, n: {n} }}"));
            } else if let Some((kind, body)) = object_cost(token) {
                parts.push(self.object_cost_part(kind, body, token)?);
            } else if let Some(n) = token
                .strip_prefix("PayLife<")
                .and_then(|t| t.strip_suffix('>'))
                .and_then(|t| t.parse::<u16>().ok())
            {
                parts.push(format!("PayLife({n})"));
            } else if token.chars().all(|c| c.is_ascii_digit())
                || matches!(token, "W" | "U" | "B" | "R" | "G" | "C")
            {
                mana.push('{');
                mana.push_str(token);
                mana.push('}');
            } else if let Some(pair) = hybrid_pair(token) {
                // `Cost$ UR T` is `{U/R}, {T}` — one mana of either colour
                // (CR 107.4e), which the corpus writes as the two letters run
                // together and this side writes with the slash the card
                // prints. The two have to be *different* letters: `{U/U}` is
                // not a symbol, and `ColorPair::new` asserts as much. Nothing
                // in the reference writes a doubled pair, which is checked
                // rather than assumed — a doubled one would be a token this
                // rule quietly turned into a panic at compile time.
                //
                // Strictly two colours, so `2W` and `WP` keep refusing by
                // name: a `{2/W}` costs *two* generic as its other half and a
                // `{W/P}` is paid with life, and neither is what a reader
                // that saw "two letters" would have written.
                mana.push_str(&pair);
            } else {
                let head = token.split('<').next().unwrap_or(token);
                return self.deny(format!("cost `{head}`"));
            }
        }
        Some((mana, parts))
    }

    /// One cost part the player pays by naming an object.
    ///
    /// Five spellings of one shape — `Sac<1/…>`, `Discard<1/…>`,
    /// `tapXType<1/…>`, `Return<1/…>`, `ExileFromGrave<1/…>` — and they were
    /// worth writing once rather than five times because what differs
    /// between them is two facts: which `CostPart` they are, and whether the
    /// object has to be one the payer controls.
    ///
    /// **One object per part**, which is what `CostPart` carries: 97 of the
    /// corpus's costs sacrifice two, three or X, and paying one of them
    /// would be a discount rather than the cost the card prints.
    ///
    /// "You control" is added where the script does not say it, which is the
    /// one place this reader writes a clause it did not read — and only for
    /// the three that take a permanent. CR 701.21a lets a player sacrifice
    /// only what they control and CR 118.3 says the same of tapping one to
    /// pay, so a filter without it would be the card and
    /// `cost_wizard::options` offering two different menus for one cost. A
    /// **discard** takes none of it: a card in a hand has no controller at
    /// all, and the engine reads that zone by whose hand it is. An exile
    /// from the graveyard is the same case one pile over, and
    /// `ExileFromGrave<1/CARDNAME>` — eternalize's "exile this card from
    /// your graveyard" — is refused by name, because it is paid from a zone
    /// no ability is activated from yet.
    pub(super) fn object_cost_part(
        &mut self,
        kind: &str,
        body: &str,
        token: &str,
    ) -> Option<String> {
        let mut fields = body.splitn(3, '/');
        let (Some(n), Some(spec)) = (fields.next(), fields.next()) else {
            return self.deny(format!("cost `{token}`"));
        };
        if n != "1" {
            return self.deny(format!("a cost naming `{n}` objects"));
        }
        let (variant, controlled) = match kind {
            "Sac" => ("Sacrifice", true),
            "Discard" => ("Discard", false),
            "tapXType" => ("TapOther", true),
            "ExileFromGrave" => ("ExileFromGraveyard", false),
            _ => ("ReturnToHand", true),
        };
        if spec == "CARDNAME" {
            // The source pays for itself, which is a part with no filter and
            // no question. `Sac<1/CARDNAME>` never reaches here (it is
            // matched one branch up), so this is the return's own case.
            return Some(match variant {
                "ReturnToHand" => "ReturnSelfToHand".to_string(),
                _ => return self.deny(format!("cost `{token}` naming itself")),
            });
        }
        let spec = if controlled {
            // Appended per alternative, because a valid-string's commas are
            // `Or` and a clause glued to the end would narrow only the last
            // branch.
            spec.split(',')
                .map(|alt| {
                    if alt.contains("YouCtrl") {
                        alt.to_string()
                    } else if alt.contains('.') {
                        format!("{alt}+YouCtrl")
                    } else {
                        format!("{alt}.YouCtrl")
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        } else {
            spec.to_string()
        };
        let expr = self.filter_expr(&spec)?;
        let name = self.body.filter_static("COST", &expr);
        Some(format!("{variant}(&{name})"))
    }
}
