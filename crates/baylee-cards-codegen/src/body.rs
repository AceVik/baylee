//! What a generator produces for one card.
//!
//! Two readers fill this in — [`landgen`](crate::landgen) from a land's
//! printed text and [`scriptgen`](crate::scriptgen) from a card-script reference
//! script — and [`stubgen`](crate::stubgen) renders it. Both obey the same
//! rule: a body is produced only when the whole card was understood, so a
//! generated `Coverage::Generated` (or `Implemented`, once tested) never
//! means "most of it".

use std::fmt::Write as _;

/// A `Cost` expression, from the mana a reader accumulated and the parts it
/// recognised.
///
/// Both readers build the same two pieces and used to render them the same
/// way twice. `parts` are bare variant names (`TapSelf`, `PayLife(1)`),
/// because `cost!` supplies the `CostPart::` prefix — which is the same word
/// three times on a fetchland and is not what a reader is checking.
///
/// The three forms are the card's, not the code's: an empty cost is
/// `Cost::FREE`, a bare tap is `Cost::TAP`, and everything else reads left to
/// right the way the card prints it — mana, then the rest.
#[must_use]
pub fn cost_literal(mana: &str, parts: &[String]) -> String {
    match (mana.is_empty(), parts) {
        (true, []) => "Cost::FREE".to_string(),
        (true, [one]) if one == "TapSelf" => "Cost::TAP".to_string(),
        (true, _) => format!("cost!({})", parts.join(", ")),
        (false, []) => format!("cost!(\"{mana}\")"),
        (false, _) => format!("cost!(\"{mana}\", {})", parts.join(", ")),
    }
}

/// The generated body of one recognised card.
#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct CardBody {
    /// Extra `static` items (filters) to emit above the card literal.
    pub statics: String,
    /// `EnterModifier` expressions for the front face.
    pub enter_modifiers: Vec<String>,
    /// `AlternativeCost` expressions for the front face.
    pub alternative_costs: Vec<String>,
    /// Mandatory additional `CostPart` expressions (`CostPart::…`) for a spell face.
    pub mandatory_additional_costs: Vec<String>,
    /// `convoke = true` on the face, when present.
    pub convoke: bool,
    /// `delve = true` on the face, when present.
    pub delve: bool,
    /// `improvise = true` on the face, when present.
    pub improvise: bool,
    /// `replicate` expression for the face, when present.
    pub replicate: Option<String>,
    /// `disguise` expression for the face, when present.
    pub disguise: Option<String>,
    /// `flashback` expression for the front face, when present.
    pub flashback: Option<String>,
    /// Bare `CostPart` expressions accompanying `flashback`, before the
    /// renderer adds the variant prefix. Always empty without a flashback cost.
    pub flashback_parts: Vec<String>,
    /// `madness` expression for the front face, when present.
    pub madness: Option<String>,
    /// `foretell` expression for the front face, when present.
    pub foretell: Option<String>,
    /// `miracle` expression for the front face, when present.
    pub miracle: Option<String>,
    /// `dash` expression for the front face, when present.
    pub dash: Option<String>,
    /// `blitz` expression for the front face, when present.
    pub blitz: Option<String>,
    /// `morph` expression for the front face, when present.
    pub morph: Option<String>,
    /// `megamorph` expression for the front face, when present.
    pub megamorph: Option<String>,
    /// Bare `CostPart` expressions accompanying `morph`/`megamorph`/`disguise`,
    /// before the renderer adds the `CostPart::` prefix.
    pub morph_parts: Vec<String>,
    /// `bestow` expression for the front face, when present.
    pub bestow: Option<String>,
    /// `prototype` expression for the face, when present.
    pub prototype: Option<String>,
    /// `bargain = true` on the face, when present.
    pub bargain: bool,
    /// `emerge` expression for the face, when present.
    pub emerge: Option<String>,
    /// `jump_start = true` on the face, when present.
    pub jump_start: bool,
    /// `aftermath = true` on the face, when present.
    pub aftermath: bool,
    /// `disturb` mana expression (e.g. `"{1}{U}"`) on a Disturb card's faces,
    /// emitted on the back face as `disturb = true` with `mana_cost`.
    pub disturb: Option<String>,
    /// `escape = Some(Escape { ... })` expression on the face, when present.
    pub escape: Option<String>,
    /// `squad = cost!(...)` expression on the face, when present.
    pub squad: Option<String>,
    /// `offspring = cost!(...)` expression on the face, when present.
    pub offspring: Option<String>,
    /// Whether `CardDef::fuse` should be emitted (`true`) on this split card.
    pub fuse: bool,
    /// `behold_or_pay` expression for the face, when present.
    pub behold_or_pay: Option<String>,
    /// `mutate` expression for the face, when present.
    pub mutate: Option<String>,
    /// `plot` expression for the face, when present.
    pub plot: Option<String>,
    /// `casualty` expression for the face, when present.
    pub casualty: Option<String>,
    /// `impending` `(count, mana!(...))` expression for the face, when present.
    pub impending: Option<String>,
    /// `gift` (`GiftKind::...`) expression for the face, when present.
    pub gift: Option<String>,
    /// `warp` (`cost!(...)`) expression for the face, when present.
    pub warp: Option<String>,
    /// `cost_reduction` expression for the front face, when present.
    pub cost_reduction: Option<String>,
    /// `entwine` expression for the front face, when present.
    pub entwine: Option<String>,
    /// `buyback` expression for the front face, when present.
    pub buyback: Option<String>,
    /// `multikicker` expression for the front face, when present.
    pub multikicker: Option<String>,
    /// `additional_costs` (kicker) expressions for the front face.
    pub additional_costs: Vec<String>,
    /// `kicked_targets` expression for the front face, when present.
    pub kicked_targets: Option<String>,
    /// `KeywordSet` constants to union.
    pub keywords: Vec<String>,
    /// Ability expressions for the card literal (or face 0 on a two-faced card).
    pub abilities: Vec<String>,
    /// Second face body when a card has two transcoded faces (`Split`,
    /// `Adventure`, `Modal`, `DoubleFaced`). `None` for single-faced cards.
    pub back: Option<Box<Self>>,
    /// Whether `back` is an Adventure face (CR 716, `FaceDef::adventure`).
    pub back_is_adventure: bool,
    /// Whether `back` is a Prepare inset face (CR 723.3, `AbilityDef::Prepared`).
    pub back_is_prepare: bool,
    /// Whether the script declares bare `K:Partner` (emitted on `CardDef::partner`).
    pub has_partner: bool,
    /// Short human notes for the `// IMPLEMENTED — …` line.
    pub notes: Vec<String>,
}

impl CardBody {
    /// Whether this face itself carries any rules (ignoring `back`).
    #[must_use]
    pub fn has_own_rules(&self) -> bool {
        !(self.abilities.is_empty()
            && self.enter_modifiers.is_empty()
            && self.alternative_costs.is_empty()
            && self.mandatory_additional_costs.is_empty()
            && !self.convoke
            && !self.delve
            && !self.improvise
            && self.replicate.is_none()
            && self.disguise.is_none()
            && self.flashback.is_none()
            && self.madness.is_none()
            && self.foretell.is_none()
            && self.miracle.is_none()
            && self.dash.is_none()
            && self.blitz.is_none()
            && self.morph.is_none()
            && self.megamorph.is_none()
            && self.morph_parts.is_empty()
            && self.bestow.is_none()
            && self.prototype.is_none()
            && !self.bargain
            && self.emerge.is_none()
            && !self.jump_start
            && !self.aftermath
            && self.cost_reduction.is_none()
            && self.entwine.is_none()
            && self.buyback.is_none()
            && self.multikicker.is_none()
            && self.disturb.is_none()
            && self.escape.is_none()
            && self.squad.is_none()
            && self.offspring.is_none()
            && !self.fuse
            && self.behold_or_pay.is_none()
            && self.mutate.is_none()
            && self.plot.is_none()
            && self.casualty.is_none()
            && self.impending.is_none()
            && self.gift.is_none()
            && self.warp.is_none()
            && self.additional_costs.is_empty()
            && self.kicked_targets.is_none()
            && self.keywords.is_empty()
            && !self.has_partner)
    }

    /// Whether anything at all was read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.has_own_rules() && self.back.is_none()
    }

    /// Declares a `static <NAME>: Filter = <expr>;` and returns its name.
    ///
    /// `TargetSpec::Object` and friends hold a `&'static Filter`, and a
    /// borrow of a `Filter` literal promotes to exactly that inside a
    /// `static` — so this is legibility and not necessity. What it buys is
    /// that a target the card computes is named once, above the literal,
    /// where the reader meets it before the ability that aims with it.
    ///
    /// A filter that is *already* a name buys none of that, so it is handed
    /// straight back: `static TARGET1: Filter = Filter::CREATURE;` gives one
    /// spelling of "a creature" a second, card-local spelling, which is the
    /// duplication this was meant to prevent. Fifteen generated cards
    /// carried a `static` whose whole body was a bare constant — eight of
    /// them that one.
    pub fn filter_static(&mut self, prefix: &str, expr: &str) -> String {
        if !expr.contains(['(', '[', ',', ' ']) {
            return expr.to_string();
        }
        // And the same filter written twice in one card is one filter, for
        // the same reason: Rivendell prints "a legendary creature you
        // control" in two sentences — the clause it enters under and the
        // clause its ability asks — and was given two byte-identical
        // statics, which is one thing under two names in a file whose whole
        // job is to be read.
        if let Some(name) = self.declared(prefix, expr) {
            return name;
        }
        let n = self.statics.matches("static ").count() + 1;
        let name = format!("{prefix}{n}");
        let _ = write!(self.statics, "static {name}: Filter = {expr};\n\n");
        name
    }

    /// Declares a `static <NAME>: Condition = <expr>;` and returns its name.
    ///
    /// A state trigger holds its condition by `&'static` reference
    /// (`Trigger::State`), and the condition itself borrows a filter static,
    /// which a promoted temporary may not do: a name above the literal is
    /// the one spelling that always compiles.
    pub fn condition_static(&mut self, prefix: &str, expr: &str) -> String {
        let n = self.statics.matches("static ").count() + 1;
        let name = format!("{prefix}{n}");
        let _ = write!(self.statics, "static {name}: Condition = {expr};\n\n");
        name
    }

    /// Declares a `static <NAME>: AbilityDef = <expr>;` and returns its name.
    ///
    /// [`Modifier::GrantActivatedDefinition`] and
    /// [`Modifier::GrantTriggeredDefinition`] hold `&'static AbilityDef`,
    /// which sits above `card!` so `const` evaluation can borrow filters,
    /// effects, conditions, and targets nested inside the definition.
    pub fn ability_static(&mut self, prefix: &str, expr: &str) -> String {
        let n = self.statics.matches("static ").count() + 1;
        let name = format!("{prefix}{n}");
        let _ = write!(self.statics, "static {name}: AbilityDef = {expr};\n\n");
        name
    }

    /// Declares a deduplicated `static <NAME>: [AbilityDef; N] = [..];` slice
    /// and returns its name.
    pub fn ability_slice_static(&mut self, prefix: &str, exprs: &[String]) -> String {
        let len = exprs.len();
        let ty = format!("[AbilityDef; {len}]");
        let body = format!("[{}]", exprs.join(", "));
        if let Some(name) = self.declared_with_type(prefix, &ty, &body) {
            return name;
        }
        let n = self.statics.matches("static ").count() + 1;
        let name = format!("{prefix}{n}");
        let _ = write!(self.statics, "static {name}: {ty} = {body};\n\n");
        name
    }

    /// Removes `static` declarations whose identifier is never referenced by
    /// either face's emitted code or any reachable `static` declaration.
    pub fn prune_unused_statics(&mut self) {
        if self.statics.is_empty() {
            return;
        }
        let blocks: Vec<(&str, &str, &str)> = self
            .iter_static_blocks()
            .map(|(full, name, _, body)| (full, name, body))
            .collect();
        if blocks.is_empty() {
            return;
        }
        let mut roots = Vec::new();
        self.collect_static_roots(&mut roots);
        let mut used = vec![false; blocks.len()];
        for (i, &(_, name, _)) in blocks.iter().enumerate() {
            used[i] = roots.iter().any(|root| contains_ident(root, name));
        }
        loop {
            let mut changed = false;
            for (i, &(_, name, _)) in blocks.iter().enumerate() {
                if used[i] {
                    continue;
                }
                if blocks
                    .iter()
                    .enumerate()
                    .any(|(j, &(_, _, rhs))| used[j] && contains_ident(rhs, name))
                {
                    used[i] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut pruned = String::new();
        for (i, &(full, _, _)) in blocks.iter().enumerate() {
            if used[i] {
                pruned.push_str(full);
            }
        }
        self.statics = pruned;
    }

    fn collect_static_roots<'a>(&'a self, out: &mut Vec<&'a str>) {
        for list in [
            &self.enter_modifiers,
            &self.alternative_costs,
            &self.mandatory_additional_costs,
            &self.flashback_parts,
            &self.morph_parts,
            &self.additional_costs,
            &self.abilities,
        ] {
            out.extend(list.iter().map(String::as_str));
        }
        for opt in [
            &self.replicate,
            &self.disguise,
            &self.flashback,
            &self.madness,
            &self.foretell,
            &self.miracle,
            &self.dash,
            &self.blitz,
            &self.morph,
            &self.megamorph,
            &self.bestow,
            &self.prototype,
            &self.emerge,
            &self.entwine,
            &self.buyback,
            &self.multikicker,
            &self.disturb,
            &self.escape,
            &self.squad,
            &self.offspring,
            &self.behold_or_pay,
            &self.mutate,
            &self.plot,
            &self.casualty,
            &self.impending,
            &self.gift,
            &self.warp,
            &self.cost_reduction,
            &self.kicked_targets,
        ] {
            if let Some(s) = opt.as_deref() {
                out.push(s);
            }
        }
        if let Some(back) = self.back.as_deref() {
            back.collect_static_roots(out);
        }
    }

    fn iter_static_blocks(&self) -> impl Iterator<Item = (&str, &str, &str, &str)> {
        self.statics.split_inclusive(";\n\n").filter_map(|full| {
            let trimmed = full.strip_prefix("static ")?;
            let (name, rest) = trimmed.split_once(": ")?;
            let (ty, body_semi) = rest.split_once(" = ")?;
            let body = body_semi.strip_suffix(";\n\n")?;
            Some((full, name.trim(), ty.trim(), body))
        })
    }

    /// The name this card already gave `expr`, if it gave it one.
    ///
    /// Matched on the **whole** declaration rather than by searching for the
    /// expression anywhere in the text, because one filter is often a
    /// substring of another: `Filter::And(&[A, B])` sits inside
    /// `Filter::Or(&[Filter::And(&[A, B]), C])`, and a contains-test would
    /// hand back the name of a filter that says something else.
    ///
    /// The prefix has to match too. A name is what the card calls the
    /// thing, so `TARGET1` reused as the `CHECK` of a clause would read as
    /// an ability aiming at what it is asking about.
    fn declared(&self, prefix: &str, expr: &str) -> Option<String> {
        self.declared_with_type(prefix, "Filter", expr)
    }

    fn declared_with_type(&self, prefix: &str, want_ty: &str, expr: &str) -> Option<String> {
        self.iter_static_blocks().find_map(|(_, name, ty, body)| {
            (ty == want_ty && name.starts_with(prefix) && body == expr).then(|| name.to_string())
        })
    }
}

fn contains_ident(haystack: &str, ident: &str) -> bool {
    let bytes = haystack.as_bytes();
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    haystack.match_indices(ident).any(|(start, _)| {
        let before_ok = start == 0 || !is_ident(bytes[start - 1]);
        let end = start + ident.len();
        let after_ok = end == bytes.len() || !is_ident(bytes[end]);
        before_ok && after_ok
    })
}

#[cfg(test)]
mod tests {
    use super::CardBody;

    /// Two sentences about the same filter are one `static`, and two
    /// sentences about different ones are two — including the pair where
    /// one filter is spelled inside the other, which is where a reader that
    /// searched the text for the expression would go wrong.
    #[test]
    fn one_filter_is_named_once_however_often_the_card_says_it() {
        let inner = "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])";
        let outer = format!("Filter::Or(&[{inner}, Filter::LAND])");
        let mut body = CardBody::default();
        assert_eq!(body.filter_static("CHECK", inner), "CHECK1");
        assert_eq!(body.filter_static("CHECK", inner), "CHECK1", "the same one");
        assert_eq!(body.filter_static("CHECK", &outer), "CHECK2", "a wider one");
        assert_eq!(
            body.filter_static("TARGET", inner),
            "TARGET3",
            "the same expression under another prefix is another name"
        );
        assert_eq!(body.statics.matches("static ").count(), 3);
    }

    #[test]
    fn ability_static_emits_unique_ability_defs_alongside_filters() {
        let mut body = CardBody::default();
        assert_eq!(
            body.filter_static("CHECK", "Filter::And(&[Filter::CREATURE, Filter::TAPPED])"),
            "CHECK1"
        );
        assert_eq!(
            body.ability_static("GRANTED_ACTIVATED_", "activated!(Cost::TAP, &[])"),
            "GRANTED_ACTIVATED_2"
        );
        assert_eq!(
            body.ability_static("GRANTED_TRIGGER_", "triggered!(Trigger::ETB, &[])"),
            "GRANTED_TRIGGER_3"
        );
        assert!(
            body.statics
                .contains("static GRANTED_ACTIVATED_2: AbilityDef = activated!(Cost::TAP, &[]);")
        );
        assert!(
            body.statics
                .contains("static GRANTED_TRIGGER_3: AbilityDef = triggered!(Trigger::ETB, &[]);")
        );
    }

    #[test]
    fn ability_slice_static_dedups_and_prune_unused_statics_keeps_reachable_graph() {
        let mut body = CardBody::default();
        let ab = vec![
            "activated!(Cost::TAP, &[Effect::draw(1)])".to_string(),
            "triggered!(Trigger::ETB, &[Effect::gain_life(1)])".to_string(),
        ];
        assert_eq!(
            body.ability_slice_static("BACKUP_GRANTED", &ab),
            "BACKUP_GRANTED1"
        );
        assert_eq!(
            body.ability_slice_static("BACKUP_GRANTED", &ab),
            "BACKUP_GRANTED1"
        );
        assert_eq!(
            body.filter_static("TARGET", "Filter::And(&[Filter::CREATURE, Filter::TAPPED])"),
            "TARGET2"
        );
        let used_inner = body.filter_static(
            "FILTER",
            "Filter::And(&[Filter::ARTIFACT, Filter::ControlledByOpponent])",
        );
        let used_outer = body.filter_static(
            "TARGET",
            &format!("Filter::Or(&[{used_inner}, Filter::LAND])"),
        );
        body.statics.push_str(
            "static CHECK1: Filter = Filter::And(&[Filter::CREATURE, Filter::Attacking]);\n\n",
        );
        body.statics.push_str(
            "static CHECK10: Filter = Filter::And(&[Filter::ARTIFACT, Filter::Untapped]);\n\n",
        );
        body.statics.push_str(
            "static BACK5: Filter = Filter::And(&[Filter::ENCHANTMENT, Filter::ControlledByYou]);\n\n",
        );
        body.abilities.push(format!(
            "triggered!(Trigger::ETB, &[Effect::destroy(TargetSpec::ObjectOfEachOpponent(&{used_outer})), Effect::draw(1)], condition = Some(Condition::ControlCount(&CHECK10, 1)))"
        ));
        body.abilities.push("BACKUP_GRANTED1[0]".to_string());
        body.back = Some(Box::new(CardBody {
            abilities: vec![
                "activated!(Cost::TAP, &[Effect::destroy(TargetSpec::Object(&BACK5))])".to_string(),
            ],
            ..CardBody::default()
        }));

        body.prune_unused_statics();

        assert!(
            body.statics.contains("static BACKUP_GRANTED1:"),
            "{}",
            body.statics
        );
        assert!(
            !body.statics.contains("static TARGET2:"),
            "{}",
            body.statics
        );
        assert!(
            body.statics.contains(&format!("static {used_inner}:")),
            "{}",
            body.statics
        );
        assert!(
            body.statics.contains(&format!("static {used_outer}:")),
            "{}",
            body.statics
        );
        assert!(!body.statics.contains("static CHECK1:"), "{}", body.statics);
        assert!(body.statics.contains("static CHECK10:"), "{}", body.statics);
        assert!(body.statics.contains("static BACK5:"), "{}", body.statics);
    }
}
