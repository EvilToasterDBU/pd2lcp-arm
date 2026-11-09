#!/bin/sh

set -eu

cargo +nightly build -Zbuild-std --release -p pd2lcp-iced

(cd crates/pd2lcp-iced && cargo +nightly appimage -Zbuild-std)

mkdir -p releases
cp target/appimage/pd2lcp-iced.AppImage releases/pd2lcp-iced-aarch64.AppImage
chmod +x releases/pd2lcp-iced-aarch64.AppImage
