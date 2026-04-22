#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

PRODUCT="Topdo"
VERSION="1.0.1"
APP_NAME="$PRODUCT.app"
NOTICE_FILE="$ROOT_DIR/docs/01_安装说明.txt"

echo "[1/3] 确保 Rust 双目标已安装..."
rustup target add x86_64-apple-darwin >/dev/null

echo "[2/3] 构建 Universal app..."
pnpm tauri build --target universal-apple-darwin --bundles app

BUNDLE_BASE="$ROOT_DIR/src-tauri/target/universal-apple-darwin/release/bundle"
DMG_DIR="$BUNDLE_BASE/dmg"
MACOS_DIR="$BUNDLE_BASE/macos"
BUNDLE_DMG_SH="$DMG_DIR/bundle_dmg.sh"
VOL_ICON="$DMG_DIR/icon.icns"
APP_PATH="$MACOS_DIR/$APP_NAME"
OUTPUT_DMG="$DMG_DIR/${PRODUCT}_${VERSION}_universal.dmg"
TEMP_DMG="$DMG_DIR/${PRODUCT}_${VERSION}_universal_with_notice.dmg"
SRC_FOLDER="$MACOS_DIR"

if [ ! -d "$APP_PATH" ]; then
  echo "[错误] 未找到 app: $APP_PATH"
  exit 1
fi
if [ ! -f "$BUNDLE_DMG_SH" ]; then
  echo "[错误] 未找到 bundle_dmg.sh: $BUNDLE_DMG_SH"
  exit 1
fi
if [ ! -f "$NOTICE_FILE" ]; then
  echo "[错误] 未找到安装说明: $NOTICE_FILE"
  exit 1
fi

echo "[3/3] 重新打包 DMG（注入安装说明）..."
rm -f "$TEMP_DMG"
rm -f "$OUTPUT_DMG"
"$BUNDLE_DMG_SH" \
  --volname "$PRODUCT" \
  --icon "$APP_NAME" 180 220 \
  --app-drop-link 480 220 \
  --window-size 660 400 \
  --hide-extension "$APP_NAME" \
  --add-file "01_安装说明.txt" "$NOTICE_FILE" 330 320 \
  --volicon "$VOL_ICON" \
  "$TEMP_DMG" \
  "$SRC_FOLDER"
mv -f "$TEMP_DMG" "$OUTPUT_DMG"

echo "[完成] 发布包已生成"
echo "Universal app: $APP_PATH"
echo "Universal dmg: $OUTPUT_DMG"
