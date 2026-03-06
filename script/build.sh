#!/usr/bin/env bash
set -euo pipefail

# gh-extension-precompile から呼ばれるクロスコンパイルスクリプト
# 出力ファイル名: gh-lint_{tag}_{os}-{arch}[.exe]

TAG="${1:-dev}"
EXT_NAME="gh-lint"

# Zig のインストール
ZIG_VERSION="0.14.0"
ZIG_DIR="zig-${ZIG_VERSION}"

if ! command -v zig &>/dev/null || [[ "$(zig version)" != "${ZIG_VERSION}" ]]; then
    echo "Installing Zig ${ZIG_VERSION}..."
    curl -fsSL "https://ziglang.org/download/${ZIG_VERSION}/zig-linux-x86_64-${ZIG_VERSION}.tar.xz" \
        | tar xJ
    mv "zig-linux-x86_64-${ZIG_VERSION}" "${ZIG_DIR}"
    export PATH="${PWD}/${ZIG_DIR}:${PATH}"
fi

mkdir -p dist

declare -A TARGETS=(
    ["aarch64-macos"]="darwin arm64"
    ["x86_64-macos"]="darwin amd64"
    ["x86_64-linux-musl"]="linux amd64"
    ["aarch64-linux-musl"]="linux arm64"
    ["x86_64-windows"]="windows amd64"
)

for ZIG_TARGET in "${!TARGETS[@]}"; do
    read -r OS ARCH <<< "${TARGETS[$ZIG_TARGET]}"
    EXT=""
    [[ "$OS" == "windows" ]] && EXT=".exe"

    echo "Building for ${ZIG_TARGET}..."
    zig build -Dtarget="${ZIG_TARGET}" -Doptimize=ReleaseSafe

    OUTPUT_NAME="${EXT_NAME}_${TAG}_${OS}-${ARCH}${EXT}"
    cp "zig-out/bin/${EXT_NAME}${EXT}" "dist/${OUTPUT_NAME}"
    echo "  -> dist/${OUTPUT_NAME}"
done

echo "Build complete."
