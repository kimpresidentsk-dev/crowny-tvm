#!/bin/bash
# ═══════════════════════════════════════════════════
# CROWNIN TVM v0.1.0 — macOS 설치 스크립트
# 맥스튜디오/맥북에서 원클릭 빌드+실행
# ═══════════════════════════════════════════════════

set -e

echo ""
echo "╔═══════════════════════════════════════════════╗"
echo "║  CROWNIN TVM v0.1.0 — 설치 시작               ║"
echo "║  균형3진법 가상머신 | 한선어 v1.0              ║"
echo "╚═══════════════════════════════════════════════╝"
echo ""

# ── 1. Rust 설치 확인 ──
if ! command -v cargo &> /dev/null; then
    echo "⚙️  Rust가 설치되어 있지 않습니다. 설치합니다..."
    echo "   (rustup 공식 설치기 사용)"
    echo ""
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo ""
    echo "✅ Rust 설치 완료: $(rustc --version)"
else
    echo "✅ Rust 확인: $(rustc --version)"
fi

# cargo 경로 확인
export PATH="$HOME/.cargo/bin:$PATH"

# ── 2. 프로젝트 디렉토리 확인 ──
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

if [ ! -f "Cargo.toml" ]; then
    echo "❌ Cargo.toml을 찾을 수 없습니다."
    echo "   이 스크립트는 crowni-tvm 폴더 안에서 실행하세요."
    exit 1
fi

echo "📁 프로젝트: $SCRIPT_DIR"

# ── 3. 빌드 ──
echo ""
echo "🔨 빌드 중 (release 모드)..."
cargo build --release 2>&1 | grep -E "(Compiling|Finished|error)" || true

if [ ! -f "target/release/crowni-tvm" ]; then
    echo "❌ 빌드 실패. 위 오류를 확인하세요."
    exit 1
fi

echo "✅ 빌드 완료: target/release/crowni-tvm"

# ── 4. 바이너리 크기 확인 ──
SIZE=$(ls -lh target/release/crowni-tvm | awk '{print $5}')
echo "📦 바이너리 크기: $SIZE"

# ── 5. 테스트 ──
echo ""
echo "🧪 테스트 실행..."
cargo test --release 2>&1 | grep -E "(test |running|ok|FAILED)" || true

# ── 6. 데모 실행 ──
echo ""
echo "🚀 데모 실행..."
echo "────────────────────────────────────────────"
./target/release/crowni-tvm demo
echo "────────────────────────────────────────────"

# ── 7. 사용법 안내 ──
echo ""
echo "═══════════════════════════════════════════════"
echo "  설치 완료! 사용법:"
echo ""
echo "  # REPL (대화형) 모드"
echo "  ./target/release/crowni-tvm"
echo ""
echo "  # 한선어 프로그램 실행"
echo "  ./target/release/crowni-tvm run examples/피타고라스.hsn"
echo ""
echo "  # 명령어 목록"
echo "  ./target/release/crowni-tvm info"
echo ""
echo "  # 10진→균형3진 변환"
echo "  ./target/release/crowni-tvm trit 42"
echo ""
echo "  # 편하게 쓰려면 PATH에 추가:"
echo "  cp target/release/crowni-tvm /usr/local/bin/"
echo "═══════════════════════════════════════════════"
