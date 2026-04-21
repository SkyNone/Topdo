#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

PRODUCT="Topdo"
VERSION="1.0.0"
APP_NAME="$PRODUCT.app"

SIGNING_IDENTITY="Developer ID Application: Ronghao cui (6Z4232N2L9)"

check_notarization_env() {
  if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
    return 0
  else
    return 1
  fi
}

echo "========================================="
echo "  $PRODUCT v$VERSION 构建脚本"
echo "========================================="
echo ""

if check_notarization_env; then
  echo "[信息] 检测到公证环境变量，构建后将自动公证"
else
  echo "[警告] 未检测到公证环境变量 (APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID)"
  echo "        应用将仅签名，不进行公证。分发给其他用户时可能会被 Gatekeeper 拦截。"
  echo "        设置方法: export APPLE_ID=xxx APPLE_PASSWORD=xxx APPLE_TEAM_ID=6Z4232N2L9"
fi
echo ""

echo "[1/3] 确保 Rust 双目标已安装..."
rustup target add x86_64-apple-darwin >/dev/null

echo "[2/3] 构建 Universal app + dmg（含签名）..."
pnpm tauri build --target universal-apple-darwin --bundles app,dmg

BUNDLE_BASE="$ROOT_DIR/src-tauri/target/universal-apple-darwin/release/bundle"
DMG_DIR="$BUNDLE_BASE/dmg"
MACOS_DIR="$BUNDLE_BASE/macos"
APP_PATH="$MACOS_DIR/$APP_NAME"
OUTPUT_DMG="$DMG_DIR/${PRODUCT}_${VERSION}_universal.dmg"

if [ ! -d "$APP_PATH" ]; then
  echo "[错误] 未找到 app: $APP_PATH"
  exit 1
fi
if [ ! -f "$OUTPUT_DMG" ]; then
  echo "[错误] 未找到 dmg: $OUTPUT_DMG"
  exit 1
fi

echo "签名 DMG..."
codesign --sign "$SIGNING_IDENTITY" --force "$OUTPUT_DMG"

echo ""
echo "[3/3] 验证签名..."
codesign --verify --deep --strict --verbose=2 "$APP_PATH" 2>&1 || true
codesign --verify --verbose=2 "$OUTPUT_DMG" 2>&1 || true
echo ""

if check_notarization_env; then
  echo "提交 DMG 公证..."
  NOTARY_SUBMIT_OUTPUT=$(xcrun notarytool submit "$OUTPUT_DMG" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait 2>&1) || true
  echo "$NOTARY_SUBMIT_OUTPUT"

  if echo "$NOTARY_SUBMIT_OUTPUT" | grep -q "status: Accepted"; then
    echo "公证成功，装订票据到 DMG..."
    xcrun stapler staple "$OUTPUT_DMG"
    echo ""
    echo "[完成] 已签名并公证的发布包已生成"
  else
    echo "[警告] 公证未通过，请检查输出日志"
    echo "  可运行 xcrun notarytool log <submission-id> --apple-id \$APPLE_ID --password \$APPLE_PASSWORD --team-id \$APPLE_TEAM_ID 查看详情"
  fi
else
  echo "[完成] 已签名的发布包已生成（未公证）"
fi

echo ""
echo "Universal app: $APP_PATH"
echo "Universal dmg: $OUTPUT_DMG"
