"""Distillation from the house expert (`--distill-teacher house`, `house_distillation_v1`).

The data side writes H, the gen-7 house expert's answer computed with foresight
on a fork of the game for the student's seat (fair levels only), as the label of
every decision the recorded seat answers: `single_idx` for single choices and
priority, the candidate and pair target masks (with their free masks) for
subsets and pairs, `scalar_target` for numbers; loss weight 1 there and 0 on
every other decision. Header bit 7 (`n_ne_r`) marks where N, r1's own answer,
differs from H. At priority the coarse-class sidecar holds H's and N's classes
(`training4.coarse`); without it the candidates' action columns give pass
(PassPriority, CancelCast) against act.

`house_loss` teaches every kind, each term the mean over its rows:
- N != H at priority, where H's group (act or pass) differs from N's: H acts,
  -log(p_best_act / (p_best_act + p_pass)) (`ce_h_group`; round 0's overfit
  showed that the act group's summed probability rises while it stays spread
  over equivalent taps, so a pass keeps the top-1 the net plays); H passes,
  -log P(pass group). Inside H's group, wherever it offers more than one answer,
  H's exact candidate, -log(p_H / P(H's group)) (`ce_h_within`): which spell,
  which source; it moves no mass between the groups.
- N != H at every other single choice (mulligan, targets, cards, modes, mana
  abilities, ...): -log p_H (`ce_h_single`).
- N != H at subsets (attackers, multiple targets, cards) and pairs (blocks):
  binary cross-entropy to H's mask on every free entry (`bce_h_subset`,
  `bce_h_pair`).
- N != H at scalars (numbers): squared error to H's normalized value
  (`sq_h_scalar`).
- N = H: KL to r1 at every head (`kl_distilled_n_eq_r`, `kl_subset`, `kl_pair`,
  and `sq_scalar` to r1's number), times `kl_weight`.
- `sq_value_r1`: the value readout's squared distance to r1's at every frame,
  times `value_weight` (`--distill-value-weight`); it keeps a trainable trunk's
  value readout where the frozen value head expects it.
Per pending kind the H terms' sum and weight go into the statistics' kind block
(`distill.kind_means`), so `[distill]` lines show every kind apart.

`HouseCounter` is the held-out side: per kind, the share of decisions whose
decoded answer (as `distill.agreement` decodes it) is H's, at N != H and at
N = H, for the student and r1; with the chain counter's rates it gives the gate
(`house_gate`).
"""
from __future__ import annotations

import torch
import torch.nn.functional as F

from .distill import (CHAIN_ACTS, KIND_SLOTS, PRIORITY, TERMS, _bernoulli_kl, coarse_classes, kind_name)
from .coarse import CLASSES

PASS = CLASSES.index("pass")
H_TERMS = ("ce_h_group", "ce_h_within", "ce_h_single", "bce_h_subset", "bce_h_pair", "sq_h_scalar")
R1_TERMS = ("kl_distilled_n_eq_r", "kl_subset", "kl_pair", "sq_scalar")
GATE_TOP1 = 0.70        # held-out top-1 agreement with H at N != H over every kind
GATE_CHAIN_RATIO = 0.9  # the student's chain-start act rate against H's
GATE_ACTIVE_RATIO = 1.3  # above this the verdict flags a student more active than the teacher


def house_loss(prepared, reference, batch, plans, scope, extra=None):
    """(loss, stats [distill.STATS_WIDTH]) of the house teacher; see the module."""
    device = prepared[0][4].device
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    weight = torch.where(valid, get("loss_weight"), 0.)
    decision = get("is_decision") & valid & (weight > 0)
    heads, differs = get("head"), get("n_ne_r")
    kinds = get("pending_kind").clamp(0, KIND_SLOTS - 1)
    index = {name: i for i, name in enumerate(TERMS)}
    sums, dens = torch.zeros(len(TERMS), device=device), torch.zeros(len(TERMS), device=device)
    kind_sums, kind_dens = torch.zeros(KIND_SLOTS, device=device), torch.zeros(KIND_SLOTS, device=device)

    def add(name, rows, values, w):
        """Rows [B] (or entries [B, W]) with their values and weights."""
        w = w * rows
        sums[index[name]] += (torch.where(rows, values, 0.) * w).sum()
        dens[index[name]] += w.sum()

    def add_kind(rows, values, w, t):
        w = w * rows
        kind_sums.index_add_(0, kinds[:, t], torch.where(rows, values, 0.) * w)
        kind_dens.index_add_(0, kinds[:, t], w)

    for t, plan in enumerate(plans):
        w = weight[:, t]
        for head in range(4):
            if not plan.heads[head]:
                continue
            selected = decision[:, t] & (heads[:, t] == head)
            taught, kept = selected & differs[:, t], selected & ~differs[:, t]
            if head == 0:
                student, ref = prepared[t][1].float(), reference[t][1].float()
                width = student.shape[1]
                mask = get("cand_mask")[:, t, :width]
                genuine = mask.sum(-1) > 1
                taught, kept = taught & genuine, kept & genuine
                log_s = torch.log_softmax(torch.where(mask, student, -1e9), -1)
                log_r = torch.log_softmax(torch.where(mask, ref, -1e9), -1)
                add("kl_distilled_n_eq_r", kept, torch.where(mask, log_r.exp() * (log_r - log_s), 0.).sum(-1), w)
                # Rows of other heads carry no single choice; read index 0 there.
                h_idx = torch.where(selected, get("single_idx")[:, t], 0).clamp(0, width - 1)
                own = -log_s.gather(1, h_idx[:, None])[:, 0]
                priority = kinds[:, t] == PRIORITY
                acting = mask & (coarse_classes(batch, t, width).to(device, non_blocking=True) != PASS)
                passing = mask & ~acting
                n_idx = torch.where(mask, ref, -1e9).argmax(-1)
                if "coarse_n_idx" in batch:   # N's recorded answer where the sidecar has it
                    recorded = get("coarse_n_idx")[:, t]
                    n_idx = torch.where(recorded >= 0, recorded, n_idx).clamp(0, width - 1)
                h_act = acting.gather(1, h_idx[:, None])[:, 0]
                n_act = acting.gather(1, n_idx[:, None])[:, 0]
                split = taught & priority & (h_act != n_act)
                best = torch.where(acting, log_s, -1e9).amax(-1)
                pass_mass = torch.logsumexp(torch.where(passing, log_s, -1e9), -1)
                group = torch.where(h_act, torch.logaddexp(best, pass_mass) - best, -pass_mass)
                inside = torch.where(h_act[:, None], acting, passing)
                within = torch.logsumexp(torch.where(inside, log_s, -1e9), -1) + own
                choosing = taught & priority & (inside.sum(-1) > 1)
                single = taught & ~priority
                add("ce_h_group", split, group, w)
                add("ce_h_within", choosing, within, w)
                add("ce_h_single", single, own, w)
                at_priority = torch.where(split, group, 0.) + torch.where(choosing, within, 0.)
                add_kind(taught, torch.where(priority, at_priority, own), w, t)
            elif head in (1, 2):
                prefix = "cand" if head == 1 else "pair"
                student, ref = prepared[t][head].float(), reference[t][head].float()
                width = student.shape[1]
                eligible = get(prefix + "_mask")[:, t, :width] & (get(prefix + "_free_mask")[:, t, :width] > 0)
                free = torch.where(eligible, get(prefix + "_free_mask")[:, t, :width], 0.)
                target = (get(prefix + "_target_mask")[:, t, :width] > .5).float()
                logits, theirs = torch.where(eligible, student, 0.), torch.where(eligible, ref, 0.)
                bce = F.binary_cross_entropy_with_logits(logits, target, reduction="none")
                h_term, r1_term = ("bce_h_subset", "kl_subset") if head == 1 else ("bce_h_pair", "kl_pair")
                entries = free * w[:, None]
                add(h_term, taught[:, None] & eligible, bce, entries)
                add(r1_term, kept[:, None] & eligible, _bernoulli_kl(logits, theirs), entries)
                per_row = (bce * free).sum(-1) / free.sum(-1).clamp(min=1e-6)
                add_kind(taught & eligible.any(-1), per_row, w, t)
            else:
                number = prepared[t][3].float()
                to_h = (number - get("scalar_target")[:, t]).square()
                add("sq_h_scalar", taught, to_h, w)
                add("sq_scalar", kept, (number - reference[t][3].float()).square(), w)
                add_kind(taught, to_h, w, t)
        add("sq_value_r1", valid[:, t], (prepared[t][4].float() - reference[t][4].float()).square(),
            torch.ones_like(w))
    means = sums / dens.clamp(min=1e-6)
    total = (sum(means[index[name]] for name in H_TERMS)
             + scope.kl_weight * sum(means[index[name]] for name in R1_TERMS))
    if scope.value_weight:
        total = total + scope.value_weight * means[index["sq_value_r1"]]
    for function in (extra or {}).values():
        total = total + function(prepared, batch, plans)
    return total, torch.cat((sums.detach(), dens.detach(), kind_sums.detach(), kind_dens.detach()))


# Per kind: rows at N != H, of them the student's and r1's decoded answer is H's;
# then the same at N = H.
AGREEMENT_COLUMNS = ("n_ne_h", "student_n_ne_h", "r1_n_ne_h", "n_eq_h", "student_n_eq_h", "r1_n_eq_h")


def decoded_agreement(outputs, get, t, head, scalar_tolerance):
    """[B] rows the head decides at step t and [B] whether `outputs`' decoded answer is H's:
    single choices by argmax, subsets and pairs by the threshold-0 decode on every free
    entry, numbers within `scalar_tolerance` (as `distill.agreement`)."""
    if head == 0:
        logits = outputs[t][1].float()
        mask = get("cand_mask")[:, t, :logits.shape[1]]
        return mask.sum(-1) > 1, torch.where(mask, logits, -1e9).argmax(-1) == get("single_idx")[:, t]
    if head in (1, 2):
        prefix = "cand" if head == 1 else "pair"
        logits = outputs[t][head].float()
        width = logits.shape[1]
        eligible = get(prefix + "_mask")[:, t, :width] & (get(prefix + "_free_mask")[:, t, :width] > 0)
        target = get(prefix + "_target_mask")[:, t, :width] > .5
        return eligible.any(-1), ~(eligible & ((logits > 0) != target)).any(-1)
    number = outputs[t][3].float()
    return torch.ones_like(number, dtype=torch.bool), (number - get("scalar_target")[:, t]).abs() <= scalar_tolerance


PAIR_COLUMNS = ("rows", "exact", "class", "group", "within")


class HouseCounter:
    """Held-out agreement with H per pending kind, for the student and r1.

    At priority where N != H it also splits the student's agreement by H's and N's
    coarse class (`H tap / N pass`, ...): H's exact candidate (the raw top-1), H's
    coarse class (another mana source for the same tap counts), H's act/pass group,
    and `within`, the student's best candidate inside H's act/pass group being H's
    (the choice `ce_h_within` teaches, apart from acting or passing). Many N != H
    rows are the same intent through other mechanics (another source, casting with
    automatic payment where N taps first), which exact agreement counts as misses;
    so the gate reads priority by act/pass group and every other kind exactly
    (`val_house_gate_top1_n_ne_h`)."""

    def __init__(self, scalar_tolerance):
        self.scalar_tolerance = scalar_tolerance
        self.table = None
        self.pairs = {}
        self.free = {}          # who -> [rows, passes, chain-start answers] at free priority
        self.free_flag = False  # the sidecar marks free priorities (version 2)

    def observe_free(self, prepared, reference, batch, t, rows, get):
        """Pass and chain-start (tap, cast, activate) shares of the student's top-1, H, N and
        what the seat played, at the same free priorities (the sidecar's flag where it has
        one, else every labelled priority)."""
        if "coarse_free" in batch:
            rows = rows & get("coarse_free")[:, t]
            self.free_flag = True
        logits = prepared[t][1].float()
        width = logits.shape[1]
        mask = get("cand_mask")[:, t, :width]
        classes = coarse_classes(batch, t, width).to(logits.device, non_blocking=True)
        of = lambda index: classes.gather(1, index.clamp(0, width - 1)[:, None])[:, 0]
        n_idx = torch.where(mask, reference[t][1].float(), -1e9).argmax(-1)
        if "coarse_n_idx" in batch:
            recorded = get("coarse_n_idx")[:, t]
            n_idx = torch.where(recorded >= 0, recorded, n_idx)
        answers = dict(student=of(torch.where(mask, logits, -1e9).argmax(-1)),
                       H=of(torch.where(rows, get("single_idx")[:, t], 0)), N=of(n_idx))
        valid = dict.fromkeys(answers, rows)
        if "coarse_p_class" in batch:
            answers["played"] = get("coarse_p_class")[:, t]
            valid["played"] = rows & (answers["played"] >= 0)
        starts = torch.tensor(CHAIN_ACTS, device=logits.device)
        for who, answer in answers.items():
            chosen = valid[who]
            counts = torch.stack((chosen, chosen & (answer == PASS), chosen & torch.isin(answer, starts))).sum(1)
            row = self.free.setdefault(who, [0, 0, 0])
            for i, count in enumerate(counts.tolist()):
                row[i] += count

    def observe_priority(self, prepared, reference, batch, t, rows, get):
        """Tally priority rows at N != H by (H's class, N's class)."""
        logits = prepared[t][1].float()
        width = logits.shape[1]
        mask = get("cand_mask")[:, t, :width]
        classes = coarse_classes(batch, t, width).to(logits.device, non_blocking=True)
        h_idx = torch.where(rows, get("single_idx")[:, t], 0).clamp(0, width - 1)
        n_idx = torch.where(mask, reference[t][1].float(), -1e9).argmax(-1)
        if "coarse_n_idx" in batch:
            recorded = get("coarse_n_idx")[:, t]
            n_idx = torch.where(recorded >= 0, recorded, n_idx).clamp(0, width - 1)
        s_idx = torch.where(mask, logits, -1e9).argmax(-1)
        of = lambda index: classes.gather(1, index[:, None])[:, 0]
        h_class, n_class, s_class = of(h_idx), of(n_idx), of(s_idx)
        h_group = h_class != PASS
        inside = mask & ((classes != PASS) == h_group[:, None])
        s_within = torch.where(inside, logits, -1e9).argmax(-1)
        table = torch.stack((rows, s_idx == h_idx, s_class == h_class, (s_class != PASS) == h_group,
                             s_within == h_idx, h_class, n_class), 1)[rows].tolist()
        for _, exact, same_class, same_group, within, h, n in table:
            key = f"H {CLASSES[h]} / N {CLASSES[n]}"
            row = self.pairs.setdefault(key, [0] * len(PAIR_COLUMNS))
            for i, hit in enumerate((1, exact, same_class, same_group, within)):
                row[i] += int(hit)

    def observe(self, prepared, reference, batch, plans):
        device = prepared[0][4].device
        get = lambda key: batch[key].to(device, non_blocking=True)
        valid = get("step_valid")
        decision = get("is_decision") & valid & (get("loss_weight") > 0)
        heads, differs = get("head"), get("n_ne_r")
        kinds = get("pending_kind").clamp(0, KIND_SLOTS - 1)
        if self.table is None:
            self.table = torch.zeros(KIND_SLOTS, len(AGREEMENT_COLUMNS), dtype=torch.float64, device=device)
        for t, plan in enumerate(plans):
            for head in range(4):
                if not plan.heads[head]:
                    continue
                decides, mine = decoded_agreement(prepared, get, t, head, self.scalar_tolerance)
                _, theirs = decoded_agreement(reference, get, t, head, self.scalar_tolerance)
                rows = decision[:, t] & (heads[:, t] == head) & decides
                apart, same = rows & differs[:, t], rows & ~differs[:, t]
                counts = torch.stack((apart, apart & mine, apart & theirs, same, same & mine, same & theirs), 1)
                self.table.index_add_(0, kinds[:, t], counts.double())
                if head == 0:
                    priority = rows & (kinds[:, t] == PRIORITY)
                    self.observe_priority(prepared, reference, batch, t, priority & differs[:, t], get)
                    self.observe_free(prepared, reference, batch, t, priority, get)

    def metrics(self, chain=None):
        """`val_house_*`: agreement over every kind and per kind; with the chain counter's
        metrics (`distill.ChainCounter`, R := H), the chain-start act rates, the pass
        shares at priority, chain completion and the gate."""
        table = (self.table.cpu() if self.table is not None
                 else torch.zeros(KIND_SLOTS, len(AGREEMENT_COLUMNS), dtype=torch.float64))
        share = lambda hits, rows: float(hits) / float(rows) if rows else None
        by_kind = {}
        for code in range(KIND_SLOTS):
            n_ne, s_ne, r_ne, n_eq, s_eq, r_eq = table[code].tolist()
            if n_ne or n_eq:
                by_kind[kind_name(code)] = dict(n_ne_h=int(n_ne), student=share(s_ne, n_ne), r1=share(r_ne, n_ne),
                                                n_eq_h=int(n_eq), student_n_eq_h=share(s_eq, n_eq),
                                                r1_n_eq_h=share(r_eq, n_eq))
        n_ne, s_ne, r_ne, n_eq, s_eq, r_eq = table.sum(0).tolist()
        out = dict(val_house_n_ne_h=int(n_ne), val_house_top1_n_ne_h=share(s_ne, n_ne),
                   val_house_top1_n_ne_h_r1=share(r_ne, n_ne), val_house_n_eq_h=int(n_eq),
                   val_house_top1_n_eq_h=share(s_eq, n_eq), val_house_top1_n_eq_h_r1=share(r_eq, n_eq),
                   val_house_top1_n_ne_h_by_kind=by_kind,
                   # The drift watch: where r1 already answers as H, how often the student still does.
                   val_house_top1_n_eq_h_by_kind={name: row["student_n_eq_h"] for name, row in by_kind.items()
                                                  if row["n_eq_h"]})
        rates = lambda row: dict(rows=row[0], **{name: share(hits, row[0])
                                                 for name, hits in zip(PAIR_COLUMNS[1:], row[1:])})
        total = [sum(row[i] for row in self.pairs.values()) for i in range(len(PAIR_COLUMNS))]
        # The gate's agreement: priority by act/pass group, every other kind exactly.
        other_rows, other_hits = n_ne - table[PRIORITY, 0].item(), s_ne - table[PRIORITY, 1].item()
        group_hits = total[PAIR_COLUMNS.index("group")]
        out.update(val_house_priority_n_ne_h=rates(total),
                   val_house_priority_n_ne_h_by_pair={key: rates(row) for key, row in
                                                      sorted(self.pairs.items(), key=lambda item: -item[1][0])},
                   val_house_other_top1_n_ne_h=share(other_hits, other_rows),
                   val_house_gate_top1_n_ne_h=share(group_hits + other_hits, total[0] + other_rows),
                   # Before training the student is r1: its top-1 against H's, N's and what was played.
                   val_house_free_priority=dict(
                       flagged=self.free_flag,
                       **{who: dict(rows=rows, pass_share=share(passes, rows), chain_start_share=share(starts, rows))
                          for who, (rows, passes, starts) in self.free.items()}))
        chain = chain or {}
        out.update(
            val_house_chain_start_act=chain.get("val_distill_chain_opportunity_student_act"),
            val_house_chain_start_act_h=chain.get("val_distill_chain_opportunity_r_act"),
            val_house_chain_start_act_r1=chain.get("val_distill_chain_opportunity_r1_act"),
            val_house_chain_start_chances=chain.get("val_distill_chain_opportunity_decisions"),
            val_house_chain_start_recall=chain.get("val_distill_chain_start_student_act"),
            val_house_chain_start_recall_r1=chain.get("val_distill_chain_start_r1_act"),
            val_house_chain_completion=chain.get("val_distill_chain_completion"),
            val_house_chain_completion_r1=chain.get("val_distill_chain_completion_r1"),
            val_house_priority_decisions=chain.get("val_distill_priority_decisions"),
            val_house_priority_pass_share=chain.get("val_distill_priority_pass_share_student"),
            val_house_priority_pass_share_h=chain.get("val_distill_priority_pass_share_r"),
            val_house_priority_pass_share_r1=chain.get("val_distill_priority_pass_share_r1"))
        out["val_house_gate"] = house_gate(out)[1]
        return out


def house_gate(metrics, min_top1=GATE_TOP1, min_ratio=GATE_CHAIN_RATIO, active_ratio=GATE_ACTIVE_RATIO):
    """(passed, verdict): held-out agreement with H at N != H over every kind of at least
    `min_top1`, priority read by act/pass group and every other kind by its exact answer
    (`val_house_gate_top1_n_ne_h`), and the student's chain-start act rate (where H's last
    priority answer was a pass) at least `min_ratio` times H's own on the same rows.
    One-sided: passivity was round 0's failure. Above `active_ratio` times H's the verdict
    says `more_active_than_teacher` and passes; the play check judges whether that is
    reckless. Beside it: the raw exact top-1 (all kinds, and priority alone), priority's
    exact-within-group and class agreement, and the student's, H's and N's (r1's own
    recorded answer, the behaviour before training for a raw arm) chain-start act rate
    and pass shares side by side."""
    top1 = metrics.get("val_house_gate_top1_n_ne_h")
    mine, theirs = metrics.get("val_house_chain_start_act"), metrics.get("val_house_chain_start_act_h")
    ratio = mine / theirs if mine is not None and theirs else None
    passed = top1 is not None and top1 >= min_top1 and ratio is not None and ratio >= min_ratio
    priority = metrics.get("val_house_priority_n_ne_h") or {}
    act = dict(student=mine, H=theirs, N=metrics.get("val_house_chain_start_act_r1"))
    return passed, dict(
        passed=passed, min_top1=min_top1, min_ratio=min_ratio, gate_top1_n_ne_h=top1,
        priority_group_n_ne_h=priority.get("group"), other_kinds_top1_n_ne_h=metrics.get("val_house_other_top1_n_ne_h"),
        exact_top1_n_ne_h=metrics.get("val_house_top1_n_ne_h"), priority_exact_n_ne_h=priority.get("exact"),
        priority_within_n_ne_h=priority.get("within"), priority_class_n_ne_h=priority.get("class"),
        priority_rows_n_ne_h=priority.get("rows"), top1_n_ne_h_r1=metrics.get("val_house_top1_n_ne_h_r1"),
        chain_start_act_ratio=ratio, more_active_than_teacher=ratio is not None and ratio > active_ratio,
        chain_start_act=act,
        pass_share_at_chances={who: None if rate is None else 1 - rate for who, rate in act.items()},
        pass_share_at_priority=dict(student=metrics.get("val_house_priority_pass_share"),
                                    H=metrics.get("val_house_priority_pass_share_h"),
                                    N=metrics.get("val_house_priority_pass_share_r1")),
        chain_completion=dict(student=metrics.get("val_house_chain_completion"),
                              N=metrics.get("val_house_chain_completion_r1")),
        chain_start_chances=metrics.get("val_house_chain_start_chances"),
        at_free_priority=metrics.get("val_house_free_priority"))
