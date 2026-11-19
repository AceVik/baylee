// The smallest C ABI over XRT's C++ API that the Rust host needs: open a
// hardware context for an xclbin, allocate and sync buffer objects, and run
// the NPU kernel (opcode 3, instruction BO, A, B, C) as mlir-aie's test.cpp
// does. Every function catches the C++ exceptions XRT throws and reports
// them as a message; no logic lives here.

#include <xrt/xrt_bo.h>
#include <xrt/xrt_device.h>
#include <xrt/xrt_hw_context.h>
#include <xrt/xrt_kernel.h>
#include <xrt/experimental/xrt_xclbin.h>

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <exception>
#include <stdexcept>
#include <string>

struct npu_ctx {
  xrt::device device;
  xrt::xclbin xclbin;
  xrt::hw_context context;
  xrt::kernel kernel;
};

struct npu_bo {
  xrt::bo bo;
};

struct npu_run {
  xrt::run run;
};

namespace {

void set_err(char *err, std::size_t cap, const char *msg) {
  if (err == nullptr || cap == 0)
    return;
  std::strncpy(err, msg, cap - 1);
  err[cap - 1] = '\0';
}

} // namespace

extern "C" {

npu_ctx *npu_open(const char *xclbin_path, const char *kernel_prefix,
                  char *err, std::size_t cap) {
  try {
    auto *c = new npu_ctx();
    try {
      c->device = xrt::device(0u);
      c->xclbin = xrt::xclbin(std::string(xclbin_path));
      std::string name;
      for (auto &k : c->xclbin.get_kernels()) {
        if (k.get_name().rfind(kernel_prefix, 0) == 0) {
          name = k.get_name();
          break;
        }
      }
      if (name.empty())
        throw std::runtime_error("no kernel with the given prefix in xclbin");
      c->device.register_xclbin(c->xclbin);
      c->context = xrt::hw_context(c->device, c->xclbin.get_uuid());
      c->kernel = xrt::kernel(c->context, name);
      return c;
    } catch (...) {
      delete c;
      throw;
    }
  } catch (const std::exception &e) {
    set_err(err, cap, e.what());
    return nullptr;
  }
}

void npu_close(npu_ctx *c) { delete c; }

// `instr` != 0 allocates the cacheable instruction BO; otherwise a host-only
// data BO. `arg` is the kernel argument whose memory group the BO joins.
npu_bo *npu_bo_alloc(npu_ctx *c, std::size_t bytes, int instr, int arg,
                     char *err, std::size_t cap) {
  try {
    auto flags = instr != 0 ? XCL_BO_FLAGS_CACHEABLE : XRT_BO_FLAGS_HOST_ONLY;
    return new npu_bo{xrt::bo(c->device, bytes, flags, c->kernel.group_id(arg))};
  } catch (const std::exception &e) {
    set_err(err, cap, e.what());
    return nullptr;
  }
}

void *npu_bo_map(npu_bo *b) {
  try {
    return b->bo.map<void *>();
  } catch (...) {
    return nullptr;
  }
}

int npu_bo_sync(npu_bo *b, int to_device, char *err, std::size_t cap) {
  try {
    b->bo.sync(to_device != 0 ? XCL_BO_SYNC_BO_TO_DEVICE
                              : XCL_BO_SYNC_BO_FROM_DEVICE);
    return 0;
  } catch (const std::exception &e) {
    set_err(err, cap, e.what());
    return -1;
  }
}

void npu_bo_free(npu_bo *b) { delete b; }

npu_run *npu_run_new(npu_ctx *c, npu_bo *instr, std::uint32_t ninstr,
                     npu_bo *a, npu_bo *b, npu_bo *out, char *err,
                     std::size_t cap) {
  try {
    xrt::run run(c->kernel);
    run.set_arg(0, std::uint64_t{3});
    run.set_arg(1, instr->bo);
    run.set_arg(2, ninstr);
    run.set_arg(3, a->bo);
    run.set_arg(4, b->bo);
    run.set_arg(5, out->bo);
    return new npu_run{std::move(run)};
  } catch (const std::exception &e) {
    set_err(err, cap, e.what());
    return nullptr;
  }
}

// Starts the run and waits for it; returns the final ert_cmd_state, or -1
// when XRT threw.
int npu_run_exec(npu_run *r, char *err, std::size_t cap) {
  try {
    r->run.start();
    return static_cast<int>(r->run.wait());
  } catch (const std::exception &e) {
    set_err(err, cap, e.what());
    return -1;
  }
}

void npu_run_free(npu_run *r) { delete r; }

int npu_cmd_state_completed() { return static_cast<int>(ERT_CMD_STATE_COMPLETED); }

} // extern "C"
