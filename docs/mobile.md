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

Four things about that are worth knowing before changing any of them.

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

**Landscape belongs to the manifest, not to the client.** A table is played
across and not down, and the activity is placed before a frame is ever drawn —
so a client that asked for landscape once it was running would start portrait
and turn. `[package.metadata.android.application.activity]` carries
`orientation = "sensorLandscape"` (both landscape directions, neither portrait
one, and the device's own rotation lock is not consulted); the iOS side is
`UISupportedInterfaceOrientations` in `ios-sim-run.sh`'s plist, saying the
same thing. What makes it safe on Android is cargo-apk's default
`configChanges`, packaged as `0x4a0` —
`orientation|keyboardHidden|screenSize` — which keeps the activity through a
rotation rather than recreating it, and a recreated `NativeActivity` re-enters
`android_main` in a process that still has an event loop. Read it back out of
a built APK rather than trusting the manifest source:

```sh
"$ANDROID_HOME"/build-tools/*/aapt2 dump xmltree \
    --file AndroidManifest.xml target/debug/apk/baylee.apk | grep -i orientation
# android:screenOrientation(0x0101001e)=6      # 6 is sensorLandscape
```

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

**Install with `--no-incremental`.** `adb install -r` on a phone that supports
it streams the APK and lets pages arrive on demand, and this one does not
survive it: the app dies before bevy starts with `Fatal signal 7 (SIGBUS),
code 2 (BUS_ADRERR)` in `__dl_load_library` — the dynamic linker reading a
page of `libbaylee_client_android.so` that is not there yet. It looks exactly
like a crash in the client and is not one.

### Wireless debugging

The phone's *pairing* port is not its *connect* port, and only mDNS knows
either. With "Wireless debugging" open on the phone and the pairing dialog
showing, `adb mdns services` advertises both:

```sh
adb mdns services
# …  _adb-tls-pairing._tcp   192.168.0.92:39295   # only while the dialog is open
# …  _adb-tls-connect._tcp   192.168.0.92:37139
adb pair 192.168.0.92:39295 <the six digits on the phone>
```

Then stop. The mDNS transport connects on its own, and an `adb connect` on top
of it gives one phone **two** transports, after which every command answers
"more than one device/emulator"; `adb disconnect <ip>:<connect port>` removes
the manual one and leaves the mDNS one.

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

- **The client runs on a physical phone and is drivable from here.** A Pixel
  11 Pro XL (kodiak, Android 17) over wireless debugging: the APK builds
  (2 m 35 s), installs, launches, and `curl -s localhost:28773/health` through
  `adb forward tcp:28773 tcp:28770` answers frame 278 on a 920.21 × 443.08
  window at scale 2.4375 — the phone's own 2243 × 1080 landscape screen, not
  a default. `/screenshot` writes a 2243 × 1080 PNG that `run-as` brings
  back. That is the requirement at the top of this file, met on real
  hardware.
- It took one line to get there, and the line is not about the client.
  **The Pixel 11 falls one generation outside bevy's own carve-out.** Its GPU
  is a `"PowerVR C-Series CXTP-48-1536 MC1"`, and PowerVR's SPIR-V compiler
  aborts (`SIGABRT` in the "Async Compute T" thread, inside
  `libufwriter.so`'s `spvcompiler::getMangledImageTypeString`, called from
  `IMG_vkCreateComputePipelines`) on a sampled image inside a compute shader
  — which is `mesh_preprocess.wgsl`'s `depth_pyramid: texture_2d<f32>`, and
  that shader exists only under `GpuPreprocessingMode::Culling`. Bevy already
  holds this family to `PreprocessingOnly`, but recognises it by comparing
  the adapter name against the literal `"PowerVR D-Series DXT-48-1536 MC1"`,
  the Pixel 10's. `standalone::run` disables `INDIRECT_FIRST_INSTANCE` on
  Android, which is the one feature `GpuPreprocessingSupport::from_world`
  reads as `culling_feature_support` and which nothing else in bevy reads at
  all, so it asks for that mode and clamps no limit. `logcat` then says `Some
  GPU preprocessing are limited on this device.` instead of `fully
  supported`, and **that line is the measurement** — read it before blaming
  or crediting anything else. Worth reporting upstream: a
  `starts_with("PowerVR")` in `get_pixel10_driver_version` would cover the
  family.
- **The same driver cannot resolve 4x multisampling either**, and that one is
  worth knowing before it costs somebody a day. `Msaa` defaults to
  `Sample4`; on a tiler the end-of-pass resolve is where tile memory is
  written back, and the damage falls on whatever was written last — the UI
  pass. The lobby drew three or four oversized glyphs out of a screen of
  text, a panel in two halves at different offsets, and a different subset on
  every frame. **It reads exactly like a font that failed to load, and it is
  not**: nothing is logged, the fonts are in the APK, and the same build
  draws them on the desktop and in the simulator. The 3D behind it is
  untouched, which is why the felt and the sky look perfect throughout and
  point away from the cause.

  Both cameras carry `Msaa::Off` under `cfg(target_os = "android")`. The
  measurement, with the clock stopped (`POST /pause`) and two screenshots
  five seconds apart, same phone and same build:

  | | differing pixels | bright pixels |
  | --- | --- | --- |
  | `Sample4` (bevy's default) | 105 000 | 1 200 |
  | `Msaa::Off` | 4 300 | 4 700 |

  Four other things were ruled out first, each by a build and a measurement
  rather than by argument, and they are listed so that nobody re-runs them:
  the fractional scale factor (2.4375, pinned to 2 — no change),
  `WgpuSettingsPriority::WebGPU` in place of the adapter's own claims (no
  change), GPU preprocessing (`bevy_ui_render` does not use it) and push
  constants (it does not use those either — it writes a plain
  `RawBufferVec` every frame).
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

Two things do **not** work yet:

- **The lobby is readable on Android but not yet usable by hand.** The panel
  backgrounds do not draw and the two input fields come out smeared, so a
  person cannot sign in on the phone even though every label is legible.
  This is what is *left* after the multisampling fix below, and it is a much
  smaller thing than what it started as.
- **The Android emulator loses the device.** With `-gpu host` the guest gets
  the host GPU through gfxstream (`AdapterInfo … "Apple M1 Max", driver:
  "MoltenVK"`), and one frame after `Creating new window baylee` it reports
  `DeviceLost … (driver implementation is at fault)`, then panics in
  `wgpu-hal … swapchain: Trying to destroy a SwapchainAcquireSemaphore that is
  still in use by a SurfaceTexture`. `-gpu swiftshader_indirect` did not help —
  the emulator logged `option: host` anyway and then wedged at 0 % CPU without
  ever opening its adb port. The physical Pixel was the thing to try instead,
  and it worked — so this is a dead end that costs nothing, not a blocker.

The `WinitSettings::mobile()` above is not a candidate fix for it: that was a
run loop nobody woke, and this is a device the driver hands back. The obvious
next lever is not one either, and that is worth writing down before someone
spends an hour on it — `-feature -Vulkan` would push the guest onto GLES, which
wgpu does support on Android, except that the SDK's own
`emulator/lib/advancedFeatures.ini` already says `Vulkan = off` and there is no
override in `~/.android/`, and the guest reported a Vulkan adapter anyway. So
whatever turns it on comes from the system image or from gfxstream itself, and
the flag is not the switch it looks like.
