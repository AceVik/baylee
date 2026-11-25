"""Opt-in bundle distillation (`--distill-bundle`) on `bundle_distillation_v1` datasets.

The dataset's ordinary policy targets hold R, the tactical layer's answer with
the bundle, at every labelled decision of the recorded seats (loss weight 1;
every other decision weight 0); header flag bit 7 marks where N, the net's own
greedy answer, differs (`n_ne_r`).

The loss changes the student only where the bundle disagrees with the net, and
only in scope (`Scope`: pending kinds, priority by default, optionally narrowed
to subtypes such as cast/pass/instant_their_turn/spend):
- cross-entropy to R at in-scope decisions where N != R, optionally weighted by
  the rule groups that fired there (product of their weights, others 1);
- beta * KL(r1 || student) everywhere else: in-scope decisions where N = R, the
  same kinds' out-of-scope subtypes, other single choices, subsets and pairs
  (Bernoulli per free entry), and the squared distance to r1's scalar. Each term
  is reported apart.
With `out_of_scope` "ce-n" (`--distill-out-of-scope ce-n`) the same kinds'
out-of-scope single choices instead get cross-entropy to N, r1's own top-1,
times its weight (`ce_n_out_of_scope`); their KL is still reported.
Outcome weighting does not apply: the label is R whatever the game's result.
Value and auxiliary targets are not part of this loss.

With `advantage` "pairwise" (`--distill-advantage pairwise`, `training4.distill_probe`)
nothing is cross-entropy to R: at priority frames the dataset's probes measured
(R and N played on in the same fair worlds), R's log-odds over N move from r1's
by R's measured advantage (`bce_adv_pairwise`); every other priority frame keeps
r1 by the KL (`kl_distilled_n_eq_r`, and `kl_adv_unprobed` where N != R without a
usable probe), and every other kind keeps its KL as above.

The reference r1 is a frozen copy of the --init weights. When nothing before the
heads learns (stage A) it reads the student's own boards and memory, a second
readout and no trunk. With frozen layers below L (stage B) it runs only its own
layers L and above on the inputs the student's layer L received. Otherwise it
encodes its own boards. Without a shared trunk it carries its own memory.

`AUXILIARY_TARGETS` is the hook for stage B's extra auxiliary heads: each entry
`name -> fn(student_outputs, batch, plans) -> loss` joins the total. Empty today.

A student with the deliberation block (thinking revision 2, `--think-supervision`)
adds `thinking_terms` through the same hook: every step of the block gets this
loss's own target (R in scope where N != R, r1 elsewhere), a penalty for being
worse than the step before, and a confidence trained on whether its answer is
that target. The reference never deliberates: its block is switched off, so it
stays r1 whatever the student's block learns.
"""
from __future__ import annotations

from contextlib import contextmanager
import copy
from dataclasses import dataclass
import json
import math

import torch
import torch.nn.functional as F

import model4 as M
from .coarse import CLASSES
from .deferred import deferred_readouts
from .distill_probe import MODES as ADVANTAGE_MODES, AdvantageCounter, pairwise, probed_rows
from .execution import encode_boards, plan_batch, transfer_batch

CONTRACT = "bundle_distillation_v1"
HOUSE_CONTRACT = "house_distillation_v1"   # R := H, the house expert's answer (`training4.house`)
CONTRACTS = {"bundle": CONTRACT, "house": HOUSE_CONTRACT}
N_NE_R_FLAG = 128  # BST4 header flags bit 7
TERMS = ("ce_r", "kl_distilled_n_eq_r", "kl_kind_out_of_scope", "kl_single", "kl_subset", "kl_pair", "sq_scalar",
         "ce_n_out_of_scope", "ce_coarse_r", "ce_fine_n",
         # The house teacher's terms (`training4.house`); a bundle run leaves them empty.
         "ce_h_group", "ce_h_within", "ce_h_single", "bce_h_subset", "bce_h_pair", "sq_h_scalar", "sq_value_r1",
         # The pairwise advantage loss (`training4.distill_probe`); every other mode leaves them empty.
         "bce_adv_pairwise", "kl_adv_unprobed")
ADV_PAIRWISE, ADV_UNPROBED = TERMS.index("bce_adv_pairwise"), TERMS.index("kl_adv_unprobed")
KL_TERMS = slice(1, 7)
KIND_SLOTS = 32   # pending kinds (`features4::PendingKind4`, 0..27) with room to spare
# Training statistics: per term weighted sum, then weight; then per pending kind the
# house terms' sum and weight (zero in a bundle run).
STATS_WIDTH = 2 * len(TERMS) + 2 * KIND_SLOTS
OUT_OF_SCOPE_MODES = ("kl", "ce-n")
PRIORITY = 0   # pending kind of a priority decision
COARSE = CLASSES   # coarse classes of a priority answer (coarse_class_v1)
COARSE_MODES = (None, "act", "class", "act-exact", "act-best")
AUXILIARY_TARGETS = {}


def dataset_contract(meta):
    value = meta.get("bundle_distillation")
    return value.get("contract") if isinstance(value, dict) else None


def check_datasets(datasets, enabled, teacher="bundle"):
    """Fail closed both ways: R labels train only under --distill-bundle, and only
    under their own teacher (`--distill-teacher`): bundle_distillation_v1, or
    house_distillation_v1 whose R is the house expert's answer H.

    Also hands each dataset the subtype and rule-group vocabularies its frames index.
    """
    for dataset in datasets:
        meta = getattr(dataset, "meta", None) or {}
        contract = dataset_contract(meta)
        if enabled and contract != CONTRACTS[teacher]:
            raise ValueError(f"--distill-bundle with the {teacher} teacher needs {CONTRACTS[teacher]} datasets, "
                             f"not {contract}")
        if not enabled and contract is not None:
            raise ValueError(f"{contract} labels train only under --distill-bundle")
        dataset.distillation = enabled
        if enabled:
            dataset.distill_subtypes = list(meta["bundle_distillation"].get("subtypes") or [])
            dataset.distill_groups = list(meta["bundle_distillation"].get("groups") or [])


@dataclass(frozen=True)
class Scope:
    """What is distilled to R; everything else stays at r1 through the KL.

    `out_of_scope` "ce-n" holds the distilled kinds' out-of-scope single choices
    at r1's own top-1 (N) by cross-entropy, weight `out_of_scope_weight`, its own
    term beside `ce_r`; their KL is then reported but not trained.

    `coarse` ("act" or "class") teaches the labelled priority decisions where
    N != R by a group of answers, from the dataset's sidecar (`training4.coarse`):
    "act" splits pass (PassPriority, CancelCast) from act (tap, cast, activate,
    land, other), "class" takes the six classes of `COARSE`. Where R's group
    differs from N's, cross-entropy to R's group on the student's summed
    probability of it (`ce_coarse_r`), which teaches both directions. Where both
    share a group, a `cast` decision keeps its cross-entropy to R (`ce_r`: which
    spell); any other, cross-entropy to N's recorded candidate (`ce_fine_n`: a
    tap's source or order). Every other decision keeps its term (the KL at
    N = R; `ce_r` at target questions).

    "act-exact" groups as "act", but where R acts and N passes `ce_coarse_r` is the
    cross-entropy to R's own candidate: the summed act mass can grow while it
    stays spread over equivalent taps and a single pass keeps the top-1, which
    is what the net plays. Where R passes and N acts it stays the group's.

    `teacher` "house" (house_distillation_v1, R := H) replaces all of this with
    `training4.house.house_loss`: every labelled kind is taught, so kinds,
    subtypes, group weights, the out-of-scope mode and `coarse` do not apply;
    `value_weight` adds the value readout's squared distance to r1 there.

    `advantage` "pairwise" (`training4.distill_probe`) replaces the cross-entropy
    to R at priority: where the dataset's probe sidecar measured R beside N in at
    least `advantage_min_worlds` worlds, R's log-odds over N move from r1's by
    clamp(mean / `advantage_beta`, +-`advantage_clip`) (weight
    `advantage_weight`); everywhere else the KL keeps r1. It needs priority alone
    among the kinds, every subtype, no group weights, the out-of-scope KL, no
    coarse mode and the bundle teacher. The other `advantage_*` fields also set
    how validation reads the probes when the mode is off; `advantage_rows`, the
    run's probed rows per training batch, is the constant each probed row's
    weight is divided by (None: the batch's own count of them)."""
    kinds: tuple = (0,)
    subtypes: tuple | None = None
    subtype_names: tuple = ()
    group_weights: tuple | None = None
    kl_weight: float = 1.0
    out_of_scope: str = "kl"
    out_of_scope_weight: float = 1.0
    coarse: str | None = None
    teacher: str = "bundle"
    value_weight: float = 0.0
    advantage: str | None = None
    advantage_beta: float = 0.05
    advantage_clip: float = 3.0
    advantage_weight: float = 1.0
    advantage_min_worlds: int = 2
    advantage_rows: float | None = None

    @classmethod
    def from_options(cls, kinds, kl_weight, subtypes, group_weights, datasets, out_of_scope="kl",
                     out_of_scope_weight=1.0, coarse=None, teacher="bundle", value_weight=0.0, advantage=None,
                     advantage_beta=0.05, advantage_clip=3.0, advantage_weight=1.0, advantage_min_worlds=2,
                     advantage_rows=None):
        """Names resolved against vocabularies every dataset shares; unknown names fail."""
        if teacher not in CONTRACTS:
            raise ValueError(f"Unknown distillation teacher {teacher!r}")
        if teacher == "house" and (subtypes or group_weights or out_of_scope != "kl" or coarse):
            raise ValueError("The house teacher teaches every kind: no subtypes, group weights, out-of-scope "
                             "mode or coarse mode")
        if not value_weight >= 0 or (value_weight and teacher != "house"):
            raise ValueError("A value weight is nonnegative and belongs to the house teacher")
        if advantage not in ADVANTAGE_MODES:
            raise ValueError(f"Unknown advantage distillation {advantage!r}")
        if not (math.isfinite(advantage_beta) and advantage_beta > 0 and math.isfinite(advantage_clip)
                and advantage_clip > 0 and math.isfinite(advantage_weight) and advantage_weight >= 0
                and int(advantage_min_worlds) == advantage_min_worlds >= 1):
            raise ValueError("Advantage distillation needs a positive beta and clip, a nonnegative weight and at "
                             "least one world")
        if advantage and (teacher != "bundle" or subtypes or group_weights or out_of_scope != "kl" or coarse
                          or tuple(kinds) != (PRIORITY,)):
            raise ValueError("Advantage distillation teaches the bundle teacher's priority probes alone: priority "
                             "the only kind, no subtypes, group weights, out-of-scope mode or coarse mode")
        if advantage_rows is not None and not (advantage and math.isfinite(advantage_rows) and advantage_rows >= 1):
            raise ValueError("The probed rows per batch are a finite number of at least 1, of an advantage run")
        if advantage and not all(getattr(d, "probes", None) is not None for d in datasets):
            raise ValueError("Advantage distillation needs every dataset's probe sidecar (bundle-distill/probe.*)")
        vocabularies = {(tuple(d.distill_subtypes), tuple(d.distill_groups)) for d in datasets}
        if len(vocabularies) != 1:
            raise ValueError("Training and validation distillation vocabularies differ")
        (subtype_names, group_names), = vocabularies
        codes = None
        if subtypes:
            missing = [name for name in subtypes if name not in subtype_names]
            if missing:
                raise ValueError(f"Dataset declares no distillation subtype {missing}")
            codes = tuple(sorted(subtype_names.index(name) for name in subtypes))
        weights = None
        if group_weights:
            missing = [name for name in group_weights if name not in group_names]
            if missing:
                raise ValueError(f"Dataset declares no rule group {missing}")
            weights = tuple(float(group_weights.get(name, 1.0)) for name in group_names)
        if out_of_scope not in OUT_OF_SCOPE_MODES:
            raise ValueError(f"Unknown out-of-scope distillation {out_of_scope!r}")
        if coarse not in COARSE_MODES:
            raise ValueError(f"Unknown coarse distillation {coarse!r}")
        if coarse and (PRIORITY not in kinds or out_of_scope != "kl"):
            raise ValueError("Coarse distillation needs priority among the kinds and the out-of-scope KL")
        return cls(tuple(kinds), codes, subtype_names, weights, kl_weight, out_of_scope, float(out_of_scope_weight),
                   coarse, teacher, float(value_weight), advantage, float(advantage_beta), float(advantage_clip),
                   float(advantage_weight), int(advantage_min_worlds),
                   None if advantage_rows is None else float(advantage_rows))

    def groups(self, classes):
        """`classes` (COARSE indices) as this scope's coarse groups: act/pass or the class itself."""
        return classes if self.coarse == "class" else (classes != COARSE.index("pass")).long()

    def masks(self, batch, device):
        """[B, U] masks: in the kinds, and in scope (kind and subtype)."""
        in_kind = torch.isin(batch["pending_kind"].to(device), torch.tensor(self.kinds, device=device))
        if self.subtypes is None:
            return in_kind, in_kind
        subtype = batch["distill_subtype"].to(device)
        return in_kind, in_kind & torch.isin(subtype, torch.tensor(self.subtypes, device=device))

    def multiplier(self, batch, device):
        """[B, U] cross-entropy weight: product of the fired groups' weights (others 1)."""
        if self.group_weights is None:
            return None
        weights = torch.tensor(self.group_weights, device=device)
        fired = batch["fired_mask"].to(device)
        bits = (fired[..., None] >> torch.arange(len(weights), device=device)) & 1
        return torch.where(bits.bool(), weights, 1.).prod(-1)


class Reference:
    """Frozen r1 and its outputs on the student's batch."""

    @staticmethod
    def snapshot(net):
        """Copy the --init weights before any freezing or adapter; rules are attached later."""
        rules = net.rules
        net.rules = None
        try:
            return copy.deepcopy(net)
        finally:
            net.rules = rules

    def __init__(self, copied, student, trainable_from_layer=-1):
        from .partial import upstream_frozen, below_layer_frozen
        rules = student.rules
        rules_frozen = not any(p.requires_grad for p in rules.parameters())
        copied.rules = rules if rules_frozen else copy.deepcopy(rules)
        self.net = copied
        self.own_rules = not rules_frozen
        self.net.eval()
        block = getattr(self.net, "deliberation", None)
        if block is not None:
            # r1 is the heads' first proposal; zero steps return it exactly.
            block.steps_override, block.collect = 0, False
        for parameter in self.net.parameters():
            parameter.requires_grad_(False)
        # Stage A: everything before the heads is frozen, so boards and memory are r1's.
        self.shared_trunk = upstream_frozen(student)
        # Stage B: what feeds layer L is frozen, so r1 differs from layer L on.
        self.shared_lower = (trainable_from_layer if not self.shared_trunk and trainable_from_layer >= 0
                             and below_layer_frozen(student, trainable_from_layer) else None)
        self.captured = []
        self.hidden = None

    @contextmanager
    def capturing(self, student):
        """Record the inputs of the student's layer L during its board encoding."""
        self.captured = []
        if self.shared_lower is None:
            yield
            return
        student.capture_layer_input = (self.shared_lower, self.captured)
        try:
            yield
        finally:
            student.capture_layer_input = None

    def _upper_boards(self, lanes):
        """r1's layers L and above on each captured chunk, split back into frames."""
        boards = []
        for inputs, pad in self.captured:
            x = inputs
            for layer in self.net.trunk_layers[self.shared_lower:]:
                x = layer(x, src_key_padding_mask=pad)
            board = self.net.trunk_norm(x)
            frames = board.shape[0] // lanes
            board = board.reshape(lanes, frames, *board.shape[1:])
            mask = pad.reshape(lanes, frames, *pad.shape[1:])
            boards.extend((board[:, t], mask[:, t]) for t in range(frames))
        return boards

    def outputs(self, host, batch, boards, hidden, plans, device, bf16, chunk, resets=None):
        with torch.no_grad():
            b, unroll = batch["step_valid"].shape
            if self.own_rules:
                with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                    self.net.clear_structure()
                    self.net.prepare_structure((host["entity_rules"], host["cand_rules"]))
            if self.shared_trunk:
                ref_boards, ref_hidden = boards, hidden.detach()
            else:
                with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
                    ref_boards = self._upper_boards(b) if self.captured else None
                    if ref_boards is None or len(ref_boards) != unroll:
                        ref_boards = encode_boards(self.net, batch, chunk, device)
                if resets is not None or self.hidden is None or self.hidden.shape[0] != b:
                    self.hidden = torch.zeros_like(hidden)
                new = batch["is_new_traj"].to(device, non_blocking=True)
                ref_hidden = torch.where(new[:, None, None], torch.zeros_like(self.hidden), self.hidden)
            prepared, final = deferred_readouts(self.net, batch, ref_boards, ref_hidden, plans,
                                                self.net.decision_chunk, device, bf16, resets=resets)
            if not self.shared_trunk:
                self.hidden = final
            if self.own_rules:
                self.net.clear_structure()
            self.captured = []
        return prepared


def _bernoulli_kl(student, reference):
    """KL(Bernoulli(sigmoid(reference)) || Bernoulli(sigmoid(student))) per entry."""
    p = torch.sigmoid(reference)
    return (p * (F.logsigmoid(reference) - F.logsigmoid(student))
            + (1 - p) * (F.logsigmoid(-reference) - F.logsigmoid(-student)))


def distill_loss(prepared, reference, batch, plans, scope, extra=None):
    """Returns (loss, stats [STATS_WIDTH]): per term weighted sum, then weight, then
    the house teacher's per-kind block (zero here)."""
    if scope.teacher == "house":
        from .house import house_loss
        return house_loss(prepared, reference, batch, plans, scope, extra)
    device = prepared[0][4].device
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    weight = torch.where(valid, get("loss_weight"), 0.)
    decision = get("is_decision") & valid & (weight > 0)
    heads = get("head")
    in_kind, in_scope = scope.masks(batch, device)
    multiplier = scope.multiplier(batch, device)
    differs = get("n_ne_r")
    sums = torch.zeros(len(TERMS), device=device)
    dens = torch.zeros(len(TERMS), device=device)
    for t, plan in enumerate(plans):
        for kind in range(4):
            if not plan.heads[kind]:
                continue
            selected = decision[:, t] & (heads[:, t] == kind)
            w = weight[:, t] * selected
            if kind == 0:
                student = prepared[t][1].float()
                ref = reference[t][1].float()
                mask = get("cand_mask")[:, t, :student.shape[1]]
                genuine = selected & (mask.sum(-1) > 1)
                distilled = genuine & in_scope[:, t]
                target = torch.where(distilled & differs[:, t], get("single_idx")[:, t], 0)
                log_s = torch.log_softmax(torch.where(mask, student, -1e9), -1)
                log_r = torch.log_softmax(torch.where(mask, ref, -1e9), -1)
                ce = -log_s.gather(1, target[:, None])[:, 0]
                kl = torch.where(mask, log_r.exp() * (log_r - log_s), 0.).sum(-1)
                outside = genuine & in_kind[:, t] & ~in_scope[:, t]
                to_r = distilled & differs[:, t]
                coarse_routes = []
                if scope.coarse:
                    # Priority decisions where N != R learn by group (the dataset's sidecar:
                    # classes per candidate, N's recorded answer); the KL keeps N = R.
                    n_idx = get("coarse_n_idx")[:, t]
                    r_idx = get("single_idx")[:, t].clamp(min=0)
                    taught = (genuine & (get("pending_kind")[:, t] == PRIORITY) & differs[:, t]
                              & (n_idx >= 0))
                    groups = scope.groups(batch["coarse_class"][:, t, :student.shape[1]].to(device, non_blocking=True))
                    r_group = groups.gather(1, r_idx[:, None])
                    split = taught & (r_group != groups.gather(1, n_idx.clamp(min=0)[:, None]))[:, 0]
                    # Inside one group a cast decision learns which spell from R.
                    cast = (get("distill_subtype")[:, t] == scope.subtype_names.index("cast")
                            if "cast" in scope.subtype_names else torch.zeros_like(taught))
                    kept = taught & ~split & cast
                    to_r, outside = (to_r & ~taught) | kept, outside & ~taught
                    ce = -log_s.gather(1, torch.where(to_r, r_idx, 0)[:, None])[:, 0]
                    to_group = -torch.logsumexp(torch.where(mask & (groups == r_group), log_s, -1e9), -1)
                    if scope.coarse == "act-exact":
                        # R acts where N passes: R's own candidate, not the act group's mass.
                        to_group = torch.where(r_group[:, 0] == 1, -log_s.gather(1, r_idx[:, None])[:, 0], to_group)
                    elif scope.coarse == "act-best":
                        # R acts where N passes: the student's best act against the pass group,
                        # -log(p_best / (p_best + p_pass)); it is below log 2 exactly when an act leads.
                        best = torch.where(mask & (groups == 1), log_s, -1e9).amax(-1)
                        passing = torch.logsumexp(torch.where(mask & (groups == 0), log_s, -1e9), -1)
                        to_group = torch.where(r_group[:, 0] == 1, torch.logaddexp(best, passing) - best, to_group)
                    coarse_routes = [
                        (8, split, to_group),
                        (9, taught & ~split & ~kept, -log_s.gather(1, n_idx.clamp(min=0)[:, None])[:, 0])]
                routes = [(0, to_r, ce), (1, distilled & ~differs[:, t], kl),
                          (2, outside, kl), (3, genuine & ~in_kind[:, t], kl)] + coarse_routes
                if scope.advantage:
                    # No cross-entropy to R: the probed frames move R's log-odds over N by R's
                    # measured advantage; every other priority frame keeps r1 by the KL.
                    probed = distilled & probed_rows(get, t, scope)
                    _, _, _, pair = pairwise(student, ref, mask, probed, get("probe_n_idx")[:, t],
                                             get("single_idx")[:, t], get("probe_mean")[:, t], scope)
                    routes = [(1, distilled & ~differs[:, t] & ~probed, kl),
                              (ADV_UNPROBED, distilled & differs[:, t] & ~probed, kl),
                              (ADV_PAIRWISE, probed, pair),
                              (2, outside, kl), (3, genuine & ~in_kind[:, t], kl)]
                if scope.out_of_scope == "ce-n":
                    # N: r1's own top-1, the answer the out-of-scope rows should keep.
                    n_top1 = torch.where(mask, ref, -1e9).argmax(-1)
                    routes.append((7, outside, -log_s.gather(1, n_top1[:, None])[:, 0]))
                for index, rows, values in routes:
                    rw = weight[:, t] * rows
                    if index == 0 and multiplier is not None:
                        rw = rw * multiplier[:, t]
                    sums[index] = sums[index] + (torch.where(rows, values, 0.) * rw).sum()
                    dens[index] = dens[index] + rw.sum()
            elif kind in (1, 2):
                prefix = "cand" if kind == 1 else "pair"
                student = prepared[t][kind].float()
                ref = reference[t][kind].float()
                width = student.shape[1]
                mask = get(prefix + "_mask")[:, t, :width]
                free = get(prefix + "_free_mask")[:, t, :width]
                eligible = selected[:, None] & mask & (free > 0)
                kl = torch.where(eligible, _bernoulli_kl(torch.where(eligible, student, 0.),
                                                         torch.where(eligible, ref, 0.)), 0.)
                ew = torch.where(eligible, free, 0.) * w[:, None]
                index = 4 if kind == 1 else 5
                sums[index] = sums[index] + (kl * ew).sum()
                dens[index] = dens[index] + ew.sum()
            else:
                error = torch.where(selected, prepared[t][3].float() - reference[t][3].float(), 0.)
                sums[6] = sums[6] + (error.square() * w).sum()
                dens[6] = dens[6] + w.sum()
    means = sums / dens.clamp(min=1e-6)
    if scope.advantage:
        # Every priority frame without a usable probe keeps r1 as one pooled KL, N = R and
        # unprobed N != R alike; the two are reported apart.
        kept = (sums[1] + sums[ADV_UNPROBED]) / (dens[1] + dens[ADV_UNPROBED]).clamp(min=1e-6)
        others = means[2:KL_TERMS.stop].sum()   # out of scope, other single choices, subsets, pairs, scalars
        # A probed row weighs the same however many share its batch; the reported term is their mean.
        pairs = means[ADV_PAIRWISE] if scope.advantage_rows is None else sums[ADV_PAIRWISE] / scope.advantage_rows
        total = scope.advantage_weight * pairs + scope.kl_weight * (kept + others)
    elif scope.out_of_scope == "ce-n":
        # The out-of-scope KL stays a reported guard; the anchor to N trains there.
        trained_kl = means[1] + means[3:7].sum()
        total = means[0] + scope.kl_weight * trained_kl + scope.out_of_scope_weight * means[7]
    else:
        total = means[0] + scope.kl_weight * means[KL_TERMS].sum()
    if scope.coarse:
        total = total + means[8] + means[9]
    for name, function in (extra or {}).items():
        total = total + function(prepared, batch, plans)
    return total, torch.cat((sums.detach(), dens.detach(), torch.zeros(2 * KIND_SLOTS, device=device)))


def thinking_terms(frames, reference, batch, plans, scope, config):
    """Per-step terms of the deliberation block under this loss's targets.

    `frames[t]["logits"]` is `[S + 1, B, C]` (step 0: the heads' first proposal).
    A step's loss at a decision is the distillation loss it would have as the
    final answer: cross-entropy to R at in-scope single choices where N != R
    (with the rule groups' weights), `kl_weight` * KL(r1 || step) at every other
    single choice and at subsets. From these: the mean over the steps before
    the last (deep supervision; the last step is `distill_loss`'s own), a
    penalty where a step is worse than the one before, and the confidence
    against "this step's answer is its target" (R, or r1's own answer). The
    consequence term is left out: the game's outcome belongs to the answer that
    was played, not to R. Blocker pairs and scalars are not deliberated.

    Returns (loss, stats): per step the in-scope N != R decisions answered as
    R `[S + 1]`, then their count.
    """
    if scope.advantage:
        raise ValueError("The thinking terms target R; advantage distillation has no R to target")
    device = next((f["logits"].device for f in frames or () if f is not None), None)
    if device is None:
        raise ValueError("No thinking trace: the deliberation block must collect during deferred readouts")
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    weight = torch.where(valid, get("loss_weight"), 0.)
    decision = get("is_decision") & valid & (weight > 0)
    heads = get("head")
    _, in_scope = scope.masks(batch, device)
    multiplier = scope.multiplier(batch, device)
    differs = get("n_ne_r")
    zero = torch.zeros((), device=device)
    deep = improve = confidence = weights = decisions = count = zero
    right_at_r = None
    for t, (frame, plan) in enumerate(zip(frames, plans, strict=True)):
        if frame is None or frame["logits"].shape[0] < 2 or not (plan.heads[0] or plan.heads[1]):
            continue
        logits = frame["logits"].float()
        steps, width = logits.shape[0] - 1, logits.shape[2]
        mask = get("cand_mask")[:, t, :width]
        ref = reference[t][1].float()
        parts = []
        if plan.heads[0]:
            genuine = decision[:, t] & (heads[:, t] == 0) & (mask.sum(-1) > 1)
            to_r = genuine & in_scope[:, t] & differs[:, t]
            target = torch.where(to_r, get("single_idx")[:, t], 0)
            log_s = torch.log_softmax(torch.where(mask[None], logits, -1e9), -1)
            log_r = torch.log_softmax(torch.where(mask, ref, -1e9), -1)
            ce = -log_s.gather(2, target[None, :, None].expand(steps + 1, -1, 1))[..., 0]
            kl = torch.where(mask[None], log_r.exp()[None] * (log_r[None] - log_s), 0.).sum(-1)
            ce_weight = multiplier[:, t] if multiplier is not None else torch.ones_like(weight[:, t])
            losses = torch.where(to_r[None], ce * ce_weight[None], scope.kl_weight * kl)
            answer = torch.where(mask[None], logits, -1e9).argmax(-1)
            wanted = torch.where(to_r, target, torch.where(mask, ref, -1e9).argmax(-1))
            right = (answer == wanted[None]) & genuine[None]
            hits = (right & to_r[None]).sum(-1).to(losses.dtype)
            right_at_r = hits if right_at_r is None else right_at_r + hits
            count = count + to_r.sum()
            parts.append((losses, genuine, right))
        if plan.heads[1]:
            selected = decision[:, t] & (heads[:, t] == 1)
            free = get("cand_free_mask")[:, t, :width]
            eligible = selected[:, None] & mask & (free > 0)
            entries = torch.where(eligible[None], _bernoulli_kl(torch.where(eligible[None], logits, 0.),
                                                                torch.where(eligible, ref, 0.)[None]), 0.)
            free = torch.where(eligible, free, 0.)
            losses = scope.kl_weight * (entries * free[None]).sum(-1) / free.sum(-1).clamp(min=1.)[None]
            genuine = eligible.any(-1)
            right = ~(eligible[None] & ((logits > 0) != (ref > 0)[None])).any(-1) & genuine[None]
            parts.append((losses, genuine, right))
        for losses, genuine, right in parts:
            w = weight[:, t] * genuine
            if steps > 1:
                deep = deep + (losses[1:steps] * w).sum() / (steps - 1)
            gain = F.relu(losses[1:] - losses[:-1].detach() + config.improve_margin)
            improve = improve + (gain * w).sum() / steps
            weights = weights + w.sum()
            known = genuine.to(losses.dtype)
            confidence = confidence + (F.binary_cross_entropy_with_logits(
                frame["confidence"].float(), right[1:].to(losses.dtype), reduction="none") * known).sum() / steps
            decisions = decisions + known.sum()
    total = (config.deep_weight * deep / weights.clamp(min=1e-6)
             + config.improve_weight * improve / weights.clamp(min=1e-6)
             + config.confidence_weight * confidence / decisions.clamp(min=1.))
    if right_at_r is None:
        return total, torch.zeros(2, device=device)
    return total, torch.cat((right_at_r.detach(), count.detach().to(right_at_r.dtype)[None]))


def term_means(stats):
    n = len(TERMS)
    return {name: float(stats[i] / stats[n + i]) if float(stats[n + i]) > 0 else None
            for i, name in enumerate(TERMS)}


# `features4::PendingKind4`; unnamed codes read as kind<N>.
KIND_NAMES = ("priority", "mana_ability", "damage_source", "damage_effect", "prevention", "attackers", "blockers",
              "cards", "targets", "arrange", "legend", "mulligan", "mulligan_bottom", "discard", "yes_no",
              "cast_mode", "number", "colour", "subtype", "card_name", "player", "pile")


def kind_name(code):
    return KIND_NAMES[code] if 0 <= code < len(KIND_NAMES) else f"kind{code}"


def kind_means(stats):
    """The house terms per pending kind: {name: [mean, weight]} for kinds with rows."""
    base, out = 2 * len(TERMS), {}
    for code in range(KIND_SLOTS):
        weight = float(stats[base + KIND_SLOTS + code])
        if weight > 0:
            out[kind_name(code)] = [float(stats[base + code]) / weight, round(weight)]
    return out


def train_line(stats):
    """`[distill] train` JSON: every term's mean, and per pending kind where any trained."""
    stats = stats.cpu()
    line = term_means(stats)
    kinds = kind_means(stats)
    if kinds:
        line["kinds"] = kinds
    return line


# What a distillation epoch line reports of validation (the full set is in meta.json).
EPOCH_VALIDATION = ("val_distill_selection", "val_distill_distilled_n_ne_r", "val_distill_distilled_top1_r",
                    "val_distill_out_of_scope_top1_r", "val_distill_out_of_scope_top1_n",
                    "val_distill_out_of_scope_top1_neither", "val_distill_coarse_top1_r",
                    "val_distill_coarse_top1_r_n_ne_r", "val_distill_act_r_act_n_pass_student_act",
                    "val_distill_act_r_pass_n_act_student_pass", "val_distill_chain_start_student_act",
                    "val_distill_chain_start_r1_act", "val_distill_chain_start_top1_r", "val_distill_chain_start_top1_n",
                    "val_distill_chain_mid_student_act", "val_distill_chain_mid_r1_act",
                    "val_distill_chain_completion", "val_distill_chain_completion_r1",
                    "val_distill_pregate_start_act_ratio", "val_distill_pregate_pass_ratio",
                    "val_distill_chain_start_pass_margin", "val_distill_chain_start_pass_margin_r1",
                    *(f"val_distill_{name}" for name in TERMS),
                    "val_value_mse_to_r1",
                    # The probes (`training4.distill_probe.AdvantageCounter`), where the dataset has them.
                    "val_adv_selection", "val_adv_probed_rows", "val_adv_r_ahead_rows", "val_adv_r_ahead_top1_r",
                    "val_adv_r_ahead_top1_r_r1", "val_adv_n_ahead_rows", "val_adv_n_ahead_top1_n",
                    "val_adv_n_ahead_top1_n_r1", "val_adv_shift_corr", "val_adv_shift_mean_r_ahead",
                    "val_adv_shift_mean_n_ahead",
                    # The house teacher's gate (`training4.house.HouseCounter`).
                    "val_house_gate_top1_n_ne_h", "val_house_other_top1_n_ne_h", "val_house_top1_n_ne_h",
                    "val_house_top1_n_ne_h_by_kind", "val_house_top1_n_eq_h",
                    "val_house_top1_n_eq_h_by_kind", "val_house_priority_n_ne_h", "val_house_free_priority",
                    "val_house_chain_start_act", "val_house_chain_start_act_h", "val_house_chain_start_act_r1",
                    "val_house_priority_pass_share", "val_house_priority_pass_share_h",
                    "val_house_priority_pass_share_r1", "val_house_gate")


def epoch_line(epoch, step, stats, val_metrics, drift=None):
    """`[distill] epoch` JSON: the epoch's training terms (and per pending kind, for the
    house teacher), the pass drift at the chain starts it trained on, and validation's
    agreement with R. A distillation run trains none of the behaviour-cloning terms the
    `[train4] epoch` line prints, so they read zero there."""
    train = train_line(stats) if stats is not None else None
    validation = {key: val_metrics[key] for key in EPOCH_VALIDATION if key in val_metrics}
    return "[distill] epoch " + json.dumps(dict(epoch=epoch, step=step, train=train,
                                                drift=None if drift is None else drift_means(drift),
                                                validation=validation or None))


GROUPS = ("distilled", "out_of_scope", "single", "subset", "pair", "scalar")
# A priority candidate's action: its one-hot column in the candidate features
# (features4::encode_candidates4); anything else is "none".
PRIORITY_ACTIONS = (("pass", 0), ("cancel", 1), ("land", 2), ("cast", 3), ("activate", 4), ("mana", 5),
                    ("granted", 8))
ACTION_NAMES = tuple(name for name, _ in PRIORITY_ACTIONS) + ("none",)
ANSWER_COLUMNS = ("decisions", "top1_n", "top1_r", "top1_neither", "r1_top1_r", "n_flag_mismatch")


def action_classes(cand_feat):
    """[..., C] index into ACTION_NAMES of each priority candidate's action."""
    columns = torch.tensor([column for _, column in PRIORITY_ACTIONS], device=cand_feat.device)
    marked = cand_feat[..., columns] > .5
    return torch.where(marked.any(-1), marked.int().argmax(-1), len(PRIORITY_ACTIONS))


# ACTION_NAMES -> COARSE: a mana ability is a tap; a cancel passes (coarse_class_v1 counts
# CancelCast as pass); a granted action or none is other.
ACTION_COARSE = tuple(COARSE.index({"mana": "tap", "cancel": "pass", "granted": "other", "none": "other"}.get(name, name))
                      for name in ACTION_NAMES)


def coarse_classes(batch, t, width):
    """[B, width] COARSE index of each candidate of step t (on the batch's device): the
    dataset's sidecar where a frame has one (`training4.coarse`), else the action column."""
    actions = action_classes(batch["cand_feat"][:, t, :width])
    classes = torch.tensor(ACTION_COARSE, device=actions.device)[actions]
    if "coarse_class" in batch:
        sidecar = batch["coarse_class"][:, t, :width].to(classes.device)
        classes = torch.where(sidecar >= 0, sidecar, classes)
    return classes


def act_directions(prepared, reference, batch, plans):
    """[R act / N pass decisions, of them the student acts, R pass / N act decisions,
    of them the student passes] at labelled priority decisions. N is the sidecar's
    recorded answer where the frame has one, else r1's top-1."""
    device = prepared[0][4].device
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    decision = (get("is_decision") & valid & (get("loss_weight") > 0) & (get("head") == 0)
                & (get("pending_kind") == PRIORITY))
    passes = COARSE.index("pass")
    counts = torch.zeros(4, dtype=torch.float64, device=device)
    for t, plan in enumerate(plans):
        if not plan.heads[0]:
            continue
        logits = prepared[t][1].float()
        mask = get("cand_mask")[:, t, :logits.shape[1]]
        rows = decision[:, t] & (mask.sum(-1) > 1)
        acts = coarse_classes(batch, t, logits.shape[1]).to(device, non_blocking=True) != passes
        n_idx = torch.where(mask, reference[t][1].float(), -1e9).argmax(-1)
        if "coarse_n_idx" in batch:
            recorded = get("coarse_n_idx")[:, t]
            n_idx = torch.where(recorded >= 0, recorded, n_idx)
        side = lambda index: acts.gather(1, index[:, None])[:, 0]
        r_act, n_act = side(get("single_idx")[:, t].clamp(min=0)), side(n_idx)
        s_act = side(torch.where(mask, logits, -1e9).argmax(-1))
        up, down = rows & r_act & ~n_act, rows & ~r_act & n_act
        counts += torch.stack((up.sum(), (up & s_act).sum(), down.sum(), (down & ~s_act).sum())).double()
    return counts


def coarse_confusion(prepared, batch, plans, scope):
    """[2, len(COARSE), len(COARSE)] counts of R's class x the student's top-1 class at
    labelled priority decisions: every one, then those where N != R."""
    device = prepared[0][4].device
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    decision = get("is_decision") & valid & (get("loss_weight") > 0) & (get("head") == 0)
    priority = get("pending_kind") == PRIORITY
    differs = get("n_ne_r")
    n = len(COARSE)
    table = torch.zeros(2, n * n, dtype=torch.float64, device=device)
    for t, plan in enumerate(plans):
        if not plan.heads[0]:
            continue
        logits = prepared[t][1].float()
        mask = get("cand_mask")[:, t, :logits.shape[1]]
        rows = decision[:, t] & priority[:, t] & (mask.sum(-1) > 1)
        classes = coarse_classes(batch, t, logits.shape[1]).to(device, non_blocking=True)
        r_class = classes.gather(1, get("single_idx")[:, t].clamp(min=0)[:, None])[:, 0]
        s_class = classes.gather(1, torch.where(mask, logits, -1e9).argmax(-1)[:, None])[:, 0]
        cell = r_class * n + s_class
        for layer, chosen in enumerate((rows, rows & differs[:, t])):
            table[layer] += torch.bincount(cell[chosen], minlength=n * n).double()
    return table.view(2, n, n)


CHAIN_FIELDS = ("decisions", "top1_r", "top1_n", "student_act", "r1_act")
CHAIN_ACTS = tuple(COARSE.index(name) for name in ("tap", "cast", "activate"))   # R's acts that make a chain


def pass_margin(logits, mask, classes):
    """[B] the best pass candidate's logit minus the best acting candidate's; nan where
    either side has no candidate."""
    passing = mask & (classes == COARSE.index("pass"))
    acting = mask & (classes != COARSE.index("pass"))
    best = lambda chosen: torch.where(chosen, logits, float("-inf")).max(-1).values
    margin = best(passing) - best(acting)
    return torch.where(passing.any(-1) & acting.any(-1), margin, torch.full_like(margin, float("nan")))


class ChainCounter:
    """R's acting chains at labelled priority decisions, read in trajectory order per lane.

    R acts where it taps, casts or activates. A chain is a run of R's acts in
    one trajectory; its `start` is the first after R passed (or at the
    trajectory's start), every later one is `mid` (teacher forced: the recorded
    states already hold R's earlier acts). Rows where R passes are `pass` and
    end a chain; R's land drops and other answers are `neutral` and neither
    start, continue nor end one (a land drop would otherwise count as a chain
    the student always starts). Per position, overall, by R's subtype and by
    R's coarse class, it counts the student's top-1 against R and N and whether
    the student and r1 act, anything but pass (N: the sidecar's recorded answer,
    else r1's top-1). Per chain it records whether the student, and r1, act at
    every step: in play one pass ends the sequence. The validation lanes carry
    each trajectory on.
    """

    def __init__(self, scope):
        self.scope = scope
        self.lanes = {}
        self.counts = {}
        self.margins = {}   # position -> key -> [student margin sum, r1 margin sum, rows]
        self.chains = []    # (length, student acts through, r1 acts through)

    def close(self, lane):
        chain = self.lanes.get(lane, {}).get("chain")
        if chain:
            self.chains.append((len(chain), all(s for s, _ in chain), all(n for _, n in chain)))
        self.lanes[lane] = dict(prev_act=False, chain=[])

    def add(self, position, keys, values):
        for key in keys:
            row = self.counts.setdefault(position, {}).setdefault(key, [0] * len(CHAIN_FIELDS))
            for i, value in enumerate(values):
                row[i] += value

    def observe(self, prepared, reference, batch, plans):
        device = prepared[0][4].device
        get = lambda key: batch[key].to(device, non_blocking=True)
        valid = get("step_valid")
        labelled = (get("is_decision") & valid & (get("loss_weight") > 0) & (get("head") == 0)
                    & (get("pending_kind") == PRIORITY))
        columns, gaps = [], []
        passes = COARSE.index("pass")
        for t, plan in enumerate(plans):
            b = labelled.shape[0]
            if not plan.heads[0]:
                columns.append(torch.full((b, 7), -1, dtype=torch.long, device=device))
                gaps.append(torch.full((b, 2), float("nan"), device=device))
                continue
            logits = prepared[t][1].float()
            mask = get("cand_mask")[:, t, :logits.shape[1]]
            classes = coarse_classes(batch, t, logits.shape[1]).to(device, non_blocking=True)
            r_idx = get("single_idx")[:, t].clamp(min=0)
            s_idx = torch.where(mask, logits, -1e9).argmax(-1)
            n_idx = torch.where(mask, reference[t][1].float(), -1e9).argmax(-1)
            if "coarse_n_idx" in batch:
                recorded = get("coarse_n_idx")[:, t]
                n_idx = torch.where(recorded >= 0, recorded, n_idx)
            of = lambda index: classes.gather(1, index[:, None])[:, 0]
            rows = labelled[:, t] & (mask.sum(-1) > 1)
            columns.append(torch.stack((rows.long(), r_idx, n_idx, s_idx, of(r_idx), of(n_idx), of(s_idx)), 1))
            gaps.append(torch.stack((pass_margin(logits, mask, classes), pass_margin(reference[t][1].float(), mask,
                                                                                     classes)), 1))
        table = torch.stack(columns, 1).cpu().tolist()          # [B, U, 7]
        margins = torch.stack(gaps, 1).cpu().tolist()           # [B, U, 2]: student, r1
        subtypes = (batch["distill_subtype"].cpu().tolist() if "distill_subtype" in batch
                    else [[-1] * len(plans)] * len(table))
        names = dict(enumerate(self.scope.subtype_names))
        for lane in torch.nonzero(batch["is_new_traj"].cpu())[:, 0].tolist():
            self.close(lane)
        for lane, steps in enumerate(table):
            state = self.lanes.setdefault(lane, dict(prev_act=False, chain=[]))
            for t, (row, r_idx, n_idx, s_idx, r_class, n_class, s_class) in enumerate(steps):
                if row != 1:
                    continue
                n_act, s_act = n_class != passes, s_class != passes
                if not state["prev_act"] and (r_class in CHAIN_ACTS or r_class == passes):
                    # A chance to start a chain: R starts one here or passes again.
                    self.add("opportunity", ("all",), (1, s_idx == r_idx, s_idx == n_idx, s_act, n_act))
                if r_class in CHAIN_ACTS:
                    position = "mid" if state["prev_act"] else "start"
                    state["chain"].append((s_act, n_act))
                    state["prev_act"] = True
                elif r_class == passes:
                    position = "pass"
                    self.close(lane)
                    state = self.lanes[lane]
                else:
                    position = "neutral"
                keys = ("all", f"subtype:{names.get(subtypes[lane][t], 'unknown')}", f"class:{COARSE[r_class]}")
                self.add(position, keys, (1, s_idx == r_idx, s_idx == n_idx, s_act, n_act))
                mine, theirs = margins[lane][t][:2]
                if mine == mine and theirs == theirs:   # both a pass and an act candidate on offer
                    for key in keys:
                        row = self.margins.setdefault(position, {}).setdefault(key, [0., 0., 0])
                        row[0], row[1], row[2] = row[0] + mine, row[1] + theirs, row[2] + 1

    def metrics(self):
        """Validation keys: overall rates per position, the breakdown, chain completion."""
        for lane in list(self.lanes):
            self.close(lane)
        rate = lambda row: {name: (row[i] / row[0] if row[0] else None) if i else row[0]
                            for i, name in enumerate(CHAIN_FIELDS)}
        out = {}
        for position in ("start", "mid", "pass", "neutral", "opportunity"):
            row = self.counts.get(position, {}).get("all", [0] * len(CHAIN_FIELDS))
            for name, value in rate(row).items():
                out[f"val_distill_chain_{position}_{name}"] = value
        # Each player's own chain-start rate where a chain can start (R's last answer was a
        # pass): R's is its starts over those rows; and each one's share of passes at priority.
        chances = out["val_distill_chain_opportunity_decisions"]
        out["val_distill_chain_opportunity_r_act"] = (out["val_distill_chain_start_decisions"] / chances
                                                      if chances else None)
        rows = [self.counts.get(position, {}).get("all", [0] * len(CHAIN_FIELDS))
                for position in ("start", "mid", "pass", "neutral")]
        total = sum(row[0] for row in rows)
        out.update(val_distill_priority_decisions=total,
                   val_distill_priority_pass_share_r=rows[2][0] / total if total else None,
                   val_distill_priority_pass_share_student=1 - sum(row[3] for row in rows) / total if total else None,
                   val_distill_priority_pass_share_r1=1 - sum(row[4] for row in rows) / total if total else None)
        out["val_distill_chain_by"] = {position: {key: rate(row) for key, row in sorted(rows.items())}
                                       for position, rows in self.counts.items()}
        # Pass drift: the pass logit's margin over the best act candidate (positive: passing leads).
        mean = lambda row, i: row[i] / row[2] if row[2] else None
        for position in ("start", "mid"):
            row = self.margins.get(position, {}).get("all", [0., 0., 0])
            out[f"val_distill_chain_{position}_pass_margin"] = mean(row, 0)
            out[f"val_distill_chain_{position}_pass_margin_r1"] = mean(row, 1)
        out["val_distill_chain_margin_by"] = {
            position: {key: dict(student=mean(row, 0), r1=mean(row, 1), rows=row[2]) for key, row in sorted(rows.items())}
            for position, rows in self.margins.items()}
        n = len(self.chains)
        out.update(val_distill_chains=n,
                   val_distill_chain_mean_length=sum(c[0] for c in self.chains) / n if n else None,
                   val_distill_chain_completion=sum(c[1] for c in self.chains) / n if n else None,
                   val_distill_chain_completion_r1=sum(c[2] for c in self.chains) / n if n else None)
        start_s, start_r1 = out["val_distill_chain_start_student_act"], out["val_distill_chain_start_r1_act"]
        pass_s, pass_r1 = out["val_distill_chain_pass_student_act"], out["val_distill_chain_pass_r1_act"]
        out["val_distill_pregate_start_act_ratio"] = start_s / start_r1 if start_r1 else None
        out["val_distill_pregate_pass_ratio"] = ((1 - pass_s) / (1 - pass_r1)
                                                 if pass_s is not None and pass_r1 is not None and pass_r1 < 1
                                                 else None)
        return out


def chain_start_drift(prepared, reference, batch, plans):
    """[student margin sum, r1 margin sum, rows] of the pass logit over the best acting
    candidate at a training batch's chain starts (the sidecar's `coarse_chain_start`)."""
    device = prepared[0][4].device
    flags = batch["coarse_chain_start"].to(device, non_blocking=True) & batch["step_valid"].to(device)
    out = torch.zeros(3, device=device)
    for t, plan in enumerate(plans):
        if not plan.heads[0]:
            continue
        logits = prepared[t][1].detach().float()
        mask = batch["cand_mask"][:, t, :logits.shape[1]].to(device, non_blocking=True)
        classes = coarse_classes(batch, t, logits.shape[1]).to(device, non_blocking=True)
        mine, theirs = pass_margin(logits, mask, classes), pass_margin(reference[t][1].float(), mask, classes)
        rows = flags[:, t] & ~mine.isnan() & ~theirs.isnan()
        out += torch.stack((torch.where(rows, mine, 0.).sum(), torch.where(rows, theirs, 0.).sum(), rows.sum()))
    return out


def drift_means(drift):
    student, r1, rows = drift.cpu().tolist()
    return dict(student=student / rows if rows else None, r1=r1 / rows if rows else None, rows=int(rows))


def pregate(metrics, min_ratio):
    """The offline pre-gate on a run's validation metrics: (passed, verdict).

    Fails where the student starts R's acting chains less than `min_ratio`
    times as often as r1 on the same rows (`val_distill_pregate_start_act_ratio`),
    or passes where R passes less than `min_ratio` times as often as r1
    (`val_distill_pregate_pass_ratio`), or where either is missing."""
    start = metrics.get("val_distill_pregate_start_act_ratio")
    passing = metrics.get("val_distill_pregate_pass_ratio")
    checks = dict(start_act_ratio=start, pass_ratio=passing)
    passed = all(value is not None and value >= min_ratio for value in checks.values())
    return passed, dict(passed=passed, min_ratio=min_ratio, **checks,
                        start_student_act=metrics.get("val_distill_chain_start_student_act"),
                        start_r1_act=metrics.get("val_distill_chain_start_r1_act"),
                        chain_completion=metrics.get("val_distill_chain_completion"),
                        chain_completion_r1=metrics.get("val_distill_chain_completion_r1"),
                        chain_starts=metrics.get("val_distill_chain_start_decisions"),
                        start_pass_margin=metrics.get("val_distill_chain_start_pass_margin"),
                        start_pass_margin_r1=metrics.get("val_distill_chain_start_pass_margin_r1"))


def answers(prepared, reference, batch, plans, scope, actions=None):
    """Single choices of the distilled kinds, against N (r1's own top-1) and R.

    Returns name -> [decisions, student = N, student = R, neither, r1 = R,
    (r1 != R) != the recorded N != R flag] for `distilled`, `out_of_scope` and
    `subtype:<name>`. With `actions` (a dict), counts by (subtype, R's action,
    N's action, the student's action) every decision where the student answers
    neither N nor R, the way the student left both.
    """
    device = prepared[0][4].device
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    decision = get("is_decision") & valid & (get("loss_weight") > 0)
    heads = get("head")
    in_kind, in_scope = scope.masks(batch, device)
    differs = get("n_ne_r")
    subtype = get("distill_subtype") if "distill_subtype" in batch else None
    names = [(-1, "unknown"), *enumerate(scope.subtype_names)]
    table = {}

    def add(name, chosen, student_n, student_r, r1_r, t):
        neither = ~student_n & ~student_r
        row = torch.stack((chosen.sum(), (chosen & student_n).sum(), (chosen & student_r).sum(),
                           (chosen & neither).sum(), (chosen & r1_r).sum(),
                           (chosen & (~r1_r != differs[:, t])).sum())).double()
        table[name] = table[name] + row if name in table else row

    for t, plan in enumerate(plans):
        if not plan.heads[0]:
            continue
        logits = prepared[t][1].float()
        mask = get("cand_mask")[:, t, :logits.shape[1]]
        genuine = decision[:, t] & (heads[:, t] == 0) & (mask.sum(-1) > 1) & in_kind[:, t]
        top1 = torch.where(mask, logits, -1e9).argmax(-1)
        n_top1 = torch.where(mask, reference[t][1].float(), -1e9).argmax(-1)
        r = get("single_idx")[:, t]
        student_n, student_r, r1_r = top1 == n_top1, top1 == r, n_top1 == r
        add("distilled", genuine & in_scope[:, t], student_n, student_r, r1_r, t)
        add("out_of_scope", genuine & ~in_scope[:, t], student_n, student_r, r1_r, t)
        if subtype is None:
            continue
        for code, name in names:
            add(f"subtype:{name}", genuine & (subtype[:, t] == code), student_n, student_r, r1_r, t)
        if actions is not None:
            classes = action_classes(get("cand_feat")[:, t, :logits.shape[1]])
            of = lambda index: classes.gather(1, index.clamp(min=0)[:, None])[:, 0].tolist()
            by_r, by_n, by_student = of(r), of(n_top1), of(top1)
            codes = dict(names)
            for row in (genuine & ~student_n & ~student_r).nonzero()[:, 0].tolist():
                key = (codes.get(int(subtype[row, t]), "unknown"), ACTION_NAMES[by_r[row]],
                       ACTION_NAMES[by_n[row]], ACTION_NAMES[by_student[row]])
                actions[key] = actions.get(key, 0) + 1
    return table


def agreement(prepared, batch, plans, scope, scalar_tolerance):
    """name -> [decisions, N != R, agree with R, agree with R at N != R] (device float64).

    `distilled` pools the scope; `out_of_scope` is the same kinds' other subtypes;
    `subtype:<name>` (and `subtype:unknown`) splits every decision of the kinds.
    Single choices agree when the argmax is R. Subsets and pairs agree when the
    threshold-0 decode equals R on every free entry (the min/max choice limits
    the actor's decoder applies are not in the frame). Scalars agree within
    `scalar_tolerance` of R's normalized value.
    """
    device = prepared[0][4].device
    get = lambda key: batch[key].to(device, non_blocking=True)
    valid = get("step_valid")
    decision = get("is_decision") & valid & (get("loss_weight") > 0)
    heads = get("head")
    in_kind, in_scope = scope.masks(batch, device)
    differs = get("n_ne_r")
    subtype = batch["distill_subtype"].to(device) if "distill_subtype" in batch else None
    table = {}

    def add(name, chosen, agree, t):
        row = torch.stack((chosen.sum(), (chosen & differs[:, t]).sum(),
                           (chosen & agree).sum(), (chosen & agree & differs[:, t]).sum())).double()
        table[name] = table[name] + row if name in table else row

    for t, plan in enumerate(plans):
        for kind in range(4):
            if not plan.heads[kind]:
                continue
            selected = decision[:, t] & (heads[:, t] == kind)
            if kind == 0:
                logits = prepared[t][1].float()
                mask = get("cand_mask")[:, t, :logits.shape[1]]
                genuine = selected & (mask.sum(-1) > 1)
                agree = torch.where(mask, logits, -1e9).argmax(-1) == get("single_idx")[:, t]
                add("distilled", genuine & in_scope[:, t], agree, t)
                add("out_of_scope", genuine & in_kind[:, t] & ~in_scope[:, t], agree, t)
                add("single", genuine & ~in_kind[:, t], agree, t)
                if subtype is not None:
                    for code, name in [(-1, "unknown"), *enumerate(scope.subtype_names)]:
                        add(f"subtype:{name}", genuine & in_kind[:, t] & (subtype[:, t] == code), agree, t)
            elif kind in (1, 2):
                prefix = "cand" if kind == 1 else "pair"
                logits = prepared[t][kind].float()
                width = logits.shape[1]
                eligible = get(prefix + "_mask")[:, t, :width] & (get(prefix + "_free_mask")[:, t, :width] > 0)
                target = get(prefix + "_target_mask")[:, t, :width] > .5
                agree = ~(eligible & ((logits > 0) != target)).any(-1)
                add("subset" if kind == 1 else "pair", selected & eligible.any(-1), agree, t)
            else:
                agree = (prepared[t][3].float() - get("scalar_target")[:, t]).abs() <= scalar_tolerance
                add("scalar", selected, agree, t)
    return table


@torch.no_grad()
def evaluate_distill(net, reference, loader, device, scope, board_chunk, scalar_tolerance,
                     thinking_steps=2, bf16=None, think=None, actions=None, progress=None, observe=None):
    """Held-out agreement with R by group and subtype and every KL guard, over whole trajectories.

    With the thinking terms' configuration `think`, also the share of in-scope
    N != R decisions each deliberation step answers as R
    (`val_distill_think_top1_r_at_n_ne_r_by_step`; step 0 is the heads' first
    proposal, the last step is the net's answer). For the distilled kinds'
    single choices, by group and subtype, also the student's agreement with N
    (r1's own top-1, the out-of-scope target), the share it answers neither N
    nor R, r1's agreement with R and where r1's top-1 disagrees with the
    recorded N != R flag (`answers`; `actions` collects how it left both).
    Where the dataset has a probe sidecar (always under `scope.advantage`), the
    student's and r1's answers where the probes put R or N ahead and the
    student's shift against the probes' (`val_adv_*`,
    `distill_probe.AdvantageCounter`); under `scope.advantage` they also give
    `val_distill_selection`.
    """
    think_steps = None
    from .prefetch import prefetch_batches
    net.eval()
    bf16 = device.type == "cuda" and torch.cuda.is_bf16_supported() if bf16 is None else bf16
    table = {}
    answer_table = {}
    confusion = torch.zeros(2, len(COARSE), len(COARSE), dtype=torch.float64, device=device)
    directions = torch.zeros(4, dtype=torch.float64, device=device)
    chains = ChainCounter(scope) if PRIORITY in scope.kinds else None
    probes = AdvantageCounter(scope) if PRIORITY in scope.kinds else None
    house = None
    if scope.teacher == "house":
        from .house import HouseCounter
        house = HouseCounter(scalar_tolerance)
    stats = torch.zeros(STATS_WIDTH, device=device)
    value = torch.zeros(3, dtype=torch.float64, device=device)
    hidden = None
    reference.hidden = None
    for host, staged in prefetch_batches(loader, device, getattr(net, "gpu_prefetch", 0)):
        plans = plan_batch(host, 0, thinking_steps)
        batch = transfer_batch(host, device) if staged is None else staged
        b = batch["step_valid"].shape[0]
        new = batch["is_new_traj"].to(device, non_blocking=True)
        hidden = (torch.zeros(b, M.RECURRENT_SLOTS, net.cfg.d_model, device=device)
                  if hidden is None or hidden.shape[0] != b else
                  torch.where(new[:, None, None], torch.zeros_like(hidden), hidden))
        with torch.autocast(device_type=device.type, dtype=torch.bfloat16, enabled=bf16):
            net.clear_structure()
            net.prepare_structure((host["entity_rules"], host["cand_rules"]))
            with reference.capturing(net):
                boards = encode_boards(net, batch, board_chunk, device)
        prepared, final = deferred_readouts(net, batch, boards, hidden, plans, net.decision_chunk, device, bf16)
        ref = reference.outputs(host, batch, boards, hidden, plans, device, bf16, board_chunk)
        _, step_stats = distill_loss(prepared, ref, batch, plans, scope)
        stats += step_stats
        if think is not None and any(frame is not None for frame in net.thinking_frames or ()):
            _, by_step = thinking_terms(net.thinking_frames, ref, batch, plans, scope, think)
            if by_step.numel() > 2:
                think_steps = by_step if think_steps is None else think_steps + by_step
        for name, row in agreement(prepared, batch, plans, scope, scalar_tolerance).items():
            table[name] = table[name] + row if name in table else row
        for name, row in answers(prepared, ref, batch, plans, scope, actions).items():
            answer_table[name] = answer_table[name] + row if name in answer_table else row
        if PRIORITY in scope.kinds:
            confusion = confusion + coarse_confusion(prepared, batch, plans, scope)
            directions = directions + act_directions(prepared, ref, batch, plans)
            chains.observe(prepared, ref, batch, plans)
            probes.observe(prepared, ref, batch, plans)   # where the dataset has a probe sidecar
        if house is not None:
            house.observe(prepared, ref, batch, plans)
        if observe is not None:   # a caller's own reading of the batch, in trajectory order per lane
            observe(prepared, ref, batch, plans)
        if progress is not None:
            progress(answer_table)
        valid = batch["step_valid"].to(device)
        mine = torch.stack([p[4].float() for p in prepared], 1)
        theirs = torch.stack([p[4].float() for p in ref], 1)
        value += torch.stack((((mine - batch["outcome"].to(device)).square() * valid).sum(),
                              ((mine - theirs).square() * valid).sum(), valid.sum())).double()
        net.clear_structure()
        hidden = final
    metrics = {f"val_distill_{name}": term for name, term in term_means(stats).items()}
    for group in GROUPS:
        table.setdefault(group, torch.zeros(4, dtype=torch.float64))
    for group, row in sorted(table.items()):
        count, differ, agree, agree_differ = row.cpu().tolist()
        metrics[f"val_distill_{group}_decisions"] = int(count)
        metrics[f"val_distill_{group}_n_ne_r"] = int(differ)
        metrics[f"val_distill_{group}_top1_r"] = agree / count if count else None
        metrics[f"val_distill_{group}_top1_r_at_n_ne_r"] = agree_differ / differ if differ else None
        metrics[f"val_distill_{group}_n_top1_r"] = (count - differ) / count if count else None
    for group, row in sorted(answer_table.items()):
        count, *rest = row.cpu().tolist()
        for name, hits in zip(ANSWER_COLUMNS[2:], rest[1:]):
            if name != "top1_r":   # the agreement table reports it
                metrics[f"val_distill_{group}_{name}"] = hits / count if count else None
        metrics[f"val_distill_{group}_top1_n"] = rest[0] / count if count else None
    if PRIORITY in scope.kinds:
        # Rows: R's coarse class; columns: the student's top-1 class (`val_distill_coarse_classes`).
        counts = confusion.cpu()
        metrics["val_distill_coarse_classes"] = list(COARSE)
        for layer, suffix in enumerate(("", "_n_ne_r")):
            total = float(counts[layer].sum())
            metrics[f"val_distill_coarse_confusion{suffix}"] = counts[layer].long().tolist()
            metrics[f"val_distill_coarse_top1_r{suffix}"] = (float(counts[layer].diagonal().sum()) / total
                                                             if total else None)
        up, up_act, down, down_pass = directions.cpu().tolist()
        metrics.update(val_distill_act_r_act_n_pass_decisions=int(up),
                       val_distill_act_r_act_n_pass_student_act=up_act / up if up else None,
                       val_distill_act_r_pass_n_act_decisions=int(down),
                       val_distill_act_r_pass_n_act_student_pass=down_pass / down if down else None)
        metrics.update(chains.metrics())
        if probes.seen or scope.advantage:
            metrics.update(probes.metrics())
    if think_steps is not None:
        metrics["val_distill_think_n_ne_r"] = int(think_steps[-1])
        metrics["val_distill_think_top1_r_at_n_ne_r_by_step"] = (
            (think_steps[:-1] / think_steps[-1]).tolist() if float(think_steps[-1]) > 0 else None)
    sse, drift, frames = value.cpu().tolist()
    metrics.update(val_mse=sse / max(frames, 1), val_value_mse_to_r1=drift / max(frames, 1), val_frames=int(frames),
                   val_distill_selection=metrics["val_distill_distilled_top1_r_at_n_ne_r"])
    if house is not None:
        # The house teacher selects on the gate's agreement with H at N != H (priority by group).
        metrics.update(house.metrics(metrics))
        metrics["val_distill_kinds"] = kind_means(stats.cpu())
        metrics["val_distill_selection"] = metrics["val_house_gate_top1_n_ne_h"]
    if scope.advantage:
        # Selects where the student answers whichever of R and N the probes put ahead.
        metrics["val_distill_selection"] = metrics["val_adv_selection"]
    return metrics
