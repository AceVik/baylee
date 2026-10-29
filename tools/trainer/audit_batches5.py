"""Compatibility entry point for bounded Torch batch measurements."""
from batch_audit5.worker import describe, main, measure, padded_bytes, sample_indices

if __name__ == '__main__':
    main()
