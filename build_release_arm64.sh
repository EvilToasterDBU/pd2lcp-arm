#!/bin/sh

set -eu

BOX64_VERSION=v0.4.4
BOX64_SOURCE="$PWD/target/box64-source"
BOX64_BUILD="$PWD/target/box64-build"
BOX64_ASSET="$PWD/target/box64"
BOX64_LIBS_ARCHIVE="$PWD/target/box64-x86-libs.tar.gz"

rm -rf "$BOX64_SOURCE" "$BOX64_BUILD" "$BOX64_ASSET"
rm -f "$BOX64_LIBS_ARCHIVE"
git clone --depth 1 --branch "$BOX64_VERSION" https://github.com/ptitSeb/box64.git "$BOX64_SOURCE"
cmake -S "$BOX64_SOURCE" -B "$BOX64_BUILD" \
  -D ARM_DYNAREC=ON \
  -D CMAKE_BUILD_TYPE=Release
cmake --build "$BOX64_BUILD" --parallel "$(nproc)"
mkdir -p "$BOX64_ASSET"
install -m 0755 "$BOX64_BUILD/box64" "$BOX64_ASSET/box64"
curl -L --fail --retry 3 \
  -o "$BOX64_LIBS_ARCHIVE" \
  https://github.com/ptitSeb/box64/releases/download/v0.4.4/box64-bundle-x86-libs-v0.4.4.tar.gz
echo '1d4a8787a8a92267c1be673ec194c69e3bb33d14c800575d94131084142fcff0  target/box64-x86-libs.tar.gz' | sha256sum -c -
install -m 0644 "$BOX64_LIBS_ARCHIVE" "$BOX64_ASSET/box64-x86-libs.tar.gz"

cargo +nightly build -Zbuild-std --release -p pd2lcp-iced

(cd crates/pd2lcp-iced && cargo +nightly appimage -Zbuild-std)

mkdir -p releases
cp target/appimage/pd2lcp-iced.AppImage releases/pd2lcp-iced-aarch64.AppImage
chmod +x releases/pd2lcp-iced-aarch64.AppImage
