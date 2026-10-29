"""FP32 local scratchpad diagnostics; these are not search or strength metrics."""
import torch
import model4 as M


@torch.no_grad()
def diagnose_depth_steps(net, inputs, depths=(1, 2, 4, 8)):
    """Compare one observation at several K, always starting from the same H.

    The caller supplies permitted observation tensors in INPUT_NAMES order.
    Candidate argmax changes describe categorical heads only; binary threshold
    changes describe subset logits, not constrained sampler action changes.
    """
    was_training = net.training
    net.eval()
    latent = []
    def capture(module, arguments, output):
        delta = output.float() - arguments[0].float()
        latent.append({"rms": float(delta.square().mean().sqrt()),
                       "max_abs": float(delta.abs().max())})
    handle = net.thinking_block.register_forward_hook(capture)
    outputs, report = {}, {}
    try:
        for k in depths:
            if type(k) is not int or not 1 <= k <= net.cfg.max_thinking_steps:
                raise ValueError("Diagnostic decision K outside supported positive range")
            latent.clear()
            values = list(inputs)
            values[-1] = torch.full_like(inputs[-1], k)
            result = net(*values)
            outputs[k] = result
            report[str(k)] = {"latent_steps": list(latent)}
        baseline = outputs[depths[0]]
        for k, result in outputs.items():
            metrics = report[str(k)]
            metrics["persistent_h_max_abs_from_baseline"] = float((result[0] - baseline[0]).abs().max())
            for index, name, mask_name in ((1, "candidate", "cand_mask"), (2, "pair", "pair_mask")):
                mask = inputs[M.INPUT_NAMES.index(mask_name)].bool()
                delta = (result[index].float() - baseline[index].float())[mask]
                metrics[name + "_logit_delta_rms"] = float(delta.square().mean().sqrt()) if delta.numel() else 0.0
                changed = ((result[index] > 0) != (baseline[index] > 0)) & mask
                metrics[name + "_binary_threshold_changes"] = int(changed.sum())
            cand_mask = inputs[M.INPUT_NAMES.index("cand_mask")].bool()
            has_choice = cand_mask.any(-1)
            metrics["categorical_argmax_changes"] = int(((result[1].argmax(-1) != baseline[1].argmax(-1)) & has_choice).sum())
            metrics["scalar_delta_rms"] = float((result[3] - baseline[3]).float().square().mean().sqrt())
        return {"baseline_k": depths[0], "per_k": report}
    finally:
        handle.remove()
        net.train(was_training)
