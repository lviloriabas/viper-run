#!/bin/sh
# Builds the release binaries for Linux and Windows into dist/.
#
# Needs Rust with the Windows target (rustup target add x86_64-pc-windows-gnu)
# and one of:
#   - Zig + cargo-zigbuild (recommended: one small tool links both targets), or
#   - a native gcc toolchain plus mingw-w64 (x86_64-w64-mingw32-gcc).
set -e
cd "$(dirname "$0")"

if command -v cargo-zigbuild >/dev/null 2>&1; then
    # glibc 2.17 keeps the Linux binary runnable on old distributions.
    cargo zigbuild --release --bins --target x86_64-unknown-linux-gnu.2.17
    cargo zigbuild --release --bins --target x86_64-pc-windows-gnu
else
    cargo build --release --bins --target x86_64-unknown-linux-gnu
    cargo build --release --bins --target x86_64-pc-windows-gnu
fi

mkdir -p dist
cp target/x86_64-unknown-linux-gnu/release/viper-run dist/viper-run-linux-x86_64
cp target/x86_64-pc-windows-gnu/release/viper-run.exe dist/viper-run-windows-x86_64.exe
cp target/x86_64-unknown-linux-gnu/release/viper-server dist/viper-server-linux-x86_64
cp target/x86_64-pc-windows-gnu/release/viper-server.exe dist/viper-server-windows-x86_64.exe
cp docs/MULTIPLAYER-ES.md dist/INSTRUCCIONES-MULTIJUGADOR.md
cp docs/MULTIPLAYER.md dist/MULTIPLAYER.md
cp docs/MULTIPLAYER-MISSIONS.md docs/VOICE-CHAT.md dist/
mkdir -p dist/model-licenses
cp assets/models/licenses/*.txt dist/model-licenses/
cp assets/models/NOTICE.txt dist/MODEL-LICENSES.txt
(cd dist && sha256sum viper-run-linux-x86_64 viper-run-windows-x86_64.exe viper-server-linux-x86_64 viper-server-windows-x86_64.exe > SHA256SUMS.txt)
ls -l dist
