#!/usr/bin/env bash
# The environment an Android build of the client needs, in one place.
#
# Source it; do not run it. Nothing here belongs in a shell profile — an
# `ANDROID_NDK_ROOT` left set for every terminal is how a desktop build picks
# up a cross compiler by accident.
#
#     source scripts/mobile/android-env.sh
#
# `/usr/libexec/java_home` does not find the Homebrew JDK (it is not linked
# into /Library/Java/JavaVirtualMachines), and `apksigner` is a Java program,
# so JAVA_HOME has to be named rather than discovered.
export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$ANDROID_HOME/ndk/28.0.12674087}"

# The API level everything agrees on. 33 is what cargo-apk defaults to and
# what the Play Store asks for; `bevy_audio` needs 26 or newer, so this is
# above the floor that matters here.
export BAYLEE_ANDROID_API="${BAYLEE_ANDROID_API:-33}"

_ndk_bin="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/darwin-x86_64/bin"

# `cc` for the C and C++ build scripts under this target — oboe-sys (the
# Android audio backend behind bevy_audio → rodio → cpal) is the one that
# actually needs it, and it builds C++ through cmake.
export CC_aarch64_linux_android="$_ndk_bin/aarch64-linux-android${BAYLEE_ANDROID_API}-clang"
export CXX_aarch64_linux_android="$_ndk_bin/aarch64-linux-android${BAYLEE_ANDROID_API}-clang++"
export AR_aarch64_linux_android="$_ndk_bin/llvm-ar"
export RANLIB_aarch64_linux_android="$_ndk_bin/llvm-ranlib"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"

export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmake/3.31.1/bin:$JAVA_HOME/bin:$PATH"
unset _ndk_bin
