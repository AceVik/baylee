//! Building a menu: one entry per thing a seat may answer, each said in
//! words the model can pick from.

use super::*;

impl Builder<'_, '_> {
    pub(super) fn line(&mut self, line: impl AsRef<str>) {
        self.text.push_str(line.as_ref());
        self.text.push('\n');
    }

    pub(super) fn option(&mut self, id: impl Into<String>, label: impl Into<String>, act: Act) {
        self.options.push(Choice {
            id: id.into(),
            alias: None,
            label: label.into(),
            act,
            card: None,
        });
    }

    /// Lists the options as `  a1  label`.
    pub(super) fn list_options(&mut self) {
        let width = self.options.iter().map(|c| c.id.len()).max().unwrap_or(1);
        let lines: Vec<String> = self
            .options
            .iter()
            .map(|c| format!("  {:width$}  {}", c.id, c.label))
            .collect();
        for line in lines {
            self.line(line);
        }
    }

    /// The shape of the answer, where the question carries one (how many
    /// ids, which field): said to every model.
    pub(super) fn answer(&mut self, shape: &str) {
        if self.style.spell_answer {
            let ask = self.request.question;
            self.line(format!("Answer: ask=\"q{ask}\", {shape}"));
        } else {
            self.line(format!("Answer: {shape}"));
        }
    }

    /// The shape of an answer that picks one of the options listed: said
    /// only to a model no schema holds ([`Style::spell_answer`]); the
    /// schema and the instructions say it to the others.
    pub(super) fn answer_pick(&mut self, shape: &str) {
        if self.style.spell_answer {
            let ask = self.request.question;
            self.line(format!("Answer: ask=\"q{ask}\", {shape}"));
        }
    }

    /// The shape of an answer that picks `min` to `max` ids from a list.
    pub(super) fn answer_ids(&mut self, min: usize, max: usize) {
        if (min, max) == (1, 1) {
            self.answer_pick(&pick_shape(min, max));
        } else {
            self.answer(&pick_shape(min, max));
        }
    }

    #[allow(clippy::too_many_lines)] // one arm per question, and no `_`
    pub(super) fn write(&mut self) {
        let pending = self.request.pending.clone();
        let resource = baylee_client_core::decision::resource_player(self.table.view);
        if resource != self.table.view.seat {
            self.line(format!(
                "You decide for {}. That player's resources are used.",
                self.table.player(resource)
            ));
        }
        match &pending {
            Pending::ChooseManaAbility {
                choice,
                options,
                player,
            } => {
                self.line(format!(
                    "QUESTION: {} must activate a land mana ability. No pass is legal.",
                    self.table.player(*player)
                ));
                for (index, option) in options.iter().enumerate() {
                    self.option(
                        format!("mana{index}"),
                        option.ability_index.map_or_else(
                            || format!("Tap {} for mana", self.table.named(option.source.object)),
                            |index| self.ability_label(option.source.object, index),
                        ),
                        Act::Now(PlayerAction::ChooseManaAbility {
                            choice: *choice,
                            source: option.source,
                            ability_index: option.ability_index,
                        }),
                    );
                }
                self.list_options();
                self.answer_pick("pick=[one mana ability id]");
            }
            Pending::ChooseDamageSource {
                choice, options, ..
            } => {
                self.line("QUESTION: Choose one exact damage source. This does not target it.");
                for &source in options {
                    self.option(
                        format!(
                            "source{}-{}-{}",
                            choice.get(),
                            source.object,
                            source.version
                        ),
                        baylee_client_core::source_choice::label_named(
                            baylee_client_core::Lang::En,
                            source,
                            Some(self.table.view),
                            &|player| self.table.player(player),
                        ),
                        Act::Now(PlayerAction::ChooseDamageSource {
                            choice: *choice,
                            source,
                        }),
                    );
                }
                self.list_options();
                self.answer_pick("pick=[one source id]");
            }
            Pending::ChooseDamageEffect {
                choice,
                damage,
                options,
                ..
            } => {
                self.line("QUESTION: Choose which replacement or prevention effect applies next.");
                for effect in options {
                    self.option(
                        format!("damage{}-{}-{}", choice.batch, choice.step, effect.id),
                        self.damage_effect(effect, damage),
                        Act::Now(PlayerAction::ChooseDamageEffect {
                            choice: *choice,
                            effect: effect.id,
                        }),
                    );
                }
                self.list_options();
                self.answer_pick("pick=[one damage effect id]");
            }
            Pending::AllocatePrevention {
                effect,
                damage,
                total,
                ..
            } => {
                let what = match effect.kind {
                    baylee_engine::choice::DamageEffectKind::RemoveCounter { .. } => {
                        "counter removals"
                    }
                    baylee_engine::choice::DamageEffectKind::RedirectNext { .. } => {
                        "points of redirected damage"
                    }
                    _ => "points of prevention",
                };
                self.line(format!("QUESTION: Allocate exactly {total} {what} among the damage parts. Each share is 0 through that part's amount. Omitted parts receive zero."));
                self.line(self.damage_effect(effect, &[]));
                for part in damage {
                    let label = baylee_client_core::damage::part_label(
                        baylee_client_core::Lang::En,
                        part,
                        &|target| self.damage_target(target),
                    );
                    self.line(format!("  d{}: {label}", part.id));
                }
                self.ask = Ask::Prevention {
                    pending: Box::new(pending.clone()),
                };
                self.answer("prevention=[{part: \"d<id>\", amount: <integer>}, ...]");
            }
            Pending::Priority { legal, .. } => self.priority(legal),
            Pending::Mulligan {
                taken,
                next_is_free,
                can_take,
                ..
            } => self.mulligan(*taken, *next_is_free, *can_take),
            Pending::MulliganBottom { count, .. } => {
                self.line(format!(
                    "QUESTION: You kept. Put {} from your hand on the bottom of your library.",
                    words::count(usize::from(*count), "card")
                ));
                self.hand_objects(usize::from(*count));
            }
            Pending::DiscardChoice { count, .. } => {
                self.line(format!(
                    "QUESTION: Your hand is over its maximum size. Discard {}.",
                    words::count(usize::from(*count), "card")
                ));
                self.hand_objects(usize::from(*count));
            }
            Pending::LegendChoice { options, .. } => {
                self.line(
                    "QUESTION: Legend rule (CR 704.5j): you control legendary permanents with \
                     the same name. Choose the one to keep; the rest go to the graveyard.",
                );
                self.objects(options, 1, 1);
            }
            Pending::ChooseCards {
                options,
                min,
                max,
                prompt,
                total,
                ..
            } => {
                let question =
                    if let baylee_engine::choice::ChoicePrompt::SacrificeFor { player } = prompt {
                        format!(
                            "Choose the permanents that {} sacrifices",
                            self.table.player(*player)
                        )
                    } else {
                        words::choice_prompt(*prompt)
                    };
                self.line(format!("QUESTION: {question}."));
                if let Some(total) = total {
                    self.card_total(total);
                }
                self.objects_weighted(
                    options,
                    usize::from(*min),
                    usize::from(*max),
                    total.as_ref(),
                );
            }
            Pending::ChooseTargets {
                options,
                player_options,
                min,
                max,
                reason,
                ..
            } => self.targets(
                options,
                player_options,
                usize::try_from(*min).unwrap_or(usize::MAX),
                usize::try_from(*max).unwrap_or(usize::MAX),
                *reason,
            ),
            Pending::ChooseSubtype { options, .. } => self.subtype(options),
            Pending::ChooseCardName { .. } => {
                self.line("QUESTION: Choose a card name: any card's English name (a single face's name is fine).");
                self.ask = Ask::CardName;
                self.answer("name=\"<the card's name>\"");
            }
            Pending::ChooseColor { options, .. } => self.color(options),
            Pending::YesNo { prompt, source, .. } => self.yes_no(prompt, *source),
            Pending::ChooseCastMode {
                object, options, ..
            } => self.cast_mode(*object, options),
            Pending::ChooseNumber {
                min, max, reason, ..
            } => self.number(*min, *max, reason),
            Pending::ChoosePlayer { options, .. } => {
                self.line("QUESTION: Choose a player.");
                for &p in options {
                    let label = self.player_label(p);
                    self.option(
                        Table::player_id(p),
                        label,
                        Act::Now(PlayerAction::ChoosePlayer(p)),
                    );
                }
                self.list_options();
                self.answer_pick("pick=[one player id]");
            }
            Pending::Arrange {
                cards,
                piles,
                prompt,
                ..
            } => self.arrange(cards, piles, *prompt),
            Pending::ChoosePile { piles, .. } => {
                self.line(
                    "QUESTION: Choose one pile: it goes into your hand, and the other pile \
                     goes into the graveyard.",
                );
                for (i, pile) in piles.iter().enumerate() {
                    let names: Vec<String> = pile.iter().map(|&o| self.table.named(o)).collect();
                    let label = if names.is_empty() {
                        "(an empty pile)".into()
                    } else {
                        names.join(", ")
                    };
                    self.option(
                        format!("pile{}", i + 1),
                        label,
                        Act::Now(PlayerAction::ChooseMode(i)),
                    );
                }
                self.list_options();
                self.answer_pick("pick=[one pile id]");
            }
            Pending::ChooseAttackers {
                attackers,
                defenders,
                ..
            } => self.attackers(attackers, defenders),
            Pending::ChooseBlockers {
                attacker,
                blockers,
                bounds,
                ..
            } => self.blockers(*attacker, blockers, bounds),
            Pending::GameOver(_) => {
                self.line("The game is over.");
                self.ask = Ask::Nothing;
            }
        }
    }

    /// Adds the next `a` option: `a1`, `a2`, … in the order offered.
    pub(super) fn offer(&mut self, label: String, act: Act, card: Option<CardIndex>) {
        let id = format!("a{}", self.options.len() + 1);
        self.option(id, label, act);
        self.options.last_mut().expect("just pushed").card = card;
    }

    /// The payment the seat agreed to, as one option: the taps that pay it
    /// and the pass that spends them (CR 605.3a).
    pub(super) fn pay_owed(&mut self, owed: ManaCost, pool: &ManaPoolView, legal: &LegalActions) {
        let sources = baylee_ai::mana_sources(self.table.view, legal);
        match manaplan::plan(&owed, pool, &sources) {
            Some(plan) if plan.steps.is_empty() => self.offer(
                format!("Pay {owed} from your pool"),
                Act::Now(PlayerAction::PassPriority),
                None,
            ),
            Some(plan) => {
                let label = format!("Pay {owed} ({})", self.taps(&plan.steps));
                let act = Act::Taps {
                    steps: plan.steps,
                    then: PlayerAction::PassPriority,
                };
                self.offer(label, act, None);
            }
            None => self.line("You cannot pay it from what you have untapped."),
        }
    }

    /// Every play a priority offers but the pass: lands, spells paid from
    /// the pool, spells the untapped sources reach, suspends, activations.
    pub(super) fn plays(&mut self, legal: &LegalActions) {
        let table = self.table;
        let view = table.view;
        for &land in &legal.lands {
            let label = format!("Play land {}", table.named(land));
            self.offer(label, Act::Now(PlayerAction::PlayLand { card: land }), None);
        }
        for &card in &legal.castable {
            let label = format!(
                "Cast {}{} (paid from your pool)",
                table.named(card),
                self.from(card)
            );
            let index = self.card_of(card);
            self.offer(label, Act::Now(PlayerAction::CastSpell { card }), index);
        }
        for reach in reachable(view, legal) {
            let label = self.reach_label(&reach);
            let act = Act::Taps {
                steps: reach.plan.steps.clone(),
                then: PlayerAction::CastSpell { card: reach.object },
            };
            self.offer(label, act, Some(reach.card.index));
        }
        for &card in &legal.suspendable {
            let label = format!("Suspend {}", table.named(card));
            self.offer(label, Act::Now(PlayerAction::Suspend { card }), None);
        }
        for &(source, index) in &legal.abilities {
            if baylee_ai::only_makes_mana(view, source, index) {
                continue;
            }
            let label = self.ability_label(source, index);
            let act = Act::Now(PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            });
            let card = self.card_of(source);
            self.offer(label, act, card);
        }
    }

    pub(super) fn granted_actions(&mut self, legal: &LegalActions) {
        use baylee_cards_dsl::SpecialActionCost;
        use baylee_engine::choice::GrantedActionKind;
        for offer in &legal.granted_actions {
            let cost = match offer.cost {
                SpecialActionCost::Life(amount) => format!("pay {amount} life"),
                SpecialActionCost::Mana(mana) => format!("pay {mana}"),
            };
            let effect = match offer.effect {
                GrantedActionKind::AddMana { color, amount } => {
                    format!("add {amount} {color:?} mana")
                }
                GrantedActionKind::PreventNextDamage { target, amount } => {
                    let recipient = match target {
                        baylee_core::ids::TargetRef::Object(source) => {
                            self.table.named_target(source)
                        }
                        baylee_core::ids::TargetRef::Player(player) => self.table.player(player),
                    };
                    format!("prevent the next {amount} damage to {recipient}")
                }
            };
            self.offer(
                format!("Until end of turn — {cost}: {effect} (one use)"),
                Act::Now(PlayerAction::TakeGrantedAction { id: offer.id }),
                offer.ability.map(|a| a.card),
            );
        }
    }

    pub(super) fn priority(&mut self, legal: &LegalActions) {
        let table = self.table;
        let view = table.view;
        let mut intro = String::from("QUESTION: You have priority.");
        if self.request.continuing {
            intro.push_str(" You are in the middle of paying for something.");
        }
        self.line(intro);
        let pool = view
            .seat(baylee_client_core::decision::resource_player(view))
            .map(|s| s.mana_pool)
            .unwrap_or_default();
        let owed = view.owed.filter(|_| view.awaiting == Some(view.seat));
        if let Some(payment) = owed {
            match payment {
                ManaPayment::Fixed(cost) => self.line(format!(
                    "You owe {cost}: a payment you agreed to. Pay it, or pass and leave it unpaid \
                     (what it was for is lost)."
                )),
                ManaPayment::AnyAmount { preventable_damage } => self.line(format!(
                    "You may generate mana to prevent up to {preventable_damage} damage. \
                     Pass to choose how much mana to pay, including zero."
                )),
            }
        } else if !pool.is_empty() {
            self.line(
                "Mana is in your pool (see your seat line); unspent mana empties at the end of \
                 this step (CR 500.5).",
            );
        }
        if let Some(payment) = owed {
            match payment {
                ManaPayment::Fixed(cost) => self.pay_owed(cost, &pool, legal),
                ManaPayment::AnyAmount { .. } => self.optional_payment_sources(legal),
            }
        }
        self.granted_actions(legal);
        self.plays(legal);
        if legal.can_pass {
            let label = if matches!(owed, Some(ManaPayment::AnyAmount { .. })) {
                "Finish generating mana and choose the payment amount".into()
            } else {
                self.pass_label(owed.is_some())
            };
            self.option("p", label, Act::Now(PlayerAction::PassPriority));
            self.options.last_mut().expect("just pushed").alias = Some("pass".into());
        }
        self.line("Options:");
        self.list_options();
        self.answer_pick("pick=[one option id]");
    }

    /// A variable payment has no fixed plan: every legal mana ability is usable.
    pub(super) fn optional_payment_sources(&mut self, legal: &LegalActions) {
        for &source in &legal.mana_abilities {
            self.offer(
                format!("Generate mana with {}", self.table.named(source)),
                Act::Now(PlayerAction::ActivateManaAbility { source }),
                self.card_of(source),
            );
        }
        for &(source, ability_index) in &legal.abilities {
            if baylee_ai::only_makes_mana(self.table.view, source, ability_index) {
                self.offer(
                    self.ability_label(source, ability_index),
                    Act::Now(PlayerAction::ActivateAbility {
                        source,
                        ability_index,
                    }),
                    self.card_of(source),
                );
            }
        }
    }

    /// The taps of a plan: "taps Forest #22, Mountain #23".
    pub(super) fn taps(&self, steps: &[manaplan::Step]) -> String {
        let named: Vec<String> = steps
            .iter()
            .map(|step| {
                let mut name = self.table.named(step.source);
                if let Some(color) = step.color {
                    let _ = write!(name, " for {{{}}}", words::color_letter(color));
                }
                name
            })
            .collect();
        format!("taps {}", named.join(", "))
    }

    pub(super) fn reach_label(&self, reach: &Reach) -> String {
        let table = self.table;
        let x = baylee_cards::by_index(reach.card.index)
            .and_then(|def| def.faces.get(usize::from(reach.card.face)))
            .is_some_and(|face| face.mana_cost.to_string().contains("{X}"));
        let mut label = match reach.from {
            ReachFrom::Hand => format!("Cast {} {}", table.named(reach.object), reach.plan.cost),
            ReachFrom::CommandZone => format!(
                "Cast your commander {} from the command zone for {} (tax included)",
                table.named(reach.object),
                reach.plan.cost
            ),
        };
        let _ = write!(label, " ({}", self.taps(&reach.plan.steps));
        if x {
            label.push_str("; X is 0 in this count, and you choose X when you cast it");
        }
        label.push(')');
        label
    }

    /// Where a castable card is cast from, when not from the hand.
    pub(super) fn from(&self, card: ObjectId) -> String {
        let view = self.table.view;
        if view.hand.iter().any(|c| c.id == card) {
            return String::new();
        }
        if view.graveyards.iter().flatten().any(|o| o.id == card) {
            let flashback = view
                .object(card)
                .and_then(|o| o.flashback)
                .map(|cost| format!(", flashback {cost}"))
                .unwrap_or_default();
            return format!(" from a graveyard{flashback}");
        }
        if view.exile.iter().flatten().any(|o| o.id == card) {
            return " from exile".into();
        }
        if view.command.iter().flatten().any(|o| o.id == card) {
            return " from the command zone".into();
        }
        String::new()
    }

    /// The card behind an object the seat may cast or activate.
    pub(super) fn card_of(&self, id: ObjectId) -> Option<CardIndex> {
        let view = self.table.view;
        view.object(id)
            .and_then(|o| o.card)
            .or_else(|| view.hand.iter().find(|c| c.id == id).map(|c| c.card))
            .map(|c| c.index)
    }

    pub(super) fn ability_label(&self, source: ObjectId, index: u32) -> String {
        use baylee_engine::choice::{PREPARED_CAST, TURN_FACE_UP, door_to_unlock, granted_slot};
        let table = self.table;
        let named = table.named(source);
        if granted_slot(index).is_some() {
            let makes_mana = table
                .view
                .object(source)
                .and_then(|o| o.granted_mana.as_ref())
                .is_some();
            return if makes_mana {
                format!("Activate {named}: a mana ability it was granted")
            } else {
                format!("Activate {named}: an ability it was granted")
            };
        }
        if index == PREPARED_CAST {
            return format!("Cast the spell {named} prepared");
        }
        if index == TURN_FACE_UP {
            return format!("Turn {named} face up");
        }
        if let Some(half) = door_to_unlock(index) {
            let side = if half == 0 { "left" } else { "right" };
            return format!("Unlock the {side} door of {named}");
        }
        let in_hand = table.view.hand.iter().any(|c| c.id == source);
        let place = if in_hand { " (from your hand)" } else { "" };
        if let Some(sentence) = ability_sentence(table, source, index) {
            return format!("Activate {named}{place}: \"{sentence}\"");
        }
        let mana = ability_def(table, source, index).is_some_and(|def| {
            matches!(
                def,
                AbilityDef::Activated {
                    mana_ability: true,
                    ..
                } | AbilityDef::ActivatedConditional {
                    mana_ability: true,
                    ..
                }
            )
        });
        let what = if mana {
            "its mana ability"
        } else {
            "its ability"
        };
        format!("Activate {named}{place}: {what} (number {})", index + 1)
    }

    pub(super) fn pass_label(&self, owed: bool) -> String {
        let table = self.table;
        let view = table.view;
        if owed {
            return "Pass priority without paying the rest".into();
        }
        if let Some(top) = view.top_of_stack() {
            return format!(
                "Pass priority (if every other player passes too, {} resolves)",
                board::stack_line(table, top)
                    .split(" · ")
                    .next()
                    .unwrap_or_default()
            );
        }
        let mine = view.active == view.seat;
        let next = match (view.phase, view.step) {
            (baylee_view::Phase::FirstMain, _) if mine => {
                "moves on to combat, where you declare attackers"
            }
            (baylee_view::Phase::SecondMain, _) if mine => "moves on to your end step",
            (_, baylee_view::Step::End) if mine => "your turn ends",
            (_, baylee_view::Step::Cleanup) => "the turn ends",
            _ if mine => "your turn goes on",
            _ => "the game goes on",
        };
        format!("Pass priority ({next})")
    }

    pub(super) fn mulligan(&mut self, taken: u8, free: bool, can_take: bool) {
        let mut intro = format!(
            "QUESTION: Keep this opening hand, or take a mulligan? You have taken {}",
            words::count(usize::from(taken), "mulligan")
        );
        if free {
            intro.push_str("; the next one is free");
        }
        intro.push_str(
            ". When you keep, you put one card on the bottom of your library for each \
             mulligan that was not free (CR 103.5).",
        );
        self.line(intro);
        self.option(
            "keep",
            "Keep this hand",
            Act::Now(PlayerAction::MulliganKeep),
        );
        if can_take {
            self.option(
                "mulligan",
                "Shuffle this hand away and draw a new one",
                Act::Now(PlayerAction::MulliganTake),
            );
        }
        self.line("Options:");
        self.list_options();
        self.answer_pick("pick=[one option id]");
    }

    /// A choice of `count` cards from the hand.
    pub(super) fn hand_objects(&mut self, count: usize) {
        let hand: Vec<ObjectId> = self.table.view.hand.iter().map(|c| c.id).collect();
        self.line("Choose from your hand:");
        let lines: Vec<String> = self
            .table
            .view
            .hand
            .iter()
            .map(|card| format!("  {}", board::hand_line(self.table, card)))
            .collect();
        for line in lines {
            self.line(line);
        }
        self.ask = Ask::Objects {
            from: hand,
            min: count,
            max: count,
        };
        self.answer_ids(count, count);
    }

    pub(super) fn objects(&mut self, options: &[ObjectId], min: usize, max: usize) {
        self.objects_weighted(options, min, max, None);
    }

    pub(super) fn objects_weighted(
        &mut self,
        options: &[ObjectId],
        min: usize,
        max: usize,
        total: Option<&CardTotal>,
    ) {
        self.line("Choose from:");
        for (i, &id) in options.iter().enumerate() {
            let mut line = format!("  {}", self.describe(id));
            if let Some(weight) = total.and_then(|t| t.weights.get(i)) {
                let _ = write!(
                    line,
                    " [{} {weight}]",
                    words::measure(total.map_or(baylee_engine::choice::Measure::Power, |t| t.of,))
                );
            }
            self.line(line);
        }
        self.ask = Ask::Objects {
            from: options.to_vec(),
            min,
            max,
        };
        self.answer_ids(min, max);
    }

    pub(super) fn card_total(&mut self, total: &CardTotal) {
        let of = words::measure(total.of);
        match (total.at_least, total.at_most) {
            (Some(least), Some(most)) => self.line(format!(
                "The chosen cards' total {of} must be from {least} to {most}."
            )),
            (Some(least), None) => {
                self.line(format!(
                    "The chosen cards' total {of} must be {least} or more."
                ));
            }
            (None, Some(most)) => {
                self.line(format!(
                    "The chosen cards' total {of} must be {most} or less."
                ));
            }
            (None, None) => {}
        }
    }

    /// One object as a choice line: the board's line for a permanent, the
    /// stack's for a spell, else its name and where it is.
    pub(super) fn describe(&self, id: ObjectId) -> String {
        let table = self.table;
        let view = table.view;
        if let Some(object) = view.battlefield.iter().find(|o| o.id == id) {
            return format!(
                "{} ({})",
                board::permanent(table, object),
                if object.controller == view.seat {
                    "yours".to_string()
                } else {
                    table.whose(object.controller)
                }
            );
        }
        if let Some(object) = view.stack.iter().find(|o| o.id == id) {
            return board::stack_line(table, object);
        }
        if let Some(card) = view.hand.iter().find(|c| c.id == id) {
            return format!("{} (in your hand)", board::hand_line(table, card));
        }
        match view.object(id) {
            Some(object) => format!("{} {} ({})", tag(id), plain(object), board::zone(table, id)),
            None => format!("{} a card you cannot see", tag(id)),
        }
    }

    pub(super) fn player_label(&self, p: PlayerId) -> String {
        let table = self.table;
        let life = table
            .view
            .seat(p)
            .map_or_else(String::new, |s| format!(", {} life", s.life));
        if p == table.me() {
            format!("you{life}")
        } else if table.ally(p) {
            format!("your teammate{life}")
        } else {
            format!("opponent{life}")
        }
    }

    pub(super) fn targets(
        &mut self,
        options: &[ObjectId],
        players: &[PlayerId],
        min: usize,
        max: usize,
        reason: TargetPrompt,
    ) {
        let table = self.table;
        let view = table.view;
        let source = view.targeting.as_ref();
        let what = source.map_or_else(
            || "the spell or ability".to_string(),
            |t| {
                format!(
                    "{} {}",
                    super::super::object_name(&t.source),
                    tag(t.source.id)
                )
            },
        );
        match reason {
            TargetPrompt::Retarget { current, index, of } => {
                let current = match current {
                    baylee_engine::choice::TargetRef::Object(source) => table.named_target(source),
                    baylee_engine::choice::TargetRef::Player(id) => table.player(id),
                };
                let keep = if min == 0 { " Choose no targets to keep this target." } else { "" };
                self.line(format!("QUESTION: Target {} of {of} for {what} is {current}. Choose a new target.{keep}", u64::from(index) + 1));
            },
            TargetPrompt::Convoke => self.line(format!(
                "QUESTION: Convoke: choose creatures to tap to help pay for {what} (each pays {{1}} \
                 or one mana of its colour)."
            )),
            TargetPrompt::Targets => {
                let mut line = format!("QUESTION: Choose targets for {what}");
                if source.is_some_and(|t| t.second) {
                    line.push_str(" (the second group of targets)");
                }
                match source.and_then(targeting_sentence) {
                    Some(sentence) => {
                        let _ = write!(line, ": \"{sentence}\"");
                    }
                    None => line.push('.'),
                }
                self.line(line);
            }
        }
        if let Some(t) = source.filter(|t| t.batch_count > 1) {
            self.line(format!(
                "{} copies of this trigger are waiting; this answers the first.",
                t.batch_count
            ));
        }
        self.line(format!(
            "Choose {} (objects by #id, players by P-id):",
            how_many(min, max)
        ));
        for &id in options {
            let line = format!("  {}", self.describe(id));
            self.line(line);
        }
        for &p in players {
            let line = format!("  {} = {}", Table::player_id(p), self.player_label(p));
            self.line(line);
        }
        self.ask = Ask::Targets {
            objects: options.to_vec(),
            players: players.to_vec(),
            min,
            max,
        };
        self.answer_ids(min, max);
    }

    pub(super) fn subtype(&mut self, options: &[SubtypeId]) {
        let names: Vec<&str> = options
            .iter()
            .filter_map(|&s| baylee_core::generated::subtypes::name(s))
            .collect();
        if names.len() <= 40 {
            self.line(format!("QUESTION: Choose a type: {}.", names.join(", ")));
        } else {
            let mine = own_subtypes(self.table, options);
            let mut line = format!(
                "QUESTION: Choose a type, by name: any of {} types is allowed.",
                names.len()
            );
            if !mine.is_empty() {
                let _ = write!(line, " Types among your cards: {}.", mine.join(", "));
            }
            self.line(line);
        }
        self.ask = Ask::Subtype {
            options: options.to_vec(),
        };
        self.answer("name=\"<the type>\"");
    }

    pub(super) fn color(&mut self, options: &[ManaColor]) {
        let intro = if self.request.continuing {
            "QUESTION: Choose the color of the mana you are making."
        } else {
            "QUESTION: Choose a color."
        };
        self.line(intro);
        for &color in options {
            self.option(
                words::color_letter(color),
                words::color_name(color),
                Act::Now(PlayerAction::ChooseColor(color)),
            );
            self.options.last_mut().expect("just pushed").alias =
                Some(words::color_name(color).into());
        }
        self.line("Options:");
        self.list_options();
        self.answer_pick("pick=[one option id]");
    }

    pub(super) fn yes_no(&mut self, prompt: &YesNoPrompt, source: Option<AbilityRef>) {
        let table = self.table;
        let about = source.and_then(ability_ref_sentence);
        let (question, yes, no) = yes_no_words(table, prompt);
        self.line(format!("QUESTION: {question}"));
        let kicker = matches!(prompt, YesNoPrompt::Kicker);
        if let Some(about) = about {
            self.line(format!("The ability asking: \"{about}\""));
        } else if kicker && let Some((name, sentence)) = source.and_then(additional_cost) {
            self.line(format!("The spell: {name}: \"{sentence}\""));
        }
        if kicker {
            // Said here because the table does not say it later: a cast
            // it cannot finish is taken back without a word, and the
            // same question comes again.
            self.line(
                "Say yes only if you can pay the whole cost with it now: a cast whose cost \
                 cannot be paid is taken back, and you get priority again (CR 601.2h, 732.1, \
                 732.2).",
            );
        }
        self.option(
            "y",
            format!("Yes: {yes}"),
            Act::Now(PlayerAction::YesNo(true)),
        );
        self.options.last_mut().expect("just pushed").alias = Some("yes".into());
        self.option(
            "n",
            format!("No: {no}"),
            Act::Now(PlayerAction::YesNo(false)),
        );
        self.options.last_mut().expect("just pushed").alias = Some("no".into());
        self.line("Options:");
        self.list_options();
        self.answer_pick("pick=[one option id]");
    }
}

impl Builder<'_, '_> {
    pub(super) fn cast_mode(&mut self, object: ObjectId, options: &[CastModeDesc]) {
        let table = self.table;
        let card = table.view.object(object).and_then(|o| o.card).or_else(|| {
            table
                .view
                .hand
                .iter()
                .find(|c| c.id == object)
                .map(|c| c.card)
        });
        self.line(format!(
            "QUESTION: How do you cast {}?",
            table.named(object)
        ));
        for (at, option) in options.iter().enumerate() {
            let label = cast_mode_label(card, option);
            self.option(
                format!("m{}", at + 1),
                label,
                Act::Now(PlayerAction::ChooseMode(at)),
            );
        }
        self.line("Options:");
        self.list_options();
        self.answer_pick("pick=[one option id]");
    }

    pub(super) fn number(&mut self, min: u32, max: u32, reason: &NumberPrompt) {
        let table = self.table;
        let question = match reason {
            NumberPrompt::TextReplacement { kind, target } => {
                let words =
                    baylee_client_core::text_choice::words(*kind, baylee_client_core::Lang::En);
                let pairs = (0..20)
                    .filter_map(|n| {
                        let pair =
                            baylee_engine::text_changes::TextReplacement::from_choice(*kind, n)?;
                        Some(format!(
                            "{n}: {} → {}",
                            words[usize::from(pair.from)],
                            words[usize::from(pair.to)]
                        ))
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!(
                    "Change words on {}. Choose one ordered pair: {pairs}",
                    table.named_target(*target)
                )
            }
            NumberPrompt::Counters { target, kind } => format!(
                "How many {} on {}?",
                baylee_client_core::interaction::counter_label(
                    *kind,
                    baylee_client_core::i18n::Lang::En
                ),
                table.named(*target),
            ),
            NumberPrompt::X => "Choose the value of X".to_string(),
            NumberPrompt::ManaPayment { preventable_damage } => format!(
                "How much mana will you pay? Each mana prevents one of up to \
                 {preventable_damage} damage; paying more is allowed but prevents no more"
            ),
            NumberPrompt::Replicate { cost } => {
                format!("How many times do you replicate it? Each copy costs {cost} more")
            }
            NumberPrompt::CombatDamage {
                source,
                recipient,
                index,
                of,
                left,
            } => format!(
                "Divide {}'s combat damage: how much goes to {} (creature {} of {of})? {left} is left to divide",
                table.named(*source),
                table.named(*recipient),
                u16::from(*index) + 1
            ),
            NumberPrompt::DivideDamage {
                target,
                index,
                of,
                left,
            } => format!(
                "Divide the damage: how much goes to {} (target {} of {of})? {left} is left to divide",
                table.named(*target),
                u16::from(*index) + 1
            ),
        };
        self.line(format!("QUESTION: {question} (from {min} to {max})."));
        self.ask = Ask::Number { min, max };
        self.answer(&format!("number=<an integer from {min} to {max}>"));
    }

    pub(super) fn arrange(
        &mut self,
        cards: &[ObjectId],
        piles: &[ArrangePile],
        prompt: ArrangePrompt,
    ) {
        let table = self.table;
        let n = cards.len();
        let intro = match prompt {
            ArrangePrompt::Scry => format!(
                "QUESTION: Scry {n} (CR 701.22a): put any number of these cards on the bottom \
                 of your library and the rest on top, in any order."
            ),
            ArrangePrompt::Surveil => format!(
                "QUESTION: Surveil {n} (CR 701.25a): put any number of these cards into your \
                 graveyard and the rest on top of your library, in any order."
            ),
            ArrangePrompt::Order => "QUESTION: Put these cards in order.".into(),
            ArrangePrompt::DiscardToLibrary => format!(
                "QUESTION: You are discarding {n} card(s). Library of Leng lets you put any \
                 of them on top of your library instead of into your graveyard, in any order; \
                 each is still discarded."
            ),
        };
        self.line(intro);
        let named: Vec<String> = cards.iter().map(|&c| table.named(c)).collect();
        self.line(format!("Cards: {}", named.join(", ")));
        self.line("Piles:");
        for (i, pile) in piles.iter().enumerate() {
            let place = match pile.place {
                ArrangePlace::LibraryTop => "the top of your library",
                ArrangePlace::LibraryBottom => "the bottom of your library",
                ArrangePlace::Graveyard => "your graveyard",
            };
            let order = if pile.ordered {
                "; list them from the top down"
            } else {
                ""
            };
            self.line(format!(
                "  pile {}: {place} ({} cards{order})",
                i + 1,
                how_many(pile.min as usize, pile.max as usize)
            ));
        }
        self.ask = Ask::Arrange {
            cards: cards.to_vec(),
            piles: piles.to_vec(),
        };
        let shape: Vec<String> = (1..=piles.len())
            .map(|i| format!("[ids of pile {i}]"))
            .collect();
        self.answer(&format!(
            "piles=[{}], every card in exactly one pile",
            shape.join(", ")
        ));
    }

    pub(super) fn attackers(&mut self, attackers: &[ObjectId], defenders: &[Defender]) {
        let table = self.table;
        self.line("QUESTION: Declare attackers.");
        self.line("Creatures that can attack:");
        for &id in attackers {
            let line = format!("  {}", self.describe(id));
            self.line(line);
        }
        self.line("Defenders:");
        for &defender in defenders {
            let line = match defender {
                Defender::Player(p) => {
                    format!("  {} = {}", Table::player_id(p), self.player_label(p))
                }
                Defender::Planeswalker(id) => format!("  {}", self.describe(id)),
            };
            self.line(line);
        }
        let _ = table;
        self.ask = Ask::Attack {
            attackers: attackers.to_vec(),
            defenders: defenders.to_vec(),
        };
        self.answer(
            "attacks=[{\"attacker\": id, \"at\": defender id}, …]; attacks=[] attacks with nothing",
        );
    }

    pub(super) fn blockers(
        &mut self,
        attacker: PlayerId,
        options: &[BlockOption],
        bounds: &[AttackerBound],
    ) {
        let table = self.table;
        self.line(format!(
            "QUESTION: Declare blockers against {} attack.",
            table.whose(attacker)
        ));
        self.line("Attacking:");
        let attacks: Vec<String> = table
            .view
            .combat
            .attackers
            .iter()
            .map(|a| {
                let object = table.view.object(a.creature);
                let line = object.map_or_else(|| tag(a.creature), |o| board::permanent(table, o));
                format!("  {line}")
            })
            .collect();
        for line in attacks {
            self.line(line);
        }
        self.line("Your creatures that can block, and what each may block:");
        for option in options {
            let blocker = table.view.object(option.blocker);
            let head = blocker.map_or_else(
                || tag(option.blocker),
                |o| {
                    let stats = board::stats(o).map(|s| format!(" {s}")).unwrap_or_default();
                    format!("{} {}{stats}", tag(o.id), super::super::object_name(o))
                },
            );
            let may: Vec<String> = option.attackers.iter().map(|&a| tag(a)).collect();
            self.line(format!("  {head} can block {}", may.join(", ")));
        }
        for bound in bounds {
            let name = table.named(bound.attacker);
            if bound.min_blockers > 1 {
                self.line(format!(
                    "  {name} can't be blocked except by {} or more creatures (CR 702.111b for menace).",
                    bound.min_blockers
                ));
            }
            if bound.max_blockers < u32::MAX {
                self.line(format!(
                    "  {name} can be blocked by at most {}.",
                    words::count(bound.max_blockers as usize, "creature")
                ));
            }
        }
        self.ask = Ask::Block {
            options: options.to_vec(),
        };
        self.answer(
            "blocks=[{\"blocker\": id, \"attacker\": id}, …]; blocks=[] blocks nothing. Each \
             blocker blocks one attacker",
        );
    }
}

impl Builder<'_, '_> {
    pub(super) fn damage_target(&self, target: baylee_engine::event::DamageTarget) -> String {
        match target {
            baylee_engine::event::DamageTarget::Object(id) => self.table.named(id),
            baylee_engine::event::DamageTarget::Player(id) => self.table.player(id),
        }
    }

    pub(super) fn damage_effect(
        &self,
        effect: &baylee_engine::choice::DamageEffectOption,
        damage: &[baylee_engine::choice::DamagePartView],
    ) -> String {
        let origin = effect
            .source
            .map(|id| self.table.named(id))
            .or_else(|| {
                effect.ability.and_then(|a| {
                    baylee_cards::by_index(a.card).map(|c| c.faces[0].name.to_string())
                })
            })
            .unwrap_or_else(|| "Rule effect".into());
        baylee_client_core::damage::effect_label(
            baylee_client_core::Lang::En,
            effect,
            damage,
            &origin,
            &|target| self.damage_target(target),
        )
    }
}
