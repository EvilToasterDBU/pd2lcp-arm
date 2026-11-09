#!/bin/sh

set -eu

BOX64_VERSION=v0.4.4
BOX64_SOURCE="$PWD/target/box64-source"
BOX64_BUILD="$PWD/target/box64-build"
BOX64_ASSET="$PWD/target/box64"

rm -rf "$BOX64_SOURCE" "$BOX64_BUILD" "$BOX64_ASSET"
git clone --depth 1 --branch "$BOX64_VERSION" https://github.com/ptitSeb/box64.git "$BOX64_SOURCE"
cmake -S "$BOX64_SOURCE" -B "$BOX64_BUILD" \
  -D ARM_DYNAREC=ON \
  -D CMAKE_BUILD_TYPE=Release
cmake --build "$BOX64_BUILD" --parallel "$(nproc)"
mkdir -p "$BOX64_ASSET"
install -m 0755 "$BOX64_BUILD/box64" "$BOX64_ASSET/box64"

cargo +nightly build -Zbuild-std --release -p pd2lcp-iced

(cd crates/pd2lcp-iced && cargo +nightly appimage -Zbuild-std)

mkdir -p releases
cp target/appimage/pd2lcp-iced.AppImage releases/pd2lcp-iced-aarch64.AppImage
chmod +x releases/pd2lcp-iced-aarch64.AppImage
