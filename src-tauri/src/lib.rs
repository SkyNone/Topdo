use aes_gcm::{
  aead::{Aead, KeyInit},
  Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::{
  fs,
  path::{Path, PathBuf},
  str::FromStr,
  sync::Mutex,
};
use tauri::{
  Emitter,
  menu::{Menu, MenuItem},
  tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
  AppHandle, LogicalSize, Manager, Size, State, WebviewWindow,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

const MAIN_WINDOW_LABEL: &str = "main";
const NORMAL_WIDTH: f64 = 320.0;
const NORMAL_HEIGHT: f64 = 500.0;
const MINI_PET_WIDTH: f64 = 80.0;
const MINI_PET_HEIGHT: f64 = 80.0;
const MINI_PILL_WIDTH: f64 = 176.0;
const MINI_PILL_HEIGHT: f64 = 44.0;
const CONFIG_FILE_NAME: &str = "config.json";
const DB_FILE_NAME: &str = "tasks.db";
const TASK_ORDER_FILE_NAME: &str = "task_order.json";
const WINDOW_SIZE_FILE_NAME: &str = "window_size.json";
const TOKEN_SALT: &str = "topdo-salt-2026";
const DEFAULT_TOGGLE_SHORTCUT: &str = "Cmd+Shift+T";
const DEFAULT_TOGGLE_MODE_SHORTCUT: &str = "Alt+T";

#[cfg(target_os = "macos")]
const KCG_NORMAL_WINDOW_LEVEL_KEY: i32 = 4;
#[cfg(target_os = "macos")]
const KCG_STATUS_WINDOW_LEVEL_KEY: i32 = 9;
#[cfg(target_os = "macos")]
const NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_APPLICATIONS: u64 = 1 << 18;

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
  fn CGWindowLevelForKey(key: i32) -> i32;
}

const CREATE_TASKS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS tasks (
    record_id TEXT PRIMARY KEY,
    id TEXT,
    name TEXT NOT NULL,
    status TEXT DEFAULT '待处理',
    priority TEXT DEFAULT '普通',
    task_type TEXT DEFAULT '日常事务',
    time_spent TEXT DEFAULT '',
    created_at TEXT DEFAULT '',
    updated_at TEXT DEFAULT '',
    completed_at TEXT DEFAULT '',
    notes TEXT DEFAULT '',
    sort_order INTEGER DEFAULT 0,
    source TEXT DEFAULT 'local',
    feishu_record_id TEXT DEFAULT '',
    sync_status TEXT DEFAULT 'synced',
    last_synced_at TEXT DEFAULT '',
    retry_count INTEGER DEFAULT 0,
    last_error TEXT DEFAULT '',
    last_retry_at TEXT DEFAULT ''
);
"#;

#[derive(Debug, Serialize, Deserialize)]
struct SyncStatus {
  service: String,
  healthy: bool,
}

#[derive(Debug, Default)]
struct UiState {
  mini_mode: bool,
  always_on_top: bool,
}

#[derive(Debug, Serialize)]
struct WindowStatePayload {
  mini_mode: bool,
  always_on_top: bool,
}

#[derive(Debug, Serialize, Clone)]
struct WindowModeChangedPayload {
  mode: String,
  mini_mode: bool,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct FeishuConfig {
  #[serde(default)]
  app_id: String,
  #[serde(default)]
  encrypted_app_secret: String,
  #[serde(default)]
  app_token: String,
  #[serde(default)]
  table_id: String,
  #[serde(default)]
  folder_token: String,
  #[serde(default)]
  collaborator_email: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct ShortcutConfig {
  #[serde(default)]
  toggle_window: String,
  #[serde(default)]
  toggle_mode: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PetPositionConfig {
  x: f64,
  y: f64,
}

impl Default for PetPositionConfig {
  fn default() -> Self {
    Self { x: 0.0, y: 0.0 }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PetConfig {
  #[serde(default = "default_pet_enabled")]
  enabled: bool,
  #[serde(default = "default_pet_show_badge")]
  show_badge: bool,
  #[serde(default = "default_pet_animations")]
  animations: bool,
  #[serde(default)]
  cat_position: PetPositionConfig,
  #[serde(default = "default_pet_window_mode")]
  window_mode: String,
  #[serde(default)]
  daily_progress_date: String,
  #[serde(default = "default_daily_progress_level")]
  daily_progress_level: i32,
}

impl Default for PetConfig {
  fn default() -> Self {
    Self {
      enabled: default_pet_enabled(),
      show_badge: default_pet_show_badge(),
      animations: default_pet_animations(),
      cat_position: PetPositionConfig::default(),
      window_mode: default_pet_window_mode(),
      daily_progress_date: String::new(),
      daily_progress_level: default_daily_progress_level(),
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct AppConfig {
  #[serde(default)]
  mode: String,
  #[serde(default)]
  feishu: FeishuConfig,
  #[serde(default)]
  shortcut: ShortcutConfig,
  #[serde(default)]
  pet: PetConfig,
  #[serde(default = "default_sync_interval")]
  sync_interval: i64,
  #[serde(default)]
  created_at: String,

  // legacy fields for backward compatible read only
  #[serde(default, skip_serializing)]
  app_mode: String,
  #[serde(default, skip_serializing)]
  app_id: String,
  #[serde(default)]
  #[serde(skip_serializing)]
  app_token: String,
  #[serde(default)]
  #[serde(skip_serializing)]
  table_id: String,
  #[serde(default = "default_sync_interval")]
  #[serde(skip_serializing)]
  sync_interval_seconds: i64,
}

#[derive(Debug, Serialize)]
struct ConfigPayload {
  mode: String,
  app_id: String,
  app_token: String,
  table_id: String,
  folder_token: String,
  collaborator_email: String,
  has_secret: bool,
}

#[derive(Debug, Default)]
struct ConfigIoLock;

#[derive(Debug, Default)]
struct GlobalShortcutState {
  toggle_window: Option<tauri_plugin_global_shortcut::Shortcut>,
  toggle_window_text: String,
  toggle_mode: Option<tauri_plugin_global_shortcut::Shortcut>,
  toggle_mode_text: String,
}

#[derive(Debug, Default)]
struct TokenManager {
  app_id: String,
  app_secret: String,
  cached_token: Option<String>,
  expires_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct LegacyAppConfig {
  #[serde(default)]
  app_mode: String,
  #[serde(default)]
  app_token: String,
  #[serde(default)]
  table_id: String,
  #[serde(default = "default_sync_interval")]
  sync_interval_seconds: i64,
}

#[derive(Debug, Serialize)]
struct ConnectionTestResult {
  success: bool,
  message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
  id: String,
  record_id: String,
  name: String,
  status: String,
  priority: String,
  task_type: String,
  time_spent: String,
  created_at: String,
  updated_at: String,
  completed_at: String,
  notes: String,
  sort_order: i64,
  source: String,
  feishu_record_id: String,
  sync_status: String,
  last_synced_at: String,
  retry_count: i64,
  last_error: String,
  last_retry_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SyncMeta {
  pending_count: i64,
  failed_count: i64,
  last_sync_at: String,
  last_error_summary: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SyncTasksResult {
  tasks: Vec<Task>,
  sync_meta: SyncMeta,
}

#[derive(Debug, Serialize)]
struct UpdateTaskResult {
  success: bool,
  message: String,
}

#[derive(Debug, Serialize)]
struct CreateTaskResult {
  record_id: String,
  synced: bool,
}

#[derive(Debug, Serialize)]
struct ShortcutConfigPayload {
  toggle_window: String,
}

#[derive(Debug, Serialize)]
struct ModeShortcutConfigPayload {
  toggle_mode: String,
}

#[derive(Debug, Serialize)]
struct SetShortcutConfigResult {
  success: bool,
  message: String,
  applied: Option<String>,
}

#[derive(Debug, Serialize)]
struct PetPositionPayload {
  x: f64,
  y: f64,
}

#[derive(Debug, Serialize)]
struct PetSettingsPayload {
  enabled: bool,
  show_badge: bool,
  animations: bool,
  cat_position: PetPositionPayload,
  window_mode: String,
  daily_progress_date: String,
  daily_progress_level: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct WindowSizePayload {
  width: f64,
  height: f64,
}

#[derive(Debug, Deserialize)]
struct FeishuTablesResponse {
  code: i32,
  msg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FeishuTenantAccessTokenResponse {
  code: i32,
  msg: Option<String>,
  tenant_access_token: Option<String>,
  expire: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct FeishuRecord {
  record_id: String,
  fields: Value,
}

#[derive(Debug, Deserialize)]
struct FeishuRecordsData {
  items: Option<Vec<FeishuRecord>>,
  records: Option<Vec<FeishuRecord>>,
}

#[derive(Debug, Deserialize)]
struct FeishuRecordsResponse {
  code: i32,
  msg: Option<String>,
  data: Option<FeishuRecordsData>,
}

#[derive(Debug, Deserialize)]
struct FeishuGenericResponse {
  code: i32,
  msg: Option<String>,
  data: Option<Value>,
}

fn now_unix_seconds() -> String {
  let now = std::time::SystemTime::now();
  let unix = now
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_secs();
  unix.to_string()
}

fn now_unix_millis() -> u128 {
  let now = std::time::SystemTime::now();
  now
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis()
}

fn now_iso() -> String {
  now_unix_seconds()
}

fn default_sync_interval() -> i64 {
  30
}

fn default_pet_enabled() -> bool {
  true
}

fn default_pet_show_badge() -> bool {
  true
}

fn default_pet_animations() -> bool {
  true
}

fn default_pet_window_mode() -> String {
  "panel".to_string()
}

fn default_daily_progress_level() -> i32 {
  1
}

fn normalize_app_mode(mode: &str) -> String {
  if mode.trim() == "feishu" {
    "feishu".to_string()
  } else if mode.trim() == "local" {
    "local".to_string()
  } else {
    String::new()
  }
}

fn default_sync_interval_seconds(value: i64) -> i64 {
  if matches!(value, 0 | 15 | 30 | 60) {
    value
  } else {
    30
  }
}

fn normalize_window_mode(value: &str) -> String {
  if value.trim() == "cat" {
    "cat".to_string()
  } else {
    "panel".to_string()
  }
}

fn normalize_daily_progress_level(value: i32) -> i32 {
  value.clamp(1, 4)
}

fn normalize_task(mut task: Task) -> Task {
  if task.id.trim().is_empty() {
    task.id = task.record_id.clone();
  }
  if task.record_id.trim().is_empty() {
    task.record_id = task.id.clone();
  }
  if task.name.trim().is_empty() {
    task.name = "未命名任务".to_string();
  }
  if task.status.trim().is_empty() {
    task.status = "待处理".to_string();
  }
  if task.priority.trim().is_empty() {
    task.priority = "普通".to_string();
  }
  if task.task_type.trim().is_empty() {
    task.task_type = "日常事务".to_string();
  }
  if task.sync_status.trim().is_empty() {
    task.sync_status = "synced".to_string();
  }
  if task.source.trim().is_empty() {
    task.source = "local".to_string();
  }
  if task.last_error.trim().is_empty() {
    task.last_error = String::new();
  }
  if task.last_retry_at.trim().is_empty() {
    task.last_retry_at = String::new();
  }
  task
}

fn to_feishu_priority_value(priority: &str) -> String {
  match priority.trim() {
    "紧急" | "今日必做" | "🔴今日必做" | "🔴 今日必做" => "今日必做".to_string(),
    "重要" | "本周完成" | "🟡本周完成" | "🟠本周完成" | "🔵本周完成" | "🟡尽快完成" | "🟡重要不紧急" => {
      "本周完成".to_string()
    }
    "普通" | "自由安排" | "⚪️自由安排" | "⚪自由安排" | "🔵有空再说" | "🔵常规任务" => {
      "自由安排".to_string()
    }
    "" => "自由安排".to_string(),
    other => other.to_string(),
  }
}

fn task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
  Ok(Task {
    record_id: row.get(0)?,
    id: row.get(1)?,
    name: row.get(2)?,
    status: row.get(3)?,
    priority: row.get(4)?,
    task_type: row.get(5)?,
    time_spent: row.get(6)?,
    created_at: row.get(7)?,
    updated_at: row.get(8)?,
    completed_at: row.get(9)?,
    notes: row.get(10)?,
    sort_order: row.get(11)?,
    source: row.get(12)?,
    feishu_record_id: row.get(13)?,
    sync_status: row.get(14)?,
    last_synced_at: row.get(15)?,
    retry_count: row.get(16)?,
    last_error: row.get(17)?,
    last_retry_at: row.get(18)?,
  })
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<(), String> {
  let mut stmt = conn
    .prepare(&format!("PRAGMA table_info({table})"))
    .map_err(|err| format!("pragma table_info failed: {err}"))?;
  let mut rows = stmt
    .query([])
    .map_err(|err| format!("pragma table_info query failed: {err}"))?;

  while let Some(row) = rows.next().map_err(|err| format!("pragma row failed: {err}"))? {
    let name: String = row.get(1).map_err(|err| format!("pragma get name failed: {err}"))?;
    if name == column {
      return Ok(());
    }
  }

  conn
    .execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"), [])
    .map_err(|err| format!("add column {column} failed: {err}"))?;
  Ok(())
}

fn open_db(path: &PathBuf) -> Result<Connection, String> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|err| format!("create db dir failed: {err}"))?;
  }

  let conn = Connection::open(path).map_err(|err| format!("open db failed: {err}"))?;
  conn
    .execute_batch(CREATE_TASKS_TABLE_SQL)
    .map_err(|err| format!("init db failed: {err}"))?;
  ensure_column(&conn, "tasks", "id", "TEXT DEFAULT ''")?;
  ensure_column(&conn, "tasks", "completed_at", "TEXT DEFAULT ''")?;
  ensure_column(&conn, "tasks", "sort_order", "INTEGER DEFAULT 0")?;
  ensure_column(&conn, "tasks", "source", "TEXT DEFAULT 'local'")?;
  ensure_column(&conn, "tasks", "feishu_record_id", "TEXT DEFAULT ''")?;
  ensure_column(&conn, "tasks", "retry_count", "INTEGER DEFAULT 0")?;
  ensure_column(&conn, "tasks", "last_error", "TEXT DEFAULT ''")?;
  ensure_column(&conn, "tasks", "last_retry_at", "TEXT DEFAULT ''")?;

  conn
    .execute(
      "UPDATE tasks SET id = record_id WHERE (id IS NULL OR id = '') AND record_id IS NOT NULL",
      [],
    )
    .map_err(|err| format!("backfill id failed: {err}"))?;
  Ok(conn)
}

fn upsert_task(conn: &Connection, task: &Task) -> Result<(), String> {
  let task = normalize_task(task.clone());
  conn
    .execute(
      "INSERT INTO tasks (
        record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
      ON CONFLICT(record_id) DO UPDATE SET
        id=excluded.id,
        name=excluded.name,
        status=excluded.status,
        priority=excluded.priority,
        task_type=excluded.task_type,
        time_spent=excluded.time_spent,
        created_at=excluded.created_at,
        updated_at=excluded.updated_at,
        completed_at=CASE
          WHEN excluded.status NOT LIKE '%已完成%' THEN excluded.completed_at
          WHEN excluded.completed_at != '' THEN excluded.completed_at
          ELSE COALESCE(tasks.completed_at, '')
        END,
        notes=excluded.notes,
        sort_order=excluded.sort_order,
        source=excluded.source,
        feishu_record_id=excluded.feishu_record_id,
        sync_status=excluded.sync_status,
        last_synced_at=excluded.last_synced_at,
        retry_count=excluded.retry_count,
        last_error=excluded.last_error,
        last_retry_at=excluded.last_retry_at",
      params![
        task.record_id,
        task.id,
        task.name,
        task.status,
        task.priority,
        task.task_type,
        task.time_spent,
        task.created_at,
        task.updated_at,
        task.completed_at,
        task.notes,
        task.sort_order,
        task.source,
        task.feishu_record_id,
        task.sync_status,
        task.last_synced_at,
        task.retry_count,
        task.last_error,
        task.last_retry_at
      ],
    )
    .map_err(|err| format!("upsert task failed: {err}"))?;
  Ok(())
}

fn get_main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
  app
    .get_webview_window(MAIN_WINDOW_LABEL)
    .ok_or_else(|| "main window not found".to_string())
}

fn apply_normal_mode(window: &WebviewWindow) -> tauri::Result<()> {
  window.set_resizable(true)?;
  window.set_min_size(Option::<Size>::None)?;
  window.set_max_size(Option::<Size>::None)?;
  window.set_size(Size::Logical(LogicalSize::new(NORMAL_WIDTH, NORMAL_HEIGHT)))?;
  Ok(())
}

fn apply_mini_mode(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
  let pet_enabled = load_app_config_from_file(app)
    .map(|cfg| cfg.pet.enabled)
    .unwrap_or(true);
  let mini_size = if pet_enabled {
    Size::Logical(LogicalSize::new(MINI_PET_WIDTH, MINI_PET_HEIGHT))
  } else {
    Size::Logical(LogicalSize::new(MINI_PILL_WIDTH, MINI_PILL_HEIGHT))
  };
  window.set_resizable(false)?;
  window.set_min_size(Some(mini_size))?;
  window.set_max_size(Some(mini_size))?;
  window.set_size(mini_size)?;
  Ok(())
}

#[cfg(target_os = "macos")]
fn macos_window_level_for_key(key: i32) -> i32 {
  unsafe { CGWindowLevelForKey(key) }
}

fn is_dev_runtime() -> bool {
  cfg!(debug_assertions)
}

fn should_include_native_traits(reason: &str) -> bool {
  !(is_dev_runtime() && reason == "setup")
}

fn apply_window_traits_safe(window: &WebviewWindow, pinned: bool, reason: &str) -> Result<(), String> {
  eprintln!("[Rust] apply_window_traits_safe start: reason={reason}, pinned={pinned}");
  window
    .set_always_on_top(pinned)
    .map_err(|err| format!("set always_on_top failed: {err}"))?;
  window
    .set_visible_on_all_workspaces(pinned)
    .map_err(|err| format!("set visible on all workspaces failed: {err}"))?;
  eprintln!(
    "[Rust] apply_window_traits_safe visible_on_all_workspaces set: reason={reason}, pinned={pinned}"
  );
  eprintln!("[Rust] apply_window_traits_safe done: reason={reason}, pinned={pinned}");
  Ok(())
}

#[cfg(target_os = "macos")]
fn apply_window_traits_native(
  window: &WebviewWindow,
  pinned: bool,
  reason: &str,
) -> Result<(), String> {
  use cocoa::{
    appkit::{NSColor, NSWindow, NSWindowCollectionBehavior},
    base::{id, nil, NO},
  };

  eprintln!("[Rust] apply_window_traits_native start: reason={reason}, pinned={pinned}");

  let ns_window_ptr = window
    .ns_window()
    .map_err(|err| format!("ns_window unavailable: {err}"))?;
  let ns_window: id = ns_window_ptr as id;

  unsafe {
    ns_window.setBackgroundColor_(NSColor::clearColor(nil));
    ns_window.setOpaque_(NO);
    ns_window.setHasShadow_(NO);
    let mut behavior = ns_window.collectionBehavior();
    let original_behavior = behavior;
    let tracked_mask = NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces.bits()
      | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary.bits()
      | NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_APPLICATIONS;
    let next_bits = if pinned {
      behavior.bits() | tracked_mask
    } else {
      behavior.bits() & !tracked_mask
    };
    behavior = NSWindowCollectionBehavior::from_bits_truncate(next_bits);
    if pinned {
      // no-op: behavior already updated from the raw bitmask above
    }
    ns_window.setCollectionBehavior_(behavior);
    let level = if pinned {
      macos_window_level_for_key(KCG_STATUS_WINDOW_LEVEL_KEY) as _
    } else {
      macos_window_level_for_key(KCG_NORMAL_WINDOW_LEVEL_KEY) as _
    };
    ns_window.setLevel_(level);
    eprintln!(
      "[Rust] apply_window_traits_native macos state: reason={reason}, pinned={pinned}, behavior_before=0x{:x}, behavior_after=0x{:x}, level={}, has_can_join_all_spaces={}, has_full_screen_auxiliary={}, has_can_join_all_applications={}",
      original_behavior.bits(),
      behavior.bits(),
      level,
      (behavior.bits() & NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces.bits()) != 0,
      (behavior.bits() & NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary.bits()) != 0,
      (behavior.bits() & NS_WINDOW_COLLECTION_BEHAVIOR_CAN_JOIN_ALL_APPLICATIONS) != 0
    );
  }

  eprintln!("[Rust] apply_window_traits_native done: reason={reason}, pinned={pinned}");
  Ok(())
}

#[cfg(not(target_os = "macos"))]
fn apply_window_traits_native(
  _window: &WebviewWindow,
  _pinned: bool,
  _reason: &str,
) -> Result<(), String> {
  Ok(())
}

fn apply_window_traits(
  window: &WebviewWindow,
  pinned: bool,
  include_native: bool,
  reason: &str,
) -> Result<(), String> {
  eprintln!(
    "[Rust] apply_window_traits start: reason={reason}, pinned={pinned}, include_native={include_native}"
  );
  apply_window_traits_safe(window, pinned, reason)?;
  if include_native {
    apply_window_traits_native(window, pinned, reason)?;
  } else {
    eprintln!("[Rust] apply_window_traits native deferred: reason={reason}, pinned={pinned}");
  }
  eprintln!(
    "[Rust] apply_window_traits done: reason={reason}, pinned={pinned}, include_native={include_native}"
  );
  Ok(())
}

fn toggle_window_visibility(app: &AppHandle) -> tauri::Result<()> {
  if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
    if window.is_visible().unwrap_or(false) {
      window.hide()?;
    } else {
      let (mini_mode, pinned) = if let Ok(state) = app.state::<Mutex<UiState>>().lock() {
        (state.mini_mode, state.always_on_top)
      } else {
        (false, true)
      };
      let _ = apply_window_traits(&window, pinned, false, "toggle_window_visibility:pre-show");
      window.unminimize()?;
      window.show()?;
      window.set_focus()?;
      let _ = apply_window_traits(&window, pinned, true, "toggle_window_visibility:post-show");
      let _ = app.emit(
        "window-mode-changed",
        WindowModeChangedPayload {
          mode: if mini_mode { "cat" } else { "panel" }.to_string(),
          mini_mode,
        },
      );
    }
  }
  Ok(())
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
  if let Ok(dir) = app.path().app_data_dir() {
    migrate_legacy_app_data(&dir)?;
    return Ok(dir);
  }

  let fallback = dirs::home_dir()
    .ok_or_else(|| "failed to resolve app data directory".to_string())?
    .join(".topdo");

  Ok(fallback)
}

fn legacy_app_support_dirs() -> Vec<PathBuf> {
  let mut dirs_list = Vec::new();
  if let Some(home) = dirs::home_dir() {
    let app_support = home.join("Library").join("Application Support");
    dirs_list.push(app_support.join("com.topdo.app"));
    dirs_list.push(app_support.join("com.taskfloat.app"));
  }
  dirs_list
}

fn copy_if_missing(src: &Path, dst: &Path) -> Result<(), String> {
  if !src.exists() || dst.exists() {
    return Ok(());
  }
  fs::copy(src, dst).map_err(|err| format!("failed to migrate {}: {err}", src.display()))?;
  Ok(())
}

fn migrate_legacy_app_data(primary_dir: &Path) -> Result<(), String> {
  fs::create_dir_all(primary_dir)
    .map_err(|err| format!("failed to create app data dir {}: {err}", primary_dir.display()))?;

  for legacy_dir in legacy_app_support_dirs() {
    if !legacy_dir.exists() || legacy_dir == primary_dir {
      continue;
    }

    copy_if_missing(&legacy_dir.join(CONFIG_FILE_NAME), &primary_dir.join(CONFIG_FILE_NAME))?;
    copy_if_missing(&legacy_dir.join(DB_FILE_NAME), &primary_dir.join(DB_FILE_NAME))?;
    copy_if_missing(
      &legacy_dir.join(TASK_ORDER_FILE_NAME),
      &primary_dir.join(TASK_ORDER_FILE_NAME),
    )?;

    break;
  }

  Ok(())
}

fn config_file_path(app: &AppHandle) -> Result<PathBuf, String> {
  Ok(config_dir(app)?.join(CONFIG_FILE_NAME))
}

fn db_file_path(app: &AppHandle) -> Result<PathBuf, String> {
  Ok(config_dir(app)?.join(DB_FILE_NAME))
}

fn task_order_file_path(app: &AppHandle) -> Result<PathBuf, String> {
  Ok(config_dir(app)?.join(TASK_ORDER_FILE_NAME))
}

fn window_size_file_path(app: &AppHandle) -> Result<PathBuf, String> {
  Ok(config_dir(app)?.join(WINDOW_SIZE_FILE_NAME))
}

fn normalize_loaded_config(mut cfg: AppConfig) -> AppConfig {
  if cfg.mode.trim().is_empty() {
    cfg.mode = normalize_app_mode(&cfg.app_mode);
  } else {
    cfg.mode = normalize_app_mode(&cfg.mode);
  }

  if cfg.feishu.app_token.trim().is_empty() {
    cfg.feishu.app_token = cfg.app_token.trim().to_string();
  }
  if cfg.feishu.table_id.trim().is_empty() {
    cfg.feishu.table_id = cfg.table_id.trim().to_string();
  }
  if cfg.feishu.app_id.trim().is_empty() {
    cfg.feishu.app_id = cfg.app_id.trim().to_string();
  }

  let normalized_sync = if cfg.sync_interval > 0 {
    cfg.sync_interval
  } else {
    cfg.sync_interval_seconds
  };
  cfg.sync_interval = default_sync_interval_seconds(normalized_sync);

  if cfg.created_at.trim().is_empty() {
    cfg.created_at = now_iso();
  }
  if cfg.shortcut.toggle_window.trim().is_empty() {
    cfg.shortcut.toggle_window = DEFAULT_TOGGLE_SHORTCUT.to_string();
  }
  if cfg.shortcut.toggle_mode.trim().is_empty() {
    cfg.shortcut.toggle_mode = DEFAULT_TOGGLE_MODE_SHORTCUT.to_string();
  }
  cfg.pet.window_mode = normalize_window_mode(&cfg.pet.window_mode);
  cfg.pet.daily_progress_level = normalize_daily_progress_level(cfg.pet.daily_progress_level);
  cfg
}

fn default_app_config() -> AppConfig {
  AppConfig {
    mode: String::new(),
    feishu: FeishuConfig::default(),
    shortcut: ShortcutConfig {
      toggle_window: DEFAULT_TOGGLE_SHORTCUT.to_string(),
      toggle_mode: DEFAULT_TOGGLE_MODE_SHORTCUT.to_string(),
    },
    pet: PetConfig::default(),
    sync_interval: 30,
    created_at: now_iso(),
    app_mode: String::new(),
    app_id: String::new(),
    app_token: String::new(),
    table_id: String::new(),
    sync_interval_seconds: 30,
  }
}

fn load_app_config_from_file(app: &AppHandle) -> Result<AppConfig, String> {
  let config_lock = app.state::<Mutex<ConfigIoLock>>();
  let _guard = config_lock
    .lock()
    .map_err(|_| "failed to lock config io".to_string())?;
  let path = config_file_path(app)?;
  if !path.exists() {
    return Ok(default_app_config());
  }

  let content = fs::read_to_string(&path).map_err(|err| format!("failed to read config: {err}"))?;
  match serde_json::from_str::<AppConfig>(&content) {
    Ok(cfg) => Ok(normalize_loaded_config(cfg)),
    Err(_) => {
      let legacy = serde_json::from_str::<LegacyAppConfig>(&content)
        .map_err(|err| format!("invalid config: {err}"))?;
      Ok(normalize_loaded_config(AppConfig {
        mode: normalize_app_mode(&legacy.app_mode),
        feishu: FeishuConfig {
          app_id: String::new(),
          encrypted_app_secret: String::new(),
          app_token: legacy.app_token,
          table_id: legacy.table_id,
          folder_token: String::new(),
          collaborator_email: String::new(),
        },
        shortcut: ShortcutConfig {
          toggle_window: DEFAULT_TOGGLE_SHORTCUT.to_string(),
          toggle_mode: DEFAULT_TOGGLE_MODE_SHORTCUT.to_string(),
        },
        pet: PetConfig::default(),
        sync_interval: default_sync_interval_seconds(legacy.sync_interval_seconds),
        created_at: now_iso(),
        ..default_app_config()
      }))
    }
  }
}

fn save_app_config_to_file(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
  let config_lock = app.state::<Mutex<ConfigIoLock>>();
  let _guard = config_lock
    .lock()
    .map_err(|_| "failed to lock config io".to_string())?;
  let dir = config_dir(app)?;
  fs::create_dir_all(&dir).map_err(|err| format!("failed to create config dir: {err}"))?;

  let path = config_file_path(app)?;
  let normalized = normalize_loaded_config(config.clone());
  let content = serde_json::to_string_pretty(&normalized)
    .map_err(|err| format!("failed to serialize config: {err}"))?;
  fs::write(path, content).map_err(|err| format!("failed to write config: {err}"))
}

fn derive_key() -> [u8; 32] {
  let hostname = whoami::fallible::hostname().unwrap_or_else(|_| "unknown-host".to_string());
  let machine = format!("{}:{}:{}", hostname, whoami::username(), TOKEN_SALT);
  let digest = Sha256::digest(machine.as_bytes());
  let mut key = [0u8; 32];
  key.copy_from_slice(&digest);
  key
}

fn encrypt_token(token: &str) -> Result<String, String> {
  let cipher =
    Aes256Gcm::new_from_slice(&derive_key()).map_err(|err| format!("cipher init failed: {err}"))?;

  let mut nonce_bytes = [0u8; 12];
  rand::thread_rng().fill_bytes(&mut nonce_bytes);
  let nonce = Nonce::from_slice(&nonce_bytes);
  let encrypted = cipher
    .encrypt(nonce, token.as_bytes())
    .map_err(|_| "token encryption failed".to_string())?;

  Ok(format!(
    "{}:{}",
    BASE64_STANDARD.encode(nonce_bytes),
    BASE64_STANDARD.encode(encrypted)
  ))
}

fn decrypt_token(encrypted: &str) -> Result<String, String> {
  let mut parts = encrypted.split(':');
  let nonce_b64 = parts
    .next()
    .ok_or_else(|| "invalid encrypted token format".to_string())?;
  let cipher_b64 = parts
    .next()
    .ok_or_else(|| "invalid encrypted token format".to_string())?;
  if parts.next().is_some() {
    return Err("invalid encrypted token format".to_string());
  }

  let nonce_bytes = BASE64_STANDARD
    .decode(nonce_b64)
    .map_err(|err| format!("invalid nonce: {err}"))?;
  if nonce_bytes.len() != 12 {
    return Err("invalid nonce length".to_string());
  }
  let cipher_bytes = BASE64_STANDARD
    .decode(cipher_b64)
    .map_err(|err| format!("invalid ciphertext: {err}"))?;

  let cipher =
    Aes256Gcm::new_from_slice(&derive_key()).map_err(|err| format!("cipher init failed: {err}"))?;
  let nonce = Nonce::from_slice(&nonce_bytes);
  let plain = cipher
    .decrypt(nonce, cipher_bytes.as_ref())
    .map_err(|_| "token decryption failed".to_string())?;
  String::from_utf8(plain).map_err(|err| format!("token utf8 decode failed: {err}"))
}

fn now_unix_i64() -> i64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_secs() as i64
}

fn is_token_invalid_code(code: i32) -> bool {
  matches!(code, 99991663 | 99991664 | 99991677)
}

async fn refresh_tenant_access_token_by_credentials(
  app_id: &str,
  app_secret: &str,
) -> Result<(String, i64), String> {
  let endpoint = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";
  let client = reqwest::Client::builder()
    .use_rustls_tls()
    .build()
    .map_err(|err| format!("http client init failed: {err}"))?;

  let response = client
    .post(endpoint)
    .header("Content-Type", "application/json")
    .json(&json!({
      "app_id": app_id,
      "app_secret": app_secret
    }))
    .send()
    .await
    .map_err(|err| format!("request failed: {err}"))?;

  let status = response.status();
  let body = response
    .text()
    .await
    .map_err(|err| format!("read response failed: {err}"))?;

  if !status.is_success() {
    return Err(build_feishu_http_error(status, &body));
  }

  let parsed: FeishuTenantAccessTokenResponse =
    serde_json::from_str(&body).map_err(|err| format!("invalid response: {err}"))?;

  if parsed.code != 0 {
    return Err(format!(
      "获取 tenant_access_token 失败 code={} {}",
      parsed.code,
      parsed.msg.unwrap_or_default()
    ));
  }

  let token = parsed
    .tenant_access_token
    .ok_or_else(|| "飞书未返回 tenant_access_token".to_string())?;
  let expire_seconds = parsed.expire.unwrap_or(7200);
  Ok((token, now_unix_i64() + expire_seconds))
}

async fn get_tenant_access_token(app: &AppHandle, force_refresh: bool) -> Result<String, String> {
  let cfg = load_app_config_from_file(app)?;
  if cfg.feishu.app_id.trim().is_empty() {
    return Err("App ID 未配置".to_string());
  }
  if cfg.feishu.encrypted_app_secret.trim().is_empty() {
    return Err("App Secret 未配置".to_string());
  }

  let app_secret = decrypt_token(cfg.feishu.encrypted_app_secret.trim())?;
  let app_id = cfg.feishu.app_id.trim().to_string();
  let manager_state = app.state::<tokio::sync::RwLock<TokenManager>>();

  {
    let mut manager = manager_state.write().await;
    if manager.app_id != app_id || manager.app_secret != app_secret {
      manager.app_id = app_id.clone();
      manager.app_secret = app_secret.clone();
      manager.cached_token = None;
      manager.expires_at = None;
    }

    if !force_refresh {
      if let (Some(token), Some(expires)) = (&manager.cached_token, manager.expires_at) {
        if now_unix_i64() < expires - 300 {
          return Ok(token.clone());
        }
      }
    }
  }

  let (new_token, expires_at) =
    refresh_tenant_access_token_by_credentials(&app_id, &app_secret).await?;
  {
    let mut manager = manager_state.write().await;
    manager.cached_token = Some(new_token.clone());
    manager.expires_at = Some(expires_at);
  }
  Ok(new_token)
}

fn value_to_display_string(value: &Value) -> String {
  match value {
    Value::Null => String::new(),
    Value::Bool(v) => v.to_string(),
    Value::Number(v) => v.to_string(),
    Value::String(v) => v.to_string(),
    Value::Array(values) => values
      .iter()
      .map(value_to_display_string)
      .filter(|v| !v.trim().is_empty())
      .collect::<Vec<_>>()
      .join(", "),
    Value::Object(map) => {
      if let Some(text) = map.get("text") {
        let s = value_to_display_string(text);
        if !s.is_empty() {
          return s;
        }
      }
      if let Some(name) = map.get("name") {
        let s = value_to_display_string(name);
        if !s.is_empty() {
          return s;
        }
      }
      if let Some(value) = map.get("value") {
        let s = value_to_display_string(value);
        if !s.is_empty() {
          return s;
        }
      }
      String::new()
    }
  }
}

fn field_string(fields: &Value, key: &str) -> String {
  fields
    .get(key)
    .map(value_to_display_string)
    .unwrap_or_default()
}

fn build_feishu_http_error(status: reqwest::StatusCode, body: &str) -> String {
  let parsed = serde_json::from_str::<Value>(body).ok();
  let code = parsed
    .as_ref()
    .and_then(|v| v.get("code"))
    .map(value_to_display_string)
    .unwrap_or_default();
  let msg = parsed
    .as_ref()
    .and_then(|v| v.get("msg"))
    .map(value_to_display_string)
    .unwrap_or_else(|| body.to_string());

  let hint = if status.as_u16() == 401 {
    "。请检查 App ID/App Secret 是否正确，以及 App Token/Table ID 是否属于该租户"
  } else {
    ""
  };

  if code.trim().is_empty() {
    format!("飞书接口请求失败: HTTP {} {}{}", status.as_u16(), msg, hint)
  } else {
    format!(
      "飞书接口请求失败: HTTP {} code={} {}{}",
      status.as_u16(),
      code,
      msg,
      hint
    )
  }
}

async fn fetch_remote_tasks(app: &AppHandle, filter_completed: bool) -> Result<Vec<Task>, String> {
  let config = load_app_config_from_file(app)?;

  if config.feishu.app_token.trim().is_empty() {
    return Err("App Token 未配置".to_string());
  }
  if config.feishu.table_id.trim().is_empty() {
    return Err("Table ID 未配置".to_string());
  }

  let endpoint = format!(
    "https://open.feishu.cn/open-apis/bitable/v1/apps/{}/tables/{}/records",
    config.feishu.app_token.trim(),
    config.feishu.table_id.trim()
  );

  let client = reqwest::Client::builder()
    .use_rustls_tls()
    .build()
    .map_err(|err| format!("http client init failed: {err}"))?;
  let mut records = Vec::new();
  for attempt in 0..2 {
    let token = get_tenant_access_token(app, attempt == 1).await?;
    let mut req = client
      .get(&endpoint)
      .query(&[("page_size", "100")])
      .header("Content-Type", "application/json")
      .header("Authorization", format!("Bearer {}", token));

    if filter_completed {
      req = req.query(&[("filter", r#"CurrentValue.[状态] != "已完成""#)]);
    }

    let response = req
      .send()
      .await
      .map_err(|err| format!("request failed: {err}"))?;
    let status = response.status();
    let body = response
      .text()
      .await
      .map_err(|err| format!("read response failed: {err}"))?;

    if !status.is_success() {
      if let Ok(value) = serde_json::from_str::<Value>(&body) {
        if let Some(code) = value.get("code").and_then(Value::as_i64) {
          if is_token_invalid_code(code as i32) && attempt == 0 {
            continue;
          }
        }
      }
      return Err(build_feishu_http_error(status, &body));
    }

    let parsed: FeishuRecordsResponse =
      serde_json::from_str(&body).map_err(|err| format!("invalid response: {err}"))?;
    if parsed.code != 0 {
      if is_token_invalid_code(parsed.code) && attempt == 0 {
        continue;
      }
      return Err(format!(
        "飞书返回错误 code={} {}",
        parsed.code,
        parsed.msg.unwrap_or_default()
      ));
    }

    records = parsed
      .data
      .and_then(|d| d.items.or(d.records))
      .unwrap_or_default();
    break;
  }

  Ok(records
    .into_iter()
    .map(|record| {
      let rid = record.record_id;
      let status = field_string(&record.fields, "状态");
      let updated_at = field_string(&record.fields, "任务更新时间");
      Task {
      id: rid.clone(),
      record_id: rid.clone(),
      name: field_string(&record.fields, "任务名称"),
      status,
      priority: field_string(&record.fields, "优先级"),
      task_type: field_string(&record.fields, "类型"),
      time_spent: field_string(&record.fields, "实际耗时(分钟)"),
      created_at: field_string(&record.fields, "任务创建时间"),
      updated_at,
      completed_at: String::new(),
      notes: field_string(&record.fields, "备注/收获"),
      sort_order: 0,
      source: "feishu".to_string(),
      feishu_record_id: rid,
      sync_status: "synced".to_string(),
      last_synced_at: now_iso(),
      retry_count: 0,
      last_error: String::new(),
      last_retry_at: String::new(),
    }})
    .collect())
}

async fn feishu_update_record_fields(
  app: &AppHandle,
  record_id: &str,
  fields: Value,
) -> Result<(), String> {
  let config = load_app_config_from_file(app)?;

  let endpoint = format!(
    "https://open.feishu.cn/open-apis/bitable/v1/apps/{}/tables/{}/records/{}",
    config.feishu.app_token.trim(),
    config.feishu.table_id.trim(),
    record_id
  );

  let client = reqwest::Client::builder()
    .use_rustls_tls()
    .build()
    .map_err(|err| format!("http client init failed: {err}"))?;

  for attempt in 0..2 {
    let token = get_tenant_access_token(app, attempt == 1).await?;
    let response = client
      .put(&endpoint)
      .header("Content-Type", "application/json")
      .header("Authorization", format!("Bearer {}", token))
      .json(&json!({ "fields": fields }))
      .send()
      .await
      .map_err(|err| format!("request failed: {err}"))?;

    let status = response.status();
    let body = response
      .text()
      .await
      .map_err(|err| format!("read response failed: {err}"))?;

    if !status.is_success() {
      if let Ok(value) = serde_json::from_str::<Value>(&body) {
        if let Some(code) = value.get("code").and_then(Value::as_i64) {
          if is_token_invalid_code(code as i32) && attempt == 0 {
            continue;
          }
        }
      }
      return Err(build_feishu_http_error(status, &body));
    }

    let parsed: FeishuGenericResponse =
      serde_json::from_str(&body).map_err(|err| format!("invalid response: {err}"))?;
    if parsed.code != 0 {
      if is_token_invalid_code(parsed.code) && attempt == 0 {
        continue;
      }
      return Err(format!(
        "飞书返回错误 code={} {}",
        parsed.code,
        parsed.msg.unwrap_or_default()
      ));
    }

    return Ok(());
  }

  Err("刷新 token 后仍无法更新记录".to_string())
}

async fn feishu_create_record(app: &AppHandle, task: &Task) -> Result<String, String> {
  let config = load_app_config_from_file(app)?;

  let endpoint = format!(
    "https://open.feishu.cn/open-apis/bitable/v1/apps/{}/tables/{}/records",
    config.feishu.app_token.trim(),
    config.feishu.table_id.trim()
  );

  let client = reqwest::Client::builder()
    .use_rustls_tls()
    .build()
    .map_err(|err| format!("http client init failed: {err}"))?;
  for attempt in 0..2 {
    let token = get_tenant_access_token(app, attempt == 1).await?;
    let response = client
      .post(&endpoint)
      .header("Content-Type", "application/json")
      .header("Authorization", format!("Bearer {}", token))
      .json(&json!({
        "fields": {
          "任务名称": task.name,
          "状态": task.status,
          "优先级": to_feishu_priority_value(&task.priority),
          "类型": task.task_type,
          "备注/收获": task.notes
        }
      }))
      .send()
      .await
      .map_err(|err| format!("request failed: {err}"))?;

    let status = response.status();
    let text = response
      .text()
      .await
      .map_err(|err| format!("read response failed: {err}"))?;

    if !status.is_success() {
      if let Ok(value) = serde_json::from_str::<Value>(&text) {
        if let Some(code) = value.get("code").and_then(Value::as_i64) {
          if is_token_invalid_code(code as i32) && attempt == 0 {
            continue;
          }
        }
      }
      return Err(build_feishu_http_error(status, &text));
    }

    let parsed: FeishuGenericResponse =
      serde_json::from_str(&text).map_err(|err| format!("invalid response: {err}"))?;

    if parsed.code != 0 {
      if is_token_invalid_code(parsed.code) && attempt == 0 {
        continue;
      }
      return Err(format!(
        "飞书返回错误 code={} {}",
        parsed.code,
        parsed.msg.unwrap_or_default()
      ));
    }

    return parsed
      .data
      .and_then(|data| {
        data
          .get("record")
          .and_then(|record| record.get("record_id"))
          .and_then(Value::as_str)
          .map(|v| v.to_string())
          .or_else(|| {
            data
              .get("record_id")
              .and_then(Value::as_str)
              .map(|v| v.to_string())
          })
      })
      .ok_or_else(|| "创建成功但未返回 record_id".to_string());
  }

  Err("刷新 token 后仍无法创建记录".to_string())
}

async fn feishu_delete_record(app: &AppHandle, record_id: &str) -> Result<(), String> {
  let config = load_app_config_from_file(app)?;
  if config.feishu.app_token.trim().is_empty() {
    return Err("App Token 未配置".to_string());
  }
  if config.feishu.table_id.trim().is_empty() {
    return Err("Table ID 未配置".to_string());
  }

  let endpoint = format!(
    "https://open.feishu.cn/open-apis/bitable/v1/apps/{}/tables/{}/records/{}",
    config.feishu.app_token.trim(),
    config.feishu.table_id.trim(),
    record_id
  );

  let client = reqwest::Client::builder()
    .use_rustls_tls()
    .build()
    .map_err(|err| format!("http client init failed: {err}"))?;

  for attempt in 0..2 {
    let token = get_tenant_access_token(app, attempt == 1).await?;
    let response = client
      .delete(&endpoint)
      .header("Content-Type", "application/json")
      .header("Authorization", format!("Bearer {}", token))
      .send()
      .await
      .map_err(|err| format!("request failed: {err}"))?;

    let status = response.status();
    let text = response
      .text()
      .await
      .map_err(|err| format!("read response failed: {err}"))?;

    if !status.is_success() {
      if let Ok(value) = serde_json::from_str::<Value>(&text) {
        if let Some(code) = value.get("code").and_then(Value::as_i64) {
          if is_token_invalid_code(code as i32) && attempt == 0 {
            continue;
          }
        }
      }
      return Err(build_feishu_http_error(status, &text));
    }

    let parsed: FeishuGenericResponse =
      serde_json::from_str(&text).map_err(|err| format!("invalid response: {err}"))?;
    if parsed.code != 0 {
      if is_token_invalid_code(parsed.code) && attempt == 0 {
        continue;
      }
      return Err(format!(
        "飞书返回错误 code={} {}",
        parsed.code,
        parsed.msg.unwrap_or_default()
      ));
    }

    return Ok(());
  }

  Err("刷新 token 后仍无法删除记录".to_string())
}

async fn db_get_all_tasks(app: AppHandle) -> Result<Vec<Task>, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let mut stmt = conn
      .prepare(
        "SELECT record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
         FROM tasks",
      )
      .map_err(|err| format!("prepare query failed: {err}"))?;

    let rows = stmt
      .query_map([], task_from_row)
      .map_err(|err| format!("query tasks failed: {err}"))?;

    let mut tasks = Vec::new();
    for row in rows {
      tasks.push(row.map_err(|err| format!("decode task failed: {err}"))?);
    }

    Ok::<Vec<Task>, String>(tasks)
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

async fn db_get_feishu_tasks(app: AppHandle) -> Result<Vec<Task>, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let mut stmt = conn
      .prepare(
        "SELECT record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
         FROM tasks
         WHERE source = 'feishu'",
      )
      .map_err(|err| format!("prepare query failed: {err}"))?;

    let rows = stmt
      .query_map([], task_from_row)
      .map_err(|err| format!("query tasks failed: {err}"))?;

    let mut tasks = Vec::new();
    for row in rows {
      tasks.push(row.map_err(|err| format!("decode task failed: {err}"))?);
    }

    Ok::<Vec<Task>, String>(tasks)
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

async fn db_prune_stale_feishu_tasks(app: AppHandle, remote_ids: Vec<String>) -> Result<(), String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;

    if remote_ids.is_empty() {
      conn
        .execute(
          "DELETE FROM tasks WHERE source = 'feishu' AND sync_status != 'pending'",
          [],
        )
        .map_err(|err| format!("prune stale feishu tasks failed: {err}"))?;
      return Ok::<(), String>(());
    }

    let placeholders = vec!["?"; remote_ids.len()].join(",");
    let sql = format!(
      "DELETE FROM tasks
       WHERE source = 'feishu'
         AND sync_status != 'pending'
         AND record_id NOT IN ({})",
      placeholders
    );

    conn
      .execute(&sql, rusqlite::params_from_iter(remote_ids.iter()))
      .map_err(|err| format!("prune stale feishu tasks failed: {err}"))?;

    Ok::<(), String>(())
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

async fn db_upsert_tasks(app: AppHandle, tasks: Vec<Task>) -> Result<(), String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let mut conn = open_db(&db_path)?;
    let tx = conn
      .transaction()
      .map_err(|err| format!("begin tx failed: {err}"))?;

    for task in tasks {
      upsert_task(&tx, &task)?;
    }

    tx.commit().map_err(|err| format!("commit tx failed: {err}"))?;
    Ok::<(), String>(())
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

async fn db_update_field_pending(
  app: AppHandle,
  record_id: String,
  field_name: String,
  value: String,
) -> Result<(), String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;

    let column = match field_name.as_str() {
      "任务名称" => "name",
      "状态" => "status",
      "优先级" => "priority",
      "类型" => "task_type",
      "实际耗时(分钟)" => "time_spent",
      "任务创建时间" => "created_at",
      "任务更新时间" => "updated_at",
      "备注/收获" => "notes",
      _ => return Err("不支持的字段名".to_string()),
    };

    let exists: Option<String> = conn
      .query_row(
        "SELECT record_id FROM tasks WHERE record_id = ?1",
        params![record_id.clone()],
        |row| row.get(0),
      )
      .optional()
      .map_err(|err| format!("query record failed: {err}"))?;

    if exists.is_none() {
      conn
        .execute(
          "INSERT INTO tasks (record_id, id, feishu_record_id, source, name, status, sync_status, updated_at, created_at, sort_order)
           VALUES (?1, ?1, ?1, 'feishu', '未命名任务', '待处理', 'pending', ?2, ?2,
             COALESCE((SELECT MAX(sort_order) + 1 FROM tasks), 0)
           )",
          params![record_id.clone(), now_iso()],
        )
        .map_err(|err| format!("insert pending task failed: {err}"))?;
    }

    let completed_at = if field_name == "状态" {
      if value.trim() == "已完成" {
        now_iso()
      } else {
        String::new()
      }
    } else {
      String::new()
    };

    let sql = format!(
      "UPDATE tasks SET {column} = ?1, sync_status = 'pending', source = 'feishu', updated_at = ?2, last_error = ''{} WHERE record_id = ?3",
      if field_name == "状态" {
        ", completed_at = ?4"
      } else {
        ""
      }
    );

    if field_name == "状态" {
      conn
        .execute(&sql, params![value, now_iso(), record_id, completed_at])
        .map_err(|err| format!("update pending field failed: {err}"))?;
    } else {
      conn
        .execute(&sql, params![value, now_iso(), record_id])
        .map_err(|err| format!("update pending field failed: {err}"))?;
    }

    Ok::<(), String>(())
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

async fn db_mark_synced(app: AppHandle, record_id: String) -> Result<(), String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    conn
      .execute(
        "UPDATE tasks
         SET sync_status = 'synced',
             last_synced_at = ?1,
             retry_count = 0,
             last_error = '',
             last_retry_at = ''
         WHERE record_id = ?2",
        params![now_iso(), record_id],
      )
      .map_err(|err| format!("mark synced failed: {err}"))?;
    Ok::<(), String>(())
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

async fn db_mark_push_result(
  app: AppHandle,
  record_id: String,
  error_message: String,
  retryable: bool,
) -> Result<(), String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    conn
      .execute(
        "UPDATE tasks
         SET sync_status = ?1,
             retry_count = COALESCE(retry_count, 0) + 1,
             last_error = ?2,
             last_retry_at = ?3
         WHERE record_id = ?4",
        params![
          if retryable { "pending" } else { "failed" },
          error_message,
          now_iso(),
          record_id
        ],
      )
      .map_err(|err| format!("mark push result failed: {err}"))?;
    Ok::<(), String>(())
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

async fn db_get_pending_tasks(app: AppHandle) -> Result<Vec<Task>, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let mut stmt = conn
      .prepare(
        "SELECT record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
         FROM tasks
         WHERE sync_status IN ('pending', 'failed') AND source = 'feishu'
         ORDER BY updated_at ASC",
      )
      .map_err(|err| format!("prepare pending query failed: {err}"))?;

    let rows = stmt
      .query_map([], task_from_row)
      .map_err(|err| format!("query pending tasks failed: {err}"))?;

    let mut tasks = Vec::new();
    for row in rows {
      tasks.push(row.map_err(|err| format!("decode pending task failed: {err}"))?);
    }

    Ok::<Vec<Task>, String>(tasks)
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

async fn db_get_feishu_sync_meta(app: AppHandle) -> Result<SyncMeta, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let pending_count: i64 = conn
      .query_row(
        "SELECT COUNT(1) FROM tasks WHERE source = 'feishu' AND sync_status = 'pending'",
        [],
        |row| row.get(0),
      )
      .map_err(|err| format!("query pending count failed: {err}"))?;

    let failed_count: i64 = conn
      .query_row(
        "SELECT COUNT(1) FROM tasks WHERE source = 'feishu' AND sync_status = 'failed'",
        [],
        |row| row.get(0),
      )
      .map_err(|err| format!("query failed count failed: {err}"))?;

    let last_error_summary: String = conn
      .query_row(
        "SELECT COALESCE(last_error, '') FROM tasks
         WHERE source = 'feishu' AND sync_status IN ('pending', 'failed') AND COALESCE(last_error, '') != ''
         ORDER BY last_retry_at DESC LIMIT 1",
        [],
        |row| row.get(0),
      )
      .optional()
      .map_err(|err| format!("query last error failed: {err}"))?
      .unwrap_or_default();

    Ok::<SyncMeta, String>(SyncMeta {
      pending_count,
      failed_count,
      last_sync_at: now_iso(),
      last_error_summary,
    })
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

fn is_non_retryable_sync_error(error: &str) -> bool {
  let text = error.trim().to_lowercase();
  if text.is_empty() {
    return false;
  }
  text.contains("http 400")
    || text.contains("http 401")
    || text.contains("http 403")
    || text.contains("recordidnotfound")
    || text.contains("code=1254043")
    || text.contains("权限")
    || text.contains("字段")
    || text.contains("app token 未配置")
    || text.contains("table id 未配置")
    || text.contains("app id 未配置")
    || text.contains("app secret 未配置")
}

async fn db_replace_record_id(app: AppHandle, old_id: String, new_id: String) -> Result<(), String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    conn
      .execute(
        "UPDATE tasks SET record_id = ?1, id = ?1, feishu_record_id = ?1, source = 'feishu', sync_status = 'synced', last_synced_at = ?2 WHERE record_id = ?3",
        params![new_id, now_iso(), old_id],
      )
      .map_err(|err| format!("replace record_id failed: {err}"))?;
    Ok::<(), String>(())
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

async fn db_get_max_sort_order(app: AppHandle, source: String) -> Result<i64, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let max: i64 = conn
      .query_row(
        "SELECT COALESCE(MAX(sort_order), 0) FROM tasks WHERE source = ?1",
        params![source],
        |row| row.get(0),
      )
      .map_err(|err| format!("query max sort failed: {err}"))?;
    Ok::<i64, String>(max)
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

#[tauri::command]
async fn get_local_tasks(app: AppHandle) -> Result<Vec<Task>, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let mut stmt = conn
      .prepare(
        "SELECT record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
         FROM tasks WHERE source = 'local' ORDER BY sort_order ASC, updated_at DESC",
      )
      .map_err(|err| format!("prepare local tasks failed: {err}"))?;
    let rows = stmt
      .query_map([], task_from_row)
      .map_err(|err| format!("query local tasks failed: {err}"))?;
    let mut tasks = Vec::new();
    for row in rows {
      tasks.push(row.map_err(|err| format!("decode local task failed: {err}"))?);
    }
    Ok::<Vec<Task>, String>(tasks)
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

#[tauri::command]
async fn create_local_task(
  app: AppHandle,
  name: String,
  priority: String,
  task_type: String,
) -> Result<Task, String> {
  let task_name = name.trim().to_string();
  if task_name.is_empty() {
    return Err("任务名称不能为空".to_string());
  }
  let id = uuid::Uuid::new_v4().to_string();
  let now = now_iso();
  let max_sort = db_get_max_sort_order(app.clone(), "local".to_string()).await?;
  let task = Task {
    id: id.clone(),
    record_id: id,
    name: task_name,
    status: "待处理".to_string(),
    priority: if priority.trim().is_empty() {
      "普通".to_string()
    } else {
      priority
    },
    task_type: if task_type.trim().is_empty() {
      "日常事务".to_string()
    } else {
      task_type
    },
    time_spent: String::new(),
    created_at: now.clone(),
    updated_at: now,
    completed_at: String::new(),
    notes: String::new(),
    sort_order: max_sort + 1,
    source: "local".to_string(),
    feishu_record_id: String::new(),
    sync_status: "synced".to_string(),
    last_synced_at: String::new(),
    retry_count: 0,
    last_error: String::new(),
    last_retry_at: String::new(),
  };
  db_upsert_tasks(app, vec![task.clone()]).await?;
  Ok(task)
}

#[tauri::command]
async fn update_local_task(
  app: AppHandle,
  id: String,
  fields: HashMap<String, String>,
) -> Result<Task, String> {
  if id.trim().is_empty() {
    return Err("id 不能为空".to_string());
  }
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let mut task: Task = conn
      .query_row(
        "SELECT record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
         FROM tasks WHERE id = ?1 AND source = 'local'",
        params![id.clone()],
        task_from_row,
      )
      .map_err(|err| format!("任务不存在: {err}"))?;

    if let Some(v) = fields.get("name") {
      if !v.trim().is_empty() {
        task.name = v.trim().to_string();
      }
    }
    if let Some(v) = fields.get("status") {
      task.status = v.trim().to_string();
      if task.status == "已完成" {
        task.completed_at = now_iso();
      } else {
        task.completed_at = String::new();
      }
    }
    if let Some(v) = fields.get("priority") {
      task.priority = v.trim().to_string();
    }
    if let Some(v) = fields.get("task_type") {
      task.task_type = v.trim().to_string();
    }
    if let Some(v) = fields.get("notes") {
      task.notes = v.to_string();
    }
    task.updated_at = now_iso();
    task.sync_status = "synced".to_string();

    upsert_task(&conn, &task)?;
    Ok::<Task, String>(task)
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

#[tauri::command]
async fn delete_local_task(app: AppHandle, id: String) -> Result<bool, String> {
  if id.trim().is_empty() {
    return Err("id 不能为空".to_string());
  }
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let affected = conn
      .execute("DELETE FROM tasks WHERE id = ?1 AND source = 'local'", params![id])
      .map_err(|err| format!("delete task failed: {err}"))?;
    Ok::<bool, String>(affected > 0)
  })
  .await
  .map_err(|err| format!("db task join failed: {err}"))?
}

fn init_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
  let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
  let mini_item = MenuItem::with_id(app, "mini", "迷你模式", true, None::<&str>)?;
  let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
  let menu = Menu::with_items(app, &[&show_item, &mini_item, &quit_item])?;

  let app_for_menu = app.clone();
  let app_for_tray = app.clone();

  let mut tray_builder = TrayIconBuilder::new()
    .menu(&menu)
    .show_menu_on_left_click(false)
    .on_menu_event(move |_tray, event| match event.id.as_ref() {
      "show" => {
        if let Some(window) = app_for_menu.get_webview_window(MAIN_WINDOW_LABEL) {
          let _ = set_window_mode_internal(&app_for_menu, "panel");
          let _ = window.unminimize();
          let _ = window.show();
          let _ = window.set_focus();
          let pinned = app_for_menu
            .state::<Mutex<UiState>>()
            .lock()
            .map(|state| state.always_on_top)
            .unwrap_or(true);
          let _ = apply_window_traits(&window, pinned, true, "tray_menu:show:post-show");
        }
      }
      "mini" => {
        if let Some(window) = app_for_menu.get_webview_window(MAIN_WINDOW_LABEL) {
          let _ = set_window_mode_internal(&app_for_menu, "cat");
          let _ = window.unminimize();
          let _ = window.show();
          let _ = window.set_focus();
          let pinned = app_for_menu
            .state::<Mutex<UiState>>()
            .lock()
            .map(|state| state.always_on_top)
            .unwrap_or(true);
          let _ = apply_window_traits(&window, pinned, true, "tray_menu:mini:post-show");
        }
      }
      "quit" => {
        app_for_menu.exit(0);
      }
      _ => {}
    })
    .on_tray_icon_event(move |_tray, event| {
      if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
      } = event
      {
        let _ = toggle_window_visibility(&app_for_tray);
      }
    });

  if let Some(icon) = app.default_window_icon() {
    tray_builder = tray_builder.icon(icon.clone());
  }

  let _ = tray_builder.build(app)?;
  Ok(())
}

fn shortcut_conflict_label(shortcut: &tauri_plugin_global_shortcut::Shortcut) -> Option<&'static str> {
  const RESERVED: [(&str, &str); 9] = [
    ("Cmd+N", "新建任务"),
    ("Cmd+,", "打开设置"),
    ("Cmd+K", "快捷键面板"),
    ("Cmd+1", "筛选：待办"),
    ("Cmd+2", "筛选：进行中"),
    ("Cmd+3", "筛选：已完成"),
    ("Cmd+4", "筛选：全部"),
    ("Cmd+Shift+R", "飞书手动同步"),
    ("Cmd+Shift+L", "主题切换"),
  ];

  for (value, label) in RESERVED {
    if let Ok(reserved) = tauri_plugin_global_shortcut::Shortcut::from_str(value) {
      if reserved.id() == shortcut.id() {
        return Some(label);
      }
    }
  }
  None
}

fn format_shortcut(shortcut: &tauri_plugin_global_shortcut::Shortcut) -> String {
  let mut parts: Vec<String> = Vec::new();
  if shortcut.mods.contains(tauri_plugin_global_shortcut::Modifiers::SUPER) {
    parts.push("Cmd".to_string());
  }
  if shortcut.mods.contains(tauri_plugin_global_shortcut::Modifiers::CONTROL) {
    parts.push("Ctrl".to_string());
  }
  if shortcut.mods.contains(tauri_plugin_global_shortcut::Modifiers::ALT) {
    parts.push("Alt".to_string());
  }
  if shortcut.mods.contains(tauri_plugin_global_shortcut::Modifiers::SHIFT) {
    parts.push("Shift".to_string());
  }

  let key_part = shortcut
    .key
    .to_string()
    .strip_prefix("Key")
    .map_or_else(|| shortcut.key.to_string(), |v| v.to_string());
  parts.push(key_part);
  parts.join("+")
}

fn parse_shortcut_input(raw: &str) -> Result<(tauri_plugin_global_shortcut::Shortcut, String), String> {
  let normalized = raw
    .trim()
    .replace('⌘', "Cmd")
    .replace('⌥', "Alt")
    .replace('⌃', "Ctrl")
    .replace('⇧', "Shift");
  if normalized.is_empty() {
    return Err("快捷键不能为空".to_string());
  }

  let shortcut = tauri_plugin_global_shortcut::Shortcut::from_str(&normalized)
    .map_err(|_| "快捷键格式无效。示例：Cmd+Shift+T".to_string())?;
  if shortcut.mods.is_empty() {
    return Err("快捷键必须包含至少一个修饰键（Cmd/Alt/Ctrl/Shift）".to_string());
  }
  if let Some(label) = shortcut_conflict_label(&shortcut) {
    return Err(format!("快捷键冲突：该组合已用于“{}”", label));
  }

  Ok((shortcut.clone(), format_shortcut(&shortcut)))
}

fn register_shortcut_with_rollback(
  manager: &tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>,
  old: Option<tauri_plugin_global_shortcut::Shortcut>,
  new_shortcut: tauri_plugin_global_shortcut::Shortcut,
) -> Result<(), String> {
  if let Some(prev) = old.clone() {
    let _ = manager.unregister(prev);
  }
  if let Err(err) = manager.register(new_shortcut.clone()) {
    if let Some(prev) = old {
      let _ = manager.register(prev);
    }
    return Err(format!("注册快捷键失败：{err}"));
  }
  Ok(())
}

fn register_toggle_shortcut(
  app: &AppHandle,
  shortcut: tauri_plugin_global_shortcut::Shortcut,
  label: String,
) -> Result<(), String> {
  let manager = app.global_shortcut();
  let state = app.state::<Mutex<GlobalShortcutState>>();

  let old = {
    let guard = state
      .lock()
      .map_err(|_| "failed to lock shortcut state".to_string())?;
    guard.toggle_window.clone()
  };

  register_shortcut_with_rollback(&manager, old, shortcut.clone())?;

  let mut guard = state
    .lock()
    .map_err(|_| "failed to lock shortcut state".to_string())?;
  guard.toggle_window = Some(shortcut);
  guard.toggle_window_text = label;
  Ok(())
}

fn register_mode_shortcut(
  app: &AppHandle,
  shortcut: tauri_plugin_global_shortcut::Shortcut,
  label: String,
) -> Result<(), String> {
  let manager = app.global_shortcut();
  let state = app.state::<Mutex<GlobalShortcutState>>();
  let old = {
    let guard = state
      .lock()
      .map_err(|_| "failed to lock shortcut state".to_string())?;
    guard.toggle_mode.clone()
  };

  register_shortcut_with_rollback(&manager, old, shortcut.clone())?;

  let mut guard = state
    .lock()
    .map_err(|_| "failed to lock shortcut state".to_string())?;
  guard.toggle_mode = Some(shortcut);
  guard.toggle_mode_text = label;
  Ok(())
}

fn set_window_mode_internal(app: &AppHandle, mode: &str) -> Result<(), String> {
  let normalized_mode = normalize_window_mode(mode);
  let window = get_main_window(app)?;
  let ui_state = app.state::<Mutex<UiState>>();
  let mut state = ui_state
    .lock()
    .map_err(|_| "failed to lock ui state".to_string())?;

  if normalized_mode == "cat" {
    apply_mini_mode(app, &window).map_err(|err| err.to_string())?;
    state.mini_mode = true;
  } else {
    apply_normal_mode(&window).map_err(|err| err.to_string())?;
    state.mini_mode = false;
  }
  apply_window_traits(
    &window,
    state.always_on_top,
    should_include_native_traits("set_window_mode_internal"),
    "set_window_mode_internal",
  )?;

  let payload = WindowModeChangedPayload {
    mode: normalized_mode.clone(),
    mini_mode: state.mini_mode,
  };
  drop(state);
  let _ = app.emit("window-mode-changed", payload);
  let mut cfg = load_app_config_from_file(app)?;
  cfg.pet.window_mode = normalized_mode;
  save_app_config_to_file(app, &cfg)?;
  Ok(())
}

fn init_global_shortcut(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
  use tauri_plugin_global_shortcut::ShortcutState;

  let app_for_handler = app.clone();

  app.plugin(
    tauri_plugin_global_shortcut::Builder::new()
      .with_handler(move |_app, current_shortcut, event| {
        if event.state() == ShortcutState::Pressed {
          if let Ok(state) = app_for_handler.state::<Mutex<GlobalShortcutState>>().lock() {
            let shortcut_id = current_shortcut.id();
            if state
              .toggle_window
              .as_ref()
              .map(|shortcut| shortcut.id() == shortcut_id)
              .unwrap_or(false)
            {
              let _ = toggle_window_visibility(&app_for_handler);
              return;
            }
            if state
              .toggle_mode
              .as_ref()
              .map(|shortcut| shortcut.id() == shortcut_id)
              .unwrap_or(false)
            {
              let current_mode = if let Ok(ui_state) = app_for_handler.state::<Mutex<UiState>>().lock() {
                if ui_state.mini_mode { "cat" } else { "panel" }
              } else {
                "panel"
              };
              let next_mode = if current_mode == "cat" { "panel" } else { "cat" };
              let _ = set_window_mode_internal(&app_for_handler, next_mode);
            }
          }
        }
      })
      .build(),
  )?;

  let cfg = load_app_config_from_file(app).unwrap_or_else(|_| default_app_config());
  let raw_shortcut = cfg.shortcut.toggle_window.trim();
  let shortcut_text = if raw_shortcut.is_empty() {
    DEFAULT_TOGGLE_SHORTCUT
  } else {
    raw_shortcut
  };

  match parse_shortcut_input(shortcut_text) {
    Ok((shortcut, label)) => {
      if let Err(err) = register_toggle_shortcut(app, shortcut, label.clone()) {
        eprintln!("[Rust] register configured shortcut failed: {}", err);
        let (fallback_shortcut, fallback_label) =
          parse_shortcut_input(DEFAULT_TOGGLE_SHORTCUT).expect("default shortcut should be valid");
        register_toggle_shortcut(app, fallback_shortcut, fallback_label)
          .map_err(Box::<dyn std::error::Error>::from)?;
      }
    }
    Err(err) => {
      eprintln!("[Rust] parse configured shortcut failed: {}", err);
      let (fallback_shortcut, fallback_label) =
        parse_shortcut_input(DEFAULT_TOGGLE_SHORTCUT).expect("default shortcut should be valid");
      register_toggle_shortcut(app, fallback_shortcut, fallback_label)
        .map_err(Box::<dyn std::error::Error>::from)?;
    }
  }

  let raw_mode_shortcut = cfg.shortcut.toggle_mode.trim();
  let mode_shortcut_text = if raw_mode_shortcut.is_empty() {
    DEFAULT_TOGGLE_MODE_SHORTCUT
  } else {
    raw_mode_shortcut
  };
  match parse_shortcut_input(mode_shortcut_text) {
    Ok((shortcut, label)) => {
      if let Ok(lock) = app.state::<Mutex<GlobalShortcutState>>().lock() {
        if lock
          .toggle_window
          .as_ref()
          .map(|v| v.id() == shortcut.id())
          .unwrap_or(false)
        {
          let fallback = parse_shortcut_input(DEFAULT_TOGGLE_MODE_SHORTCUT)
            .expect("default mode shortcut should be valid");
          register_mode_shortcut(app, fallback.0, fallback.1)
            .map_err(Box::<dyn std::error::Error>::from)?;
          return Ok(());
        }
      }
      if let Err(err) = register_mode_shortcut(app, shortcut, label.clone()) {
        eprintln!("[Rust] register configured mode shortcut failed: {}", err);
        let (fallback_shortcut, fallback_label) =
          parse_shortcut_input(DEFAULT_TOGGLE_MODE_SHORTCUT)
            .expect("default mode shortcut should be valid");
        register_mode_shortcut(app, fallback_shortcut, fallback_label)
          .map_err(Box::<dyn std::error::Error>::from)?;
      }
    }
    Err(err) => {
      eprintln!("[Rust] parse configured mode shortcut failed: {}", err);
      let (fallback_shortcut, fallback_label) = parse_shortcut_input(DEFAULT_TOGGLE_MODE_SHORTCUT)
        .expect("default mode shortcut should be valid");
      register_mode_shortcut(app, fallback_shortcut, fallback_label)
        .map_err(Box::<dyn std::error::Error>::from)?;
    }
  }

  Ok(())
}

#[tauri::command]
async fn check_feishu_api_client() -> Result<SyncStatus, String> {
  let client = reqwest::Client::builder()
    .use_rustls_tls()
    .build()
    .map_err(|err| format!("failed to build reqwest client: {err}"))?;

  drop(client);

  Ok(SyncStatus {
    service: "feishu-bitable".to_string(),
    healthy: true,
  })
}

#[tauri::command]
fn get_window_state(state: State<'_, Mutex<UiState>>) -> Result<WindowStatePayload, String> {
  let state = state
    .lock()
    .map_err(|_| "failed to lock ui state".to_string())?;

  Ok(WindowStatePayload {
    mini_mode: state.mini_mode,
    always_on_top: state.always_on_top,
  })
}

#[tauri::command]
fn reapply_window_traits(app: AppHandle, state: State<'_, Mutex<UiState>>) -> Result<(), String> {
  let pinned = state
    .lock()
    .map_err(|_| "failed to lock ui state".to_string())?
    .always_on_top;
  let window = get_main_window(&app)?;
  apply_window_traits(&window, pinned, true, "reapply_window_traits")
}

#[tauri::command]
fn toggle_always_on_top(
  app: AppHandle,
  state: State<'_, Mutex<UiState>>,
) -> Result<bool, String> {
  let window = get_main_window(&app)?;
  let mut state = state
    .lock()
    .map_err(|_| "failed to lock ui state".to_string())?;

  state.always_on_top = !state.always_on_top;
  apply_window_traits(
    &window,
    state.always_on_top,
    should_include_native_traits("toggle_always_on_top"),
    "toggle_always_on_top",
  )?;

  Ok(state.always_on_top)
}

#[tauri::command]
fn enter_mini_mode(app: AppHandle, state: State<'_, Mutex<UiState>>) -> Result<(), String> {
  let _ = state;
  set_window_mode_internal(&app, "cat")
}

#[tauri::command]
fn restore_normal_mode(app: AppHandle, state: State<'_, Mutex<UiState>>) -> Result<(), String> {
  let _ = state;
  set_window_mode_internal(&app, "panel")
}

#[tauri::command]
fn hide_window_to_tray(app: AppHandle) -> Result<(), String> {
  let window = get_main_window(&app)?;
  window.hide().map_err(|err| err.to_string())
}

#[tauri::command]
async fn save_config(
  app: AppHandle,
  mode: String,
  app_id: String,
  app_secret: String,
  app_token: String,
  table_id: String,
  folder_token: Option<String>,
  collaborator_email: Option<String>,
  sync_interval: Option<i64>,
) -> Result<(), String> {
  println!(
    "[Rust] save_config: mode={}, app_id_len={}, app_token_len={}, table_id_len={}, has_secret_input={}",
    mode,
    app_id.trim().len(),
    app_token.trim().len(),
    table_id.trim().len(),
    !app_secret.trim().is_empty()
  );
  let normalized_mode = normalize_app_mode(&mode);
  if normalized_mode.is_empty() {
    return Err("mode 必须是 local 或 feishu".to_string());
  }

  if normalized_mode == "feishu" {
    if app_id.trim().is_empty() {
      return Err("App ID 不能为空".to_string());
    }
    if app_token.trim().is_empty() {
      return Err("App Token 不能为空".to_string());
    }
    if table_id.trim().is_empty() {
      return Err("Table ID 不能为空".to_string());
    }
  }

  let mut cfg = load_app_config_from_file(&app)?;
  cfg.mode = normalized_mode;
  let incoming_app_id = app_id.trim().to_string();
  let incoming_app_token = app_token.trim().to_string();
  let incoming_table_id = table_id.trim().to_string();
  let incoming_folder_token = folder_token.unwrap_or_default().trim().to_string();
  let incoming_collaborator_email = collaborator_email.unwrap_or_default().trim().to_string();
  if cfg.mode == "feishu" || !incoming_app_id.is_empty() {
    cfg.feishu.app_id = incoming_app_id;
  }
  if cfg.mode == "feishu" || !incoming_app_token.is_empty() {
    cfg.feishu.app_token = incoming_app_token;
  }
  if cfg.mode == "feishu" || !incoming_table_id.is_empty() {
    cfg.feishu.table_id = incoming_table_id;
  }
  if cfg.mode == "feishu" || !incoming_folder_token.is_empty() {
    cfg.feishu.folder_token = incoming_folder_token;
  }
  if cfg.mode == "feishu" || !incoming_collaborator_email.is_empty() {
    cfg.feishu.collaborator_email = incoming_collaborator_email;
  }
  cfg.sync_interval = default_sync_interval_seconds(sync_interval.unwrap_or(cfg.sync_interval));
  if cfg.created_at.trim().is_empty() {
    cfg.created_at = now_iso();
  }

  if !app_secret.trim().is_empty() {
    cfg.feishu.encrypted_app_secret = encrypt_token(app_secret.trim())?;
  } else if cfg.mode == "feishu" && cfg.feishu.encrypted_app_secret.trim().is_empty() {
    return Err("App Secret 不能为空".to_string());
  }

  save_app_config_to_file(&app, &cfg)?;
  println!("[Rust] save_config: persisted mode={}", cfg.mode);

  if cfg.mode == "feishu" {
    get_tenant_access_token(&app, true).await.map(|_| ())?;
    println!("[Rust] save_config: tenant_access_token refreshed");
  }

  Ok(())
}

#[tauri::command]
fn load_config(app: AppHandle) -> Result<ConfigPayload, String> {
  let cfg = load_app_config_from_file(&app)?;
  let mode = if cfg.mode.trim().is_empty() {
    "local".to_string()
  } else {
    cfg.mode.clone()
  };

  Ok(ConfigPayload {
    mode,
    app_id: cfg.feishu.app_id,
    app_token: cfg.feishu.app_token,
    table_id: cfg.feishu.table_id,
    folder_token: cfg.feishu.folder_token,
    collaborator_email: cfg.feishu.collaborator_email,
    has_secret: !cfg.feishu.encrypted_app_secret.trim().is_empty(),
  })
}

#[tauri::command]
fn get_shortcut_config(
  app: AppHandle,
  state: State<'_, Mutex<GlobalShortcutState>>,
) -> Result<ShortcutConfigPayload, String> {
  let from_runtime = state
    .lock()
    .map_err(|_| "failed to lock shortcut state".to_string())?
    .toggle_window_text
    .trim()
    .to_string();

  if !from_runtime.is_empty() {
    return Ok(ShortcutConfigPayload {
      toggle_window: from_runtime,
    });
  }

  let cfg = load_app_config_from_file(&app)?;
  let raw = cfg.shortcut.toggle_window.trim();
  let value = if raw.is_empty() {
    DEFAULT_TOGGLE_SHORTCUT.to_string()
  } else {
    raw.to_string()
  };

  Ok(ShortcutConfigPayload {
    toggle_window: value,
  })
}

#[tauri::command]
fn set_shortcut_config(
  app: AppHandle,
  toggle_window: String,
) -> Result<SetShortcutConfigResult, String> {
  let previous = {
    let state = app.state::<Mutex<GlobalShortcutState>>();
    let guard = state
      .lock()
      .map_err(|_| "failed to lock shortcut state".to_string())?;
    (
      guard.toggle_window.clone(),
      guard.toggle_window_text.clone(),
      guard.toggle_mode.clone(),
    )
  };

  let (shortcut, label) = parse_shortcut_input(&toggle_window)?;
  if previous
    .2
    .as_ref()
    .map(|v| v.id() == shortcut.id())
    .unwrap_or(false)
  {
    return Err("快捷键冲突：与“形态切换快捷键”重复".to_string());
  }
  register_toggle_shortcut(&app, shortcut, label.clone())?;

  let mut cfg = load_app_config_from_file(&app)?;
  cfg.shortcut.toggle_window = label.clone();
  if let Err(err) = save_app_config_to_file(&app, &cfg) {
    if let Some(prev_shortcut) = previous.0 {
      let rollback_label = if previous.1.trim().is_empty() {
        format_shortcut(&prev_shortcut)
      } else {
        previous.1
      };
      let _ = register_toggle_shortcut(&app, prev_shortcut, rollback_label);
    }
    return Err(err);
  }

  Ok(SetShortcutConfigResult {
    success: true,
    message: "快捷键已更新".to_string(),
    applied: Some(label),
  })
}

#[tauri::command]
fn get_mode_shortcut_config(
  app: AppHandle,
  state: State<'_, Mutex<GlobalShortcutState>>,
) -> Result<ModeShortcutConfigPayload, String> {
  let from_runtime = state
    .lock()
    .map_err(|_| "failed to lock shortcut state".to_string())?
    .toggle_mode_text
    .trim()
    .to_string();
  if !from_runtime.is_empty() {
    return Ok(ModeShortcutConfigPayload {
      toggle_mode: from_runtime,
    });
  }

  let cfg = load_app_config_from_file(&app)?;
  let raw = cfg.shortcut.toggle_mode.trim();
  let value = if raw.is_empty() {
    DEFAULT_TOGGLE_MODE_SHORTCUT.to_string()
  } else {
    raw.to_string()
  };
  Ok(ModeShortcutConfigPayload {
    toggle_mode: value,
  })
}

#[tauri::command]
fn set_mode_shortcut_config(
  app: AppHandle,
  toggle_mode: String,
) -> Result<SetShortcutConfigResult, String> {
  let previous = {
    let state = app.state::<Mutex<GlobalShortcutState>>();
    let guard = state
      .lock()
      .map_err(|_| "failed to lock shortcut state".to_string())?;
    (
      guard.toggle_mode.clone(),
      guard.toggle_mode_text.clone(),
      guard.toggle_window.clone(),
    )
  };

  let (shortcut, label) = parse_shortcut_input(&toggle_mode)?;
  if previous
    .2
    .as_ref()
    .map(|v| v.id() == shortcut.id())
    .unwrap_or(false)
  {
    return Err("快捷键冲突：与“唤起窗口快捷键”重复".to_string());
  }

  register_mode_shortcut(&app, shortcut, label.clone())?;

  let mut cfg = load_app_config_from_file(&app)?;
  cfg.shortcut.toggle_mode = label.clone();
  if let Err(err) = save_app_config_to_file(&app, &cfg) {
    if let Some(prev_shortcut) = previous.0 {
      let rollback_label = if previous.1.trim().is_empty() {
        format_shortcut(&prev_shortcut)
      } else {
        previous.1
      };
      let _ = register_mode_shortcut(&app, prev_shortcut, rollback_label);
    }
    return Err(err);
  }

  Ok(SetShortcutConfigResult {
    success: true,
    message: "形态切换快捷键已更新".to_string(),
    applied: Some(label),
  })
}

#[tauri::command]
fn set_window_mode(app: AppHandle, mode: String) -> Result<(), String> {
  set_window_mode_internal(&app, &mode)
}

#[tauri::command]
fn get_pet_settings(app: AppHandle) -> Result<PetSettingsPayload, String> {
  let cfg = load_app_config_from_file(&app)?;
  Ok(PetSettingsPayload {
    enabled: cfg.pet.enabled,
    show_badge: cfg.pet.show_badge,
    animations: cfg.pet.animations,
    cat_position: PetPositionPayload {
      x: cfg.pet.cat_position.x,
      y: cfg.pet.cat_position.y,
    },
    window_mode: normalize_window_mode(&cfg.pet.window_mode),
    daily_progress_date: cfg.pet.daily_progress_date,
    daily_progress_level: normalize_daily_progress_level(cfg.pet.daily_progress_level),
  })
}

#[tauri::command]
fn save_pet_settings(
  app: AppHandle,
  enabled: bool,
  show_badge: bool,
  animations: bool,
  cat_x: Option<f64>,
  cat_y: Option<f64>,
  window_mode: Option<String>,
  daily_progress_date: Option<String>,
  daily_progress_level: Option<i32>,
) -> Result<(), String> {
  let mut cfg = load_app_config_from_file(&app)?;
  cfg.pet.enabled = enabled;
  cfg.pet.show_badge = show_badge;
  cfg.pet.animations = animations;
  if let Some(x) = cat_x {
    cfg.pet.cat_position.x = x;
  }
  if let Some(y) = cat_y {
    cfg.pet.cat_position.y = y;
  }
  if let Some(mode) = window_mode {
    cfg.pet.window_mode = normalize_window_mode(&mode);
  }
  if let Some(date) = daily_progress_date {
    cfg.pet.daily_progress_date = date.trim().to_string();
  }
  if let Some(level) = daily_progress_level {
    cfg.pet.daily_progress_level = normalize_daily_progress_level(level);
  }
  save_app_config_to_file(&app, &cfg)?;
  Ok(())
}

#[tauri::command]
fn get_app_mode(app: AppHandle) -> Result<String, String> {
  let path = config_file_path(&app)?;
  if !path.exists() {
    println!("[Rust] get_app_mode 返回: <empty/no-config>");
    return Ok(String::new());
  }
  let cfg = load_app_config_from_file(&app)?;
  let mode = normalize_app_mode(&cfg.mode);
  println!("[Rust] get_app_mode 返回: {}", mode);
  Ok(mode)
}

#[tauri::command]
fn set_app_mode(app: AppHandle, mode: String) -> Result<(), String> {
  println!("[Rust] set_app_mode: {}", mode);
  let normalized = normalize_app_mode(&mode);
  if normalized.is_empty() {
    return Err("mode 必须是 local 或 feishu".to_string());
  }

  let mut cfg = load_app_config_from_file(&app).unwrap_or_else(|_| default_app_config());
  cfg.mode = normalized;
  if cfg.created_at.trim().is_empty() {
    cfg.created_at = now_iso();
  }
  let res = save_app_config_to_file(&app, &cfg);
  if res.is_ok() {
    println!("[Rust] set_app_mode persisted: {}", cfg.mode);
  }
  res
}

#[tauri::command]
fn save_task_order(app: AppHandle, order: Vec<String>) -> Result<(), String> {
  let dir = config_dir(&app)?;
  fs::create_dir_all(&dir).map_err(|err| format!("failed to create config dir: {err}"))?;

  let sanitized: Vec<String> = order
    .into_iter()
    .map(|item| item.trim().to_string())
    .filter(|item| !item.is_empty())
    .collect();

  let file = task_order_file_path(&app)?;
  let content = serde_json::to_string_pretty(&sanitized)
    .map_err(|err| format!("failed to serialize task order: {err}"))?;
  fs::write(file, content).map_err(|err| format!("failed to write task order: {err}"))
}

#[tauri::command]
fn load_task_order(app: AppHandle) -> Result<Vec<String>, String> {
  let file = task_order_file_path(&app)?;
  if !file.exists() {
    return Ok(Vec::new());
  }

  let content =
    fs::read_to_string(file).map_err(|err| format!("failed to read task order: {err}"))?;
  let parsed =
    serde_json::from_str::<Vec<String>>(&content).map_err(|err| format!("invalid task order: {err}"))?;

  Ok(parsed
    .into_iter()
    .map(|item| item.trim().to_string())
    .filter(|item| !item.is_empty())
    .collect())
}

#[tauri::command]
async fn save_window_size(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
  if !(width.is_finite() && height.is_finite()) {
    return Err("窗口尺寸无效".to_string());
  }

  let size = WindowSizePayload { width, height };
  let file = window_size_file_path(&app)?;

  tokio::task::spawn_blocking(move || {
    if let Some(parent) = file.parent() {
      fs::create_dir_all(parent).map_err(|err| format!("保存窗口尺寸失败: {err}"))?;
    }
    let content =
      serde_json::to_string_pretty(&size).map_err(|err| format!("序列化窗口尺寸失败: {err}"))?;
    fs::write(file, content).map_err(|err| format!("保存窗口尺寸失败: {err}"))?;
    Ok::<(), String>(())
  })
  .await
  .map_err(|err| format!("保存窗口尺寸任务失败: {err}"))?
}

#[tauri::command]
async fn get_window_size(app: AppHandle) -> Result<Option<WindowSizePayload>, String> {
  let file = window_size_file_path(&app)?;

  tokio::task::spawn_blocking(move || {
    if !file.exists() {
      return Ok::<Option<WindowSizePayload>, String>(None);
    }
    let content = fs::read_to_string(file).map_err(|err| format!("读取窗口尺寸失败: {err}"))?;
    let size: WindowSizePayload =
      serde_json::from_str(&content).map_err(|err| format!("解析窗口尺寸失败: {err}"))?;
    Ok(Some(size))
  })
  .await
  .map_err(|err| format!("读取窗口尺寸任务失败: {err}"))?
}

#[tauri::command]
async fn reorder_local_tasks(app: AppHandle, ordered_ids: Vec<String>) -> Result<(), String> {
  let db_path = db_file_path(&app)?;
  let ids: Vec<String> = ordered_ids
    .into_iter()
    .map(|v| v.trim().to_string())
    .filter(|v| !v.is_empty())
    .collect();
  let join_result = tokio::task::spawn_blocking(move || {
    let mut conn = open_db(&db_path)?;
    let tx = conn
      .transaction()
      .map_err(|err| format!("begin reorder tx failed: {err}"))?;
    for (idx, id) in ids.iter().enumerate() {
      tx.execute(
        "UPDATE tasks SET sort_order = ?1 WHERE id = ?2 AND source = 'local'",
        params![idx as i64, id],
      )
      .map_err(|err| format!("update sort_order failed: {err}"))?;
    }
    tx.commit()
      .map_err(|err| format!("commit reorder tx failed: {err}"))?;
    Ok::<(), String>(())
  })
  .await;
  match join_result {
    Ok(inner) => inner,
    Err(err) => Err(format!("db task join failed: {err}")),
  }
}

#[tauri::command]
async fn test_connection(app: AppHandle) -> Result<ConnectionTestResult, String> {
  let config = load_app_config_from_file(&app)?;

  if config.feishu.app_id.trim().is_empty() {
    return Err("App ID 未配置".to_string());
  }
  if config.feishu.app_token.trim().is_empty() {
    return Err("App Token 未配置".to_string());
  }
  if config.feishu.encrypted_app_secret.trim().is_empty() {
    return Err("App Secret 未配置".to_string());
  }

  let endpoint = format!(
    "https://open.feishu.cn/open-apis/bitable/v1/apps/{}/tables",
    config.feishu.app_token.trim()
  );

  let client = reqwest::Client::builder()
    .use_rustls_tls()
    .build()
    .map_err(|err| format!("http client init failed: {err}"))?;

  for attempt in 0..2 {
    let token = get_tenant_access_token(&app, attempt == 1).await?;
    let response = client
      .get(&endpoint)
      .header("Content-Type", "application/json")
      .header("Authorization", format!("Bearer {}", token))
      .send()
      .await
      .map_err(|err| format!("request failed: {err}"))?;

    let status = response.status();
    let body = response
      .text()
      .await
      .map_err(|err| format!("read response failed: {err}"))?;

    let parsed = serde_json::from_str::<FeishuTablesResponse>(&body).ok();

    if status.is_success() {
      if let Some(data) = parsed {
        if data.code == 0 {
          return Ok(ConnectionTestResult {
            success: true,
            message: "连接成功：飞书多维表格 API 可访问".to_string(),
          });
        }
        if is_token_invalid_code(data.code) && attempt == 0 {
          continue;
        }
        return Ok(ConnectionTestResult {
          success: false,
          message: format!(
            "飞书返回错误 code={} {}",
            data.code,
            data.msg.unwrap_or_default()
          ),
        });
      }

      return Ok(ConnectionTestResult {
        success: false,
        message: "飞书返回格式异常：未解析到 code 字段".to_string(),
      });
    }

    if let Some(data) = &parsed {
      if is_token_invalid_code(data.code) && attempt == 0 {
        continue;
      }
    }

    let api_message = parsed.and_then(|v| v.msg).unwrap_or_default();
    return Ok(ConnectionTestResult {
      success: false,
      message: format!("HTTP {} {}", status.as_u16(), api_message),
    });
  }

  Ok(ConnectionTestResult {
    success: false,
    message: "连接失败：token 刷新重试后仍失败".to_string(),
  })
}

#[tauri::command]
async fn fetch_tasks(app: AppHandle) -> Result<Vec<Task>, String> {
  fetch_remote_tasks(&app, true).await
}

#[tauri::command]
async fn get_cached_tasks(app: AppHandle) -> Result<Vec<Task>, String> {
  db_get_all_tasks(app).await
}

#[tauri::command]
async fn cache_tasks(app: AppHandle, tasks: Vec<Task>) -> Result<(), String> {
  db_upsert_tasks(app, tasks).await
}

#[tauri::command]
async fn update_task(
  app: AppHandle,
  record_id: String,
  field_name: String,
  value: String,
) -> Result<UpdateTaskResult, String> {
  if record_id.trim().is_empty() {
    return Err("record_id 不能为空".to_string());
  }
  if field_name.trim().is_empty() {
    return Err("field_name 不能为空".to_string());
  }

  db_update_field_pending(
    app.clone(),
    record_id.clone(),
    field_name.clone(),
    value.clone(),
  )
  .await?;

  let remote_value = if field_name.trim() == "优先级" {
    to_feishu_priority_value(&value)
  } else {
    value.clone()
  };
  let fields = json!({ field_name.trim(): remote_value });
  match feishu_update_record_fields(&app, record_id.trim(), fields).await {
    Ok(_) => {
      db_mark_synced(app, record_id).await?;
      Ok(UpdateTaskResult {
        success: true,
        message: "同步成功".to_string(),
      })
    }
    Err(err) => {
      let retryable = !is_non_retryable_sync_error(&err);
      let _ = db_mark_push_result(app.clone(), record_id.clone(), err.clone(), retryable).await;
      let _ = db_update_field_pending(app, record_id, field_name, value).await;
      Ok(UpdateTaskResult {
        success: false,
        message: format!("离线缓存，待重试：{err}"),
      })
    }
  }
}

#[tauri::command]
async fn create_task(
  app: AppHandle,
  name: String,
  sync_guard: State<'_, tokio::sync::Mutex<()>>,
) -> Result<CreateTaskResult, String> {
  let task_name = name.trim().to_string();
  if task_name.is_empty() {
    return Err("任务名称不能为空".to_string());
  }

  let local_id = format!("temp-{}", now_unix_millis());
  let local_task = Task {
    id: local_id.clone(),
    record_id: local_id.clone(),
    name: task_name,
    status: "待处理".to_string(),
    priority: "普通".to_string(),
    task_type: "日常事务".to_string(),
    time_spent: String::new(),
    created_at: now_iso(),
    updated_at: now_iso(),
    completed_at: String::new(),
    notes: String::new(),
    sort_order: 0,
    source: "feishu".to_string(),
    feishu_record_id: String::new(),
    sync_status: "pending".to_string(),
    last_synced_at: String::new(),
    retry_count: 0,
    last_error: String::new(),
    last_retry_at: String::new(),
  };

  db_upsert_tasks(app.clone(), vec![local_task.clone()]).await?;

  let _guard = sync_guard.lock().await;
  match feishu_create_record(&app, &local_task).await {
    Ok(remote_id) => {
      db_replace_record_id(app, local_id, remote_id.clone()).await?;
      Ok(CreateTaskResult {
        record_id: remote_id,
        synced: true,
      })
    }
    Err(err) => {
      let retryable = !is_non_retryable_sync_error(&err);
      let _ = db_mark_push_result(app.clone(), local_id.clone(), err.clone(), retryable).await;
      eprintln!("[Rust] create_task remote create failed, keep pending: {}", err);
      Ok(CreateTaskResult {
        record_id: local_id,
        synced: false,
      })
    }
  }
}

#[tauri::command]
async fn delete_task(app: AppHandle, record_id: String) -> Result<UpdateTaskResult, String> {
  let rid = record_id.trim().to_string();
  if rid.is_empty() {
    return Err("record_id 不能为空".to_string());
  }

  match feishu_delete_record(&app, &rid).await {
    Ok(_) => {
      let db_path = db_file_path(&app)?;
      let delete_id = rid.clone();
      tokio::task::spawn_blocking(move || {
        let conn = open_db(&db_path)?;
        conn
          .execute("DELETE FROM tasks WHERE record_id = ?1", params![delete_id])
          .map_err(|err| format!("delete local cache failed: {err}"))?;
        Ok::<(), String>(())
      })
      .await
      .map_err(|err| format!("db task join failed: {err}"))??;

      Ok(UpdateTaskResult {
        success: true,
        message: "删除成功".to_string(),
      })
    }
    Err(err) => Ok(UpdateTaskResult {
      success: false,
      message: err,
    }),
  }
}

#[tauri::command]
async fn sync_tasks(
  app: AppHandle,
  sync_guard: State<'_, tokio::sync::Mutex<()>>,
) -> Result<SyncTasksResult, String> {
  let _guard = sync_guard.lock().await;
  let remote_tasks = fetch_remote_tasks(&app, false).await?;
  let remote_ids: Vec<String> = remote_tasks.iter().map(|task| task.record_id.clone()).collect();

  // 远端优先：先覆盖本地同 record_id 数据
  db_upsert_tasks(app.clone(), remote_tasks).await?;

  // 清理远端已不存在的历史飞书记录，避免数量越积越多
  db_prune_stale_feishu_tasks(app.clone(), remote_ids).await?;

  // 再推送仍然 pending 的本地改动（通常是本地新建 temp 或远端不存在的记录）
  let pending_tasks = db_get_pending_tasks(app.clone()).await?;
  for task in pending_tasks {
    if task.record_id.starts_with("temp-") {
      match feishu_create_record(&app, &task).await {
        Ok(remote_id) => {
          let _ = db_replace_record_id(app.clone(), task.record_id.clone(), remote_id).await;
        }
        Err(err) => {
          let retryable = !is_non_retryable_sync_error(&err);
          let _ = db_mark_push_result(app.clone(), task.record_id.clone(), err, retryable).await;
        }
      }
      continue;
    }

    let fields = json!({
      "任务名称": task.name,
      "状态": task.status,
      "优先级": to_feishu_priority_value(&task.priority),
      "类型": task.task_type,
      "实际耗时(分钟)": task.time_spent,
      "任务创建时间": task.created_at,
      "任务更新时间": task.updated_at,
      "备注/收获": task.notes,
    });

    match feishu_update_record_fields(&app, &task.record_id, fields).await {
      Ok(_) => {
        let _ = db_mark_synced(app.clone(), task.record_id.clone()).await;
      }
      Err(err) => {
        let retryable = !is_non_retryable_sync_error(&err);
        let _ = db_mark_push_result(app.clone(), task.record_id.clone(), err, retryable).await;
      }
    }
  }

  let tasks = db_get_feishu_tasks(app.clone()).await?;
  let sync_meta = db_get_feishu_sync_meta(app).await?;
  Ok(SyncTasksResult { tasks, sync_meta })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_autostart::init(
      tauri_plugin_autostart::MacosLauncher::LaunchAgent,
      None,
    ))
    .plugin(tauri_plugin_shell::init())
    .manage(Mutex::new(UiState {
      mini_mode: false,
      always_on_top: true,
    }))
    .manage(Mutex::new(ConfigIoLock))
    .manage(Mutex::new(GlobalShortcutState::default()))
    .manage(tokio::sync::RwLock::new(TokenManager::default()))
    .manage(tokio::sync::Mutex::new(()))
    .setup(|app| {
      if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let pinned = app
          .state::<Mutex<UiState>>()
          .lock()
          .map(|state| state.always_on_top)
          .unwrap_or(true);
        let _ = apply_window_traits(&window, pinned, should_include_native_traits("setup"), "setup");
      }

      #[cfg(desktop)]
      {
        init_tray(app.handle())?;
        init_global_shortcut(app.handle())?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      check_feishu_api_client,
      get_window_state,
      reapply_window_traits,
      toggle_always_on_top,
      enter_mini_mode,
      restore_normal_mode,
      hide_window_to_tray,
      save_config,
      load_config,
      get_shortcut_config,
      set_shortcut_config,
      get_mode_shortcut_config,
      set_mode_shortcut_config,
      set_window_mode,
      get_pet_settings,
      save_pet_settings,
      get_app_mode,
      set_app_mode,
      save_task_order,
      load_task_order,
      save_window_size,
      get_window_size,
      reorder_local_tasks,
      test_connection,
      get_local_tasks,
      create_local_task,
      update_local_task,
      delete_local_task,
      fetch_tasks,
      get_cached_tasks,
      cache_tasks,
      update_task,
      create_task,
      delete_task,
      sync_tasks
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
