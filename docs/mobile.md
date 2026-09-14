# The client on a phone

Three surfaces, one client, and one requirement that decided the whole shape:
**dev-control has to reach the client while it runs on the phone.** That rules
out the browser build as the answer — `devctl.rs` is a `TcpListener`, and wasm
has no sockets — so Android is a real APK and iOS a real `.app`, and the
browser stays what it was: a fallback with no remote control.

The loopback bind stays exactly as it was, and that is the point. `adb forward`
connects this machine to the **device's** own 127.0.0.1, and the iOS simulator
shares the host's network stack outright, so a harness that listens on
loopback is reachable from here on both without ever being reachable from the
network.

## What is installed, and where

`scripts/mobile/android-env.sh` names everything an Android build needs —
`JAVA_HOME` (Homebrew's JDK 17; `/usr/libexec/java_home` does not find it),
`ANDROID_HOME`, `ANDROID_NDK_ROOT`, the NDK's `clang` as the C/C++ compiler and
linker for `aarch64-linux-android`, and the SDK's own `cmake` on `PATH`.
**Source it, never run it**, and never put it in a shell profile: an
`ANDROID_NDK_ROOT` left set in every terminal is how a desktop build picks up a
cross compiler by accident.

Beyond the SDK that was already here, this needs two things:
`rustup target add aarch64-linux-android aarch64-apple-ios-sim` and
`cargo install cargo-apk`.

## Android

```sh
scripts/mobile/android-build.sh                          # LAN address, dev-control on
scripts/mobile/android-build.sh --gateway http://10.0.2.2:28766 --run
adb install -r target/debug/apk/baylee.apk
adb shell am start -n local.baylee.client/android.app.NativeActivity
```

Three things about that are worth knowing before changing any of them.

**An Android app never calls `main`.** It loads `libbaylee_client_android.so`
and calls `android_main`, which is why `crates/baylee-client-android` exists at
all: a crate's `crate-type` cannot be made conditional on the target, so
`cdylib` on `baylee-client` itself would link a half-gigabyte shared object on
every desktop `cargo build --workspace`. The shim declares **every** dependency
under `cfg(target_os = "android")`, so off Android it is an empty `.so` that
links in no time. `#[bevy_main]` writes the entry point, and it needs bevy's
`android-native-activity` feature — without it `android-activity` refuses to
compile at all, with six `cannot find type …Impl` errors and a
`compile_error!` explaining why.

**An Android app inherits no environment.** `BAYLEE_GATEWAY` and
`BAYLEE_DEV_CONTROL` cannot be read at runtime there, so on Android and iOS
they fall back to `option_env!` — compiled in, and *after* the two runtime
lookups, so the iOS simulator's `SIMCTL_CHILD_…` still wins. Changing the
gateway therefore means a rebuild, and `crates/baylee-client/build.rs` is what
makes cargo notice: without its `rerun-if-env-changed` the phone would keep
dialling the address from the previous build with nothing anywhere saying so.

**The address depends on where the client runs**, and this is the part that is
guessed wrong:

| where | the gateway is |
| --- | --- |
| Pixel on WiFi | `http://<this machine's LAN IP>:28766` |
| Pixel over USB | `http://127.0.0.1:28766`, after `adb reverse tcp:28766 tcp:28766` |
| Android emulator | `http://10.0.2.2:28766` |
| iOS simulator | `http://127.0.0.1:28766` |

`scripts/mobile/serve-lan.sh` starts the gateway and one agent and prints all
four; `cargo apk run` applies the `reverse_port_forward` table in the shim's
manifest, so over a cable there is nothing to bake at all.

### dev-control over the cable

```sh
adb forward tcp:28773 tcp:28770
curl -s localhost:28773/health
```

28773 on this side because 28770 and 28772 are taken by clients running on this
machine. Screenshots go to the one directory on the device the app may write
to, and come back through `run-as` — which is what `debuggable = true` in the
shim's manifest is for, rather than cosmetics:

```sh
curl -s -XPOST localhost:28773/screenshot \
     -d '{"path":"/data/data/local.baylee.client/files/shot.png"}'
adb exec-out run-as local.baylee.client cat files/shot.png > shot.png
```

A relaunch needs `adb shell am force-stop local.baylee.client` first. A
NativeActivity process outlives its activity, so starting it again re-enters
`android_main` in a process that already has an event loop, and bevy stops with
`Failed to build event loop: RecreationAttempt`.

## iOS simulator

```sh
scripts/mobile/ios-sim-run.sh --device "iPhone 15 Pro"
```

No Xcode project, deliberately: a simulator app is a directory with a Mach-O
binary and an `Info.plist`, nothing is signed, and `simctl install` takes it as
it is. A project would be a second place for the build settings to live in.
dev-control needs no forwarding here — the client binds *this* machine's
loopback.

**A phone's run loop is not a desktop's**, and this is the one line of code
the two phone targets needed. `WinitSettings::default()` is `game()`, which
asks for `UpdateMode::Continuous`; on iOS winit hands the thread to
`UIApplicationMain`, and nothing there wakes the run loop again — the client
drew the table once and then sat in `CFRunLoopRun` at 3 % CPU, alive, never
asked for another frame, with every dev-control request answering
`{"error":"no answer within 10s"}` because `pump` runs once per frame and
there were no more frames. `standalone::run` now inserts
`WinitSettings::mobile()` on android and ios: a 1/60 s `Reactive` wait, which
is a timer the run loop honours by itself, and which bevy's own
`winit_config.rs` names "default settings for mobile".

## The browser, as the fallback

`trunk serve index.html --release` from `crates/baylee-client/`, then the
phone's browser at `http://<LAN IP>:8080/?gateway=http://<LAN IP>:28766`. It
has no dev-control and it has one trap: the client renders through **WebGPU**,
which browsers only expose in a secure context, and a LAN `http://` origin is
not one. Chrome on Android will treat it as one if the origin is listed under
`chrome://flags/#unsafely-treat-insecure-origin-as-secure`; Safari has no such
switch, so on iOS this needs HTTPS or the native build.

## What is verified, and what is not

Measured on 14.09.2026 on this machine:

- The APK builds (2 m 35 s), installs, launches, and **opens its dev-control
  socket** — `dev control: listening on http://127.0.0.1:28770` in `logcat`,
  and requests forwarded with `adb forward` reach it.
- The fonts are in the APK at `assets/fonts/…`, which is why
  `standalone::asset_root` returns `""` on Android: bevy reads through the
  APK's `AssetManager`, whose root *is* that directory.
- **The iOS simulator is a working dev-control surface.** The app runs at
  about 60 fps (`/health` reported frame 12094 and, three seconds later,
  12276), answers every route with no forwarding at all, and `/screenshot`
  writes a 3840×2160 picture of the lobby. That took `WinitSettings::mobile()`
  — see above; before it, the same build drew one frame and stopped.

The window is **not** the phone's screen, and the picture is not what a player
would see. `/health` reports 1280×720 at scale 3, which is
`WindowResolution::default()` — bevy's own 1280×720 — and not the iPhone 15
Pro's 393×852 logical. So the lobby in that screenshot is laid out in
`Metrics`' *desktop* frame, landscape, and what the simulator's own screen
makes of it was not measured. The run loop is fixed and the harness reaches
the app; window sizing on iOS is the next question, and it is a different one.

One thing does **not** work yet, in the renderer and not on real hardware,
which is the one surface untested:

- **The Android emulator loses the device.** With `-gpu host` the guest gets
  the host GPU through gfxstream (`AdapterInfo … "Apple M1 Max", driver:
  "MoltenVK"`), and one frame after `Creating new window baylee` it reports
  `DeviceLost … (driver implementation is at fault)`, then panics in
  `wgpu-hal … swapchain: Trying to destroy a SwapchainAcquireSemaphore that is
  still in use by a SurfaceTexture`. `-gpu swiftshader_indirect` did not help —
  the emulator logged `option: host` anyway and then wedged at 0 % CPU without
  ever opening its adb port. A physical Pixel has a real Vulkan driver and is
  the thing to try before spending another hour here.

The `WinitSettings::mobile()` above is not a candidate fix for it: that was a
run loop nobody woke, and this is a device the driver hands back. The obvious
next lever is not one either, and that is worth writing down before someone
spends an hour on it — `-feature -Vulkan` would push the guest onto GLES, which
wgpu does support on Android, except that the SDK's own
`emulator/lib/advancedFeatures.ini` already says `Vulkan = off` and there is no
override in `~/.android/`, and the guest reported a Vulkan adapter anyway. So
whatever turns it on comes from the system image or from gfxstream itself, and
the flag is not the switch it looks like.
