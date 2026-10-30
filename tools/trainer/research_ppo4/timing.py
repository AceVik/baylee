"""Host wall-clock phase measurements for the PyTorch worker."""
from contextlib import contextmanager
from time import perf_counter


class PhaseTimings:
    def __init__(self):
        self.started = perf_counter()
        self.seconds = {}

    @contextmanager
    def measure(self, name):
        started = perf_counter()
        try:
            yield
        finally:
            self.seconds[name] = self.seconds.get(name, 0.) + perf_counter() - started

    def snapshot(self):
        # Do not synchronize CUDA: instrumentation must preserve execution behavior.
        # The final report's own write and subsequent cleanup cannot time themselves.
        return dict(clock='perf_counter', cuda_synchronized=False,
                    interpretation='host_wall_boundaries_may_include_prior_async_work_not_kernel_timings',
                    scope='main_entry_to_snapshot_excludes_imports_final_report_write_and_cleanup',
                    elapsed_seconds=perf_counter() - self.started,
                    phase_seconds=dict(self.seconds))
