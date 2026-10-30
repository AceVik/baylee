"""Action-identity and complete-history numerical parity for tensor probes."""

import math

import torch


LP_ATOL = 2e-5
HIDDEN_ATOL = 2e-5
HIDDEN_RTOL = 2e-4
KL_ATOL = 1e-7
TARGET_KL = .02


def compare(reference, actual):
    if set(reference.actions) != set(actual.actions) or set(reference.frames) != set(actual.frames):
        raise ValueError("Complete-history/action identities differ")
    per_history = {}
    per_action = []
    passed = True
    for key in sorted(reference.frames):
        rh, rv = reference.frames[key]
        ah, av = actual.frames[key]
        if not bool(torch.isfinite(ah).all()) or not math.isfinite(av):
            raise FloatingPointError("Nonfinite parity output")
        hidden_delta = float((ah-rh).abs().max())
        value_delta = abs(av-rv)
        hidden_pass = bool(torch.allclose(ah, rh, atol=HIDDEN_ATOL, rtol=HIDDEN_RTOL))
        value_pass = value_delta <= HIDDEN_ATOL + HIDDEN_RTOL * abs(rv)
        history = per_history.setdefault(key[:2], dict(frames=0, choices=0,
                     hidden_max_abs=0., value_max_abs=0., logprob_max_abs=0., passed=True))
        history['frames'] += 1
        history['hidden_max_abs'] = max(history['hidden_max_abs'], hidden_delta)
        history['value_max_abs'] = max(history['value_max_abs'], value_delta)
        history['passed'] &= hidden_pass and value_pass
        if key in reference.actions:
            expected, recorded = reference.actions[key]
            current, actual_recorded = actual.actions[key]
            if recorded != actual_recorded:
                raise ValueError("Recorded executed-action likelihood changed")
            if not math.isfinite(current):
                raise FloatingPointError("Nonfinite action likelihood")
            delta = abs(current-expected)
            history['choices'] += 1
            history['logprob_max_abs'] = max(history['logprob_max_abs'], delta)
            history['passed'] &= delta <= LP_ATOL
            per_action.append(dict(game_id=key[0], seat=key[1], cursor=key[2],
                                   reference_logprob=expected, current_logprob=current,
                                   recorded_logprob=recorded, abs_delta=delta,
                                   passed=delta <= LP_ATOL))
        passed &= history['passed']
    rkl = reference.report['collection_behavior_kl']
    akl = actual.report['collection_behavior_kl']
    kl_delta = abs(akl-rkl)
    same_acceptance = (rkl <= TARGET_KL) == (akl <= TARGET_KL)
    near_limit = min(abs(rkl-TARGET_KL), abs(akl-TARGET_KL)) <= KL_ATOL
    passed &= math.isfinite(akl) and kl_delta <= KL_ATOL and same_acceptance and not near_limit
    return dict(passed=bool(passed), kl_abs_delta=kl_delta,
                same_acceptance=same_acceptance, near_kl_limit=near_limit,
                logprob_max_abs=max(row['abs_delta'] for row in per_action),
                per_history=[dict(game_id=key[0], seat=key[1], **value)
                             for key, value in sorted(per_history.items())],
                per_action=per_action)


def validate_coverage(snapshot, expected_histories, expected_frames, expected_choices):
    histories = {key[:2] for key in snapshot.frames}
    if (len(histories) != expected_histories or len(snapshot.frames) != expected_frames
            or len(snapshot.actions) != expected_choices):
        raise ValueError("Probe did not audit the complete declared collection")
