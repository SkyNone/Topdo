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
  AppHandle, LogicalSize, Manager, Size, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

const MAIN_WINDOW_LABEL: &str = "main";
const QUICK_CAPTURE_WINDOW_LABEL: &str = "quick-capture";
const NORMAL_WIDTH: f64 = 320.0;
const NORMAL_HEIGHT: f64 = 500.0;
const NORMAL_MIN_WIDTH: f64 = 320.0;
const NORMAL_MIN_HEIGHT: f64 = 300.0;
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
const DEFAULT_QUICK_CAPTURE_SHORTCUT: &str = "Alt+Space";

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
    sub_tasks TEXT DEFAULT '[]',
    tags TEXT DEFAULT '[]',
    due_date TEXT DEFAULT '',
    source TEXT DEFAULT 'local',
    feishu_record_id TEXT DEFAULT '',
    sync_status TEXT DEFAULT 'synced',
    last_synced_at TEXT DEFAULT '',
    retry_count INTEGER DEFAULT 0,
    last_error TEXT DEFAULT '',
    last_retry_at TEXT DEFAULT ''
);
"#;

const CREATE_HABITS_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS habits (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  emoji TEXT NOT NULL DEFAULT '🎯',
  color TEXT NOT NULL DEFAULT '#10b981',
  frequency_type TEXT NOT NULL DEFAULT 'daily',
  frequency_days TEXT DEFAULT NULL,
  remind_time TEXT DEFAULT NULL,
  target_days INTEGER DEFAULT NULL,
  is_archived INTEGER DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS habit_logs (
  id TEXT PRIMARY KEY,
  habit_id TEXT NOT NULL REFERENCES habits(id) ON DELETE CASCADE,
  checked_at TEXT NOT NULL,
  checked_time TEXT DEFAULT NULL,
  note TEXT DEFAULT NULL,
  UNIQUE(habit_id, checked_at)
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
  #[serde(default)]
  tag_field_name: String,
  #[serde(default)]
  priority_field_name: String,
  #[serde(default)]
  priority_urgent_value: String,
  #[serde(default)]
  priority_important_value: String,
  #[serde(default)]
  priority_normal_value: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct ShortcutConfig {
  #[serde(default)]
  toggle_window: String,
  #[serde(default)]
  toggle_mode: String,
  #[serde(default)]
  quick_capture: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemConfig {
  #[serde(default = "default_true")]
  menu_bar_enabled: bool,
  #[serde(default = "default_true")]
  close_to_menu_bar: bool,
  #[serde(default)]
  hide_dock_icon: bool,
  #[serde(default = "default_quick_capture_shortcut")]
  quick_capture_shortcut: String,
  #[serde(default)]
  quick_capture_notify: bool,
  #[serde(default)]
  auto_backup: bool,
  #[serde(default = "default_backup_retention_days")]
  backup_retention_days: i64,
}

impl Default for SystemConfig {
  fn default() -> Self {
    Self {
      menu_bar_enabled: default_true(),
      close_to_menu_bar: default_true(),
      hide_dock_icon: false,
      quick_capture_shortcut: default_quick_capture_shortcut(),
      quick_capture_notify: false,
      auto_backup: false,
      backup_retention_days: default_backup_retention_days(),
    }
  }
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
  #[serde(default)]
  system: SystemConfig,
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
  tag_field_name: String,
  priority_field_name: String,
  priority_urgent_value: String,
  priority_important_value: String,
  priority_normal_value: String,
}

#[derive(Debug, Serialize, Clone)]
struct FeishuFieldPayload {
  field_name: String,
  field_type: i64,
  options: Vec<String>,
}

#[derive(Debug, Default)]
struct ConfigIoLock;

#[derive(Debug, Default)]
struct GlobalShortcutState {
  toggle_window: Option<tauri_plugin_global_shortcut::Shortcut>,
  toggle_window_text: String,
  toggle_mode: Option<tauri_plugin_global_shortcut::Shortcut>,
  toggle_mode_text: String,
  quick_capture: Option<tauri_plugin_global_shortcut::Shortcut>,
  quick_capture_text: String,
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
  sub_tasks: String,
  tags: String,
  due_date: String,
  recurrence_rule: String,
  recurrence_parent_id: String,
  recurrence_index: Option<i64>,
  reminder_before: Option<i64>,
  reminder_notified: bool,
  source: String,
  feishu_record_id: String,
  sync_status: String,
  last_synced_at: String,
  retry_count: i64,
  last_error: String,
  last_retry_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Habit {
  id: String,
  name: String,
  emoji: String,
  color: String,
  frequency_type: String,
  frequency_days: String,
  remind_time: String,
  target_days: Option<i64>,
  is_archived: bool,
  created_at: String,
  updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct HabitLog {
  id: String,
  habit_id: String,
  checked_at: String,
  checked_time: String,
  note: String,
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
struct SystemSettingsPayload {
  menu_bar_enabled: bool,
  close_to_menu_bar: bool,
  hide_dock_icon: bool,
  quick_capture_shortcut: String,
  quick_capture_notify: bool,
  auto_backup: bool,
  backup_retention_days: i64,
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
  has_more: Option<bool>,
  page_token: Option<String>,
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

fn default_true() -> bool {
  true
}

fn default_backup_retention_days() -> i64 {
  7
}

fn default_quick_capture_shortcut() -> String {
  DEFAULT_QUICK_CAPTURE_SHORTCUT.to_string()
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
  if task.sub_tasks.trim().is_empty() {
    task.sub_tasks = "[]".to_string();
  }
  if task.tags.trim().is_empty() {
    task.tags = "[]".to_string();
  }
  if task.recurrence_rule.trim().is_empty() {
    task.recurrence_rule = String::new();
  }
  if task.recurrence_parent_id.trim().is_empty() {
    task.recurrence_parent_id = String::new();
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

fn legacy_feishu_priority_value(priority: &str) -> String {
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

fn configured_priority_field_name(config: &FeishuConfig) -> &str {
  let value = config.priority_field_name.trim();
  if value.is_empty() { "优先级" } else { value }
}

fn configured_tag_field_name(config: &FeishuConfig) -> &str {
  let value = config.tag_field_name.trim();
  if value.is_empty() { "标签" } else { value }
}

fn configured_priority_option<'a>(config: &'a FeishuConfig, priority: &str) -> &'a str {
  let value = priority.trim();
  let local_priority = if !config.priority_urgent_value.trim().is_empty()
    && value == config.priority_urgent_value.trim()
  {
    "紧急"
  } else if !config.priority_important_value.trim().is_empty()
    && value == config.priority_important_value.trim()
  {
    "重要"
  } else if !config.priority_normal_value.trim().is_empty()
    && value == config.priority_normal_value.trim()
  {
    "普通"
  } else {
    match value {
      "紧急" | "今日必做" | "🔴今日必做" | "🔴 今日必做" => "紧急",
      "重要" | "本周完成" | "🟡本周完成" | "🟠本周完成" | "🔵本周完成" | "🟡尽快完成" | "🟡重要不紧急" => "重要",
      _ => "普通",
    }
  };
  match local_priority {
    "紧急" => config.priority_urgent_value.trim(),
    "重要" => config.priority_important_value.trim(),
    _ => config.priority_normal_value.trim(),
  }
}

fn to_feishu_priority_value(config: &FeishuConfig, priority: &str) -> String {
  let configured = configured_priority_option(config, priority);
  if configured.is_empty() {
    legacy_feishu_priority_value(priority)
  } else {
    configured.to_string()
  }
}

fn from_feishu_priority_value(config: &FeishuConfig, priority: &str) -> String {
  let value = priority.trim();
  if !config.priority_urgent_value.trim().is_empty() && value == config.priority_urgent_value.trim() {
    return "紧急".to_string();
  }
  if !config.priority_important_value.trim().is_empty() && value == config.priority_important_value.trim() {
    return "重要".to_string();
  }
  if !config.priority_normal_value.trim().is_empty() && value == config.priority_normal_value.trim() {
    return "普通".to_string();
  }

  match legacy_feishu_priority_value(value).as_str() {
    "今日必做" => "紧急".to_string(),
    "本周完成" => "重要".to_string(),
    "自由安排" => "普通".to_string(),
    _ if value.is_empty() => "普通".to_string(),
    _ => value.to_string(),
  }
}

fn normalize_tag_values(raw: &str) -> Vec<String> {
  let parsed = serde_json::from_str::<Vec<String>>(raw).ok();
  let values = parsed.unwrap_or_else(|| {
    raw
      .split(|c: char| c == ',' || c == '，' || c.is_whitespace())
      .map(|item| item.to_string())
      .collect()
  });
  let mut seen = Vec::<String>::new();
  for item in values {
    let tag = item.trim().trim_start_matches('#').chars().take(16).collect::<String>();
    if tag.is_empty() {
      continue;
    }
    if seen.iter().any(|existing| existing.eq_ignore_ascii_case(&tag)) {
      continue;
    }
    seen.push(tag);
    if seen.len() >= 5 {
      break;
    }
  }
  seen
}

fn idempotency_token(seed: &str) -> String {
  let digest = Sha256::digest(seed.as_bytes());
  let mut bytes = [0u8; 16];
  bytes.copy_from_slice(&digest[..16]);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  format!(
    "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
    bytes[0],
    bytes[1],
    bytes[2],
    bytes[3],
    bytes[4],
    bytes[5],
    bytes[6],
    bytes[7],
    bytes[8],
    bytes[9],
    bytes[10],
    bytes[11],
    bytes[12],
    bytes[13],
    bytes[14],
    bytes[15]
  )
}

fn feishu_tags_value_to_local_json(value: Option<&Value>) -> String {
  let tags = match value {
    Some(Value::Array(values)) => values
      .iter()
      .map(value_to_display_string)
      .filter(|item| !item.trim().is_empty())
      .collect::<Vec<_>>(),
    Some(value) => normalize_tag_values(&value_to_display_string(value)),
    None => Vec::new(),
  };
  serde_json::to_string(&normalize_tag_values(
    &serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()),
  ))
  .unwrap_or_else(|_| "[]".to_string())
}

fn local_tags_to_feishu_value(config: &FeishuConfig, raw: &str) -> Value {
  let tags = normalize_tag_values(raw);
  if config.tag_field_name.trim().is_empty() {
    Value::String(tags.join(", "))
  } else {
    Value::Array(tags.into_iter().map(Value::String).collect())
  }
}

fn task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
  fn text(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<String> {
    Ok(row.get::<_, Option<String>>(index)?.unwrap_or_default())
  }

  fn text_or(row: &rusqlite::Row<'_>, index: usize, default: &str) -> rusqlite::Result<String> {
    Ok(row
      .get::<_, Option<String>>(index)?
      .filter(|value| !value.trim().is_empty())
      .unwrap_or_else(|| default.to_string()))
  }

  Ok(Task {
    record_id: text(row, 0)?,
    id: text(row, 1)?,
    name: text(row, 2)?,
    status: text_or(row, 3, "待处理")?,
    priority: text_or(row, 4, "普通")?,
    task_type: text_or(row, 5, "日常事务")?,
    time_spent: text(row, 6)?,
    created_at: text(row, 7)?,
    updated_at: text(row, 8)?,
    completed_at: text(row, 9)?,
    notes: text(row, 10)?,
    sort_order: row.get::<_, Option<i64>>(11)?.unwrap_or(0),
    sub_tasks: text_or(row, 12, "[]")?,
    tags: text_or(row, 13, "[]")?,
    due_date: text(row, 14)?,
    recurrence_rule: text(row, 15)?,
    recurrence_parent_id: text(row, 16)?,
    recurrence_index: row.get(17)?,
    reminder_before: row.get(18)?,
    reminder_notified: row.get::<_, Option<i64>>(19)?.unwrap_or(0) != 0,
    source: text_or(row, 20, "local")?,
    feishu_record_id: text(row, 21)?,
    sync_status: text_or(row, 22, "synced")?,
    last_synced_at: text(row, 23)?,
    retry_count: row.get::<_, Option<i64>>(24)?.unwrap_or(0),
    last_error: text(row, 25)?,
    last_retry_at: text(row, 26)?,
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
  conn
    .execute_batch(CREATE_HABITS_TABLE_SQL)
    .map_err(|err| format!("init habits db failed: {err}"))?;
  ensure_column(&conn, "tasks", "id", "TEXT DEFAULT ''")?;
  ensure_column(&conn, "tasks", "completed_at", "TEXT DEFAULT ''")?;
  ensure_column(&conn, "tasks", "sort_order", "INTEGER DEFAULT 0")?;
  ensure_column(&conn, "tasks", "sub_tasks", "TEXT DEFAULT '[]'")?;
  ensure_column(&conn, "tasks", "tags", "TEXT DEFAULT '[]'")?;
  ensure_column(&conn, "tasks", "due_date", "TEXT DEFAULT ''")?;
  ensure_column(&conn, "tasks", "recurrence_rule", "TEXT DEFAULT ''")?;
  ensure_column(&conn, "tasks", "recurrence_parent_id", "TEXT DEFAULT ''")?;
  ensure_column(&conn, "tasks", "recurrence_index", "INTEGER DEFAULT NULL")?;
  ensure_column(&conn, "tasks", "reminder_before", "INTEGER DEFAULT NULL")?;
  ensure_column(&conn, "tasks", "reminder_notified", "INTEGER DEFAULT 0")?;
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
  conn
    .execute(
      "UPDATE tasks SET sub_tasks = '[]' WHERE sub_tasks IS NULL OR sub_tasks = ''",
      [],
    )
    .map_err(|err| format!("backfill sub_tasks failed: {err}"))?;
  conn
    .execute(
      "UPDATE tasks SET tags = '[]' WHERE tags IS NULL OR tags = ''",
      [],
    )
    .map_err(|err| format!("backfill tags failed: {err}"))?;
  conn
    .execute(
      "UPDATE tasks SET due_date = '' WHERE due_date IS NULL",
      [],
    )
    .map_err(|err| format!("backfill due_date failed: {err}"))?;
  conn
    .execute(
      "UPDATE tasks SET recurrence_rule = '' WHERE recurrence_rule IS NULL",
      [],
    )
    .map_err(|err| format!("backfill recurrence_rule failed: {err}"))?;
  conn
    .execute(
      "UPDATE tasks SET recurrence_parent_id = '' WHERE recurrence_parent_id IS NULL",
      [],
    )
    .map_err(|err| format!("backfill recurrence_parent_id failed: {err}"))?;
  conn
    .execute(
      "UPDATE tasks SET sync_status = 'synced' WHERE sync_status IS NULL OR sync_status = ''",
      [],
    )
    .map_err(|err| format!("backfill sync_status failed: {err}"))?;
  Ok(conn)
}

fn upsert_task(conn: &Connection, task: &Task) -> Result<(), String> {
  let task = normalize_task(task.clone());
  conn
    .execute(
      "INSERT INTO tasks (
        record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, sub_tasks, tags, due_date, recurrence_rule, recurrence_parent_id, recurrence_index, reminder_before, reminder_notified, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)
      ON CONFLICT(record_id) DO UPDATE SET
        id=excluded.id,
        name=CASE WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.name ELSE excluded.name END,
        status=CASE WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.status ELSE excluded.status END,
        priority=CASE WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.priority ELSE excluded.priority END,
        task_type=CASE WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.task_type ELSE excluded.task_type END,
        time_spent=CASE WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.time_spent ELSE excluded.time_spent END,
        created_at=excluded.created_at,
        updated_at=excluded.updated_at,
        completed_at=CASE
          WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.completed_at
          WHEN excluded.status NOT LIKE '%已完成%' THEN excluded.completed_at
          WHEN excluded.completed_at != '' THEN excluded.completed_at
          ELSE COALESCE(tasks.completed_at, '')
        END,
        notes=CASE WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.notes ELSE excluded.notes END,
        sort_order=excluded.sort_order,
        sub_tasks=CASE WHEN excluded.source = 'feishu' AND excluded.sub_tasks = '[]' THEN tasks.sub_tasks ELSE excluded.sub_tasks END,
        tags=CASE WHEN excluded.source = 'feishu' AND excluded.tags = '[]' THEN tasks.tags ELSE excluded.tags END,
        due_date=CASE WHEN excluded.source = 'feishu' AND excluded.due_date = '' THEN tasks.due_date ELSE excluded.due_date END,
        recurrence_rule=CASE WHEN excluded.source = 'feishu' AND excluded.recurrence_rule = '' THEN tasks.recurrence_rule ELSE excluded.recurrence_rule END,
        recurrence_parent_id=CASE WHEN excluded.source = 'feishu' AND excluded.recurrence_parent_id = '' THEN tasks.recurrence_parent_id ELSE excluded.recurrence_parent_id END,
        recurrence_index=CASE WHEN excluded.source = 'feishu' AND excluded.recurrence_index IS NULL THEN tasks.recurrence_index ELSE excluded.recurrence_index END,
        reminder_before=CASE WHEN excluded.source = 'feishu' AND excluded.reminder_before IS NULL THEN tasks.reminder_before ELSE excluded.reminder_before END,
        reminder_notified=CASE WHEN excluded.source = 'feishu' AND excluded.reminder_before IS NULL THEN tasks.reminder_notified ELSE excluded.reminder_notified END,
        source=excluded.source,
        feishu_record_id=excluded.feishu_record_id,
        sync_status=CASE WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.sync_status ELSE excluded.sync_status END,
        last_synced_at=excluded.last_synced_at,
        retry_count=CASE WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.retry_count ELSE excluded.retry_count END,
        last_error=CASE WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.last_error ELSE excluded.last_error END,
        last_retry_at=CASE WHEN tasks.source = 'feishu' AND tasks.sync_status IN ('pending', 'failed') AND excluded.source = 'feishu' THEN tasks.last_retry_at ELSE excluded.last_retry_at END",
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
        task.sub_tasks,
        task.tags,
        task.due_date,
        task.recurrence_rule,
        task.recurrence_parent_id,
        task.recurrence_index,
        task.reminder_before,
        if task.reminder_notified { 1 } else { 0 },
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
  window.set_min_size(Some(Size::Logical(LogicalSize::new(
    NORMAL_MIN_WIDTH,
    NORMAL_MIN_HEIGHT,
  ))))?;
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

fn toggle_quick_capture_window(app: &AppHandle) -> tauri::Result<()> {
  if let Some(window) = app.get_webview_window(QUICK_CAPTURE_WINDOW_LABEL) {
    if window.is_visible().unwrap_or(false) {
      window.hide()?;
    } else {
      window.center()?;
      window.show()?;
      window.set_focus()?;
      let _ = window.emit("quick-capture-focus", ());
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

fn export_dir(app: &AppHandle) -> Result<PathBuf, String> {
  Ok(config_dir(app)?.join("exports"))
}

fn backup_dir(app: &AppHandle) -> Result<PathBuf, String> {
  Ok(config_dir(app)?.join("backups"))
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
  if cfg.feishu.priority_field_name.trim().is_empty() {
    cfg.feishu.priority_field_name = "优先级".to_string();
  }
  if cfg.feishu.priority_urgent_value.trim().is_empty() {
    cfg.feishu.priority_urgent_value = "今日必做".to_string();
  }
  if cfg.feishu.priority_important_value.trim().is_empty() {
    cfg.feishu.priority_important_value = "本周完成".to_string();
  }
  if cfg.feishu.priority_normal_value.trim().is_empty() {
    cfg.feishu.priority_normal_value = "自由安排".to_string();
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
  if cfg.shortcut.quick_capture.trim().is_empty() {
    cfg.shortcut.quick_capture = cfg.system.quick_capture_shortcut.trim().to_string();
  }
  if cfg.shortcut.quick_capture.trim().is_empty() {
    cfg.shortcut.quick_capture = DEFAULT_QUICK_CAPTURE_SHORTCUT.to_string();
  }
  if cfg.system.quick_capture_shortcut.trim().is_empty() {
    cfg.system.quick_capture_shortcut = cfg.shortcut.quick_capture.clone();
  }
  if cfg.system.backup_retention_days <= 0 {
    cfg.system.backup_retention_days = default_backup_retention_days();
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
      quick_capture: DEFAULT_QUICK_CAPTURE_SHORTCUT.to_string(),
    },
    pet: PetConfig::default(),
    system: SystemConfig::default(),
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
          tag_field_name: String::new(),
          priority_field_name: "优先级".to_string(),
          priority_urgent_value: "今日必做".to_string(),
          priority_important_value: "本周完成".to_string(),
          priority_normal_value: "自由安排".to_string(),
        },
        shortcut: ShortcutConfig {
          toggle_window: DEFAULT_TOGGLE_SHORTCUT.to_string(),
          toggle_mode: DEFAULT_TOGGLE_MODE_SHORTCUT.to_string(),
          quick_capture: DEFAULT_QUICK_CAPTURE_SHORTCUT.to_string(),
        },
        pet: PetConfig::default(),
        system: SystemConfig::default(),
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
  let mut fetched_all_pages = false;
  'token_attempts: for attempt in 0..2 {
    let token = get_tenant_access_token(app, attempt == 1).await?;
    let mut page_token: Option<String> = None;
    records.clear();

    loop {
      let mut query = vec![("page_size", "100".to_string())];
      if let Some(token) = page_token.as_ref().filter(|value| !value.trim().is_empty()) {
        query.push(("page_token", token.clone()));
      }
      if filter_completed {
        query.push(("filter", r#"CurrentValue.[状态] != "已完成""#.to_string()));
      }

      let response = client
        .get(&endpoint)
        .query(&query)
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

      if !status.is_success() {
        if let Ok(value) = serde_json::from_str::<Value>(&body) {
          if let Some(code) = value.get("code").and_then(Value::as_i64) {
            if is_token_invalid_code(code as i32) && attempt == 0 {
              continue 'token_attempts;
            }
          }
        }
        return Err(build_feishu_http_error(status, &body));
      }

      let parsed: FeishuRecordsResponse =
        serde_json::from_str(&body).map_err(|err| format!("invalid response: {err}"))?;
      if parsed.code != 0 {
        if is_token_invalid_code(parsed.code) && attempt == 0 {
          continue 'token_attempts;
        }
        return Err(format!(
          "飞书返回错误 code={} {}",
          parsed.code,
          parsed.msg.unwrap_or_default()
        ));
      }

      let data = parsed.data.unwrap_or(FeishuRecordsData {
        items: None,
        records: None,
        has_more: Some(false),
        page_token: None,
      });
      records.extend(data.items.or(data.records).unwrap_or_default());

      if data.has_more.unwrap_or(false) {
        let next_page_token = data.page_token.unwrap_or_default();
        if next_page_token.trim().is_empty() {
          return Err("飞书分页返回缺少 page_token，无法完整同步".to_string());
        }
        page_token = Some(next_page_token);
        continue;
      }

      fetched_all_pages = true;
      break;
    }
    break;
  }

  if !fetched_all_pages {
    return Err("飞书记录未完整拉取，已停止本次同步".to_string());
  }

  let priority_field_name = configured_priority_field_name(&config.feishu).to_string();
  let tag_field_name = configured_tag_field_name(&config.feishu).to_string();

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
      priority: from_feishu_priority_value(
        &config.feishu,
        &field_string(&record.fields, &priority_field_name),
      ),
      task_type: field_string(&record.fields, "类型"),
      time_spent: field_string(&record.fields, "实际耗时(分钟)"),
      created_at: field_string(&record.fields, "任务创建时间"),
      updated_at,
      completed_at: String::new(),
      notes: field_string(&record.fields, "备注/收获"),
      sort_order: 0,
      sub_tasks: "[]".to_string(),
      tags: feishu_tags_value_to_local_json(record.fields.get(&tag_field_name)),
      due_date: String::new(),
      recurrence_rule: String::new(),
      recurrence_parent_id: String::new(),
      recurrence_index: None,
      reminder_before: None,
      reminder_notified: false,
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

  let mut force_refresh = false;
  for _ in 0..3 {
    let token = get_tenant_access_token(app, force_refresh).await?;
    let response = client
      .put(&endpoint)
      .header("Content-Type", "application/json")
      .header("Authorization", format!("Bearer {}", token))
      .json(&json!({ "fields": fields.clone() }))
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
          if is_token_invalid_code(code as i32) {
            force_refresh = true;
            continue;
          }
        }
      }
      return Err(build_feishu_http_error(status, &body));
    }

    let parsed: FeishuGenericResponse =
      serde_json::from_str(&body).map_err(|err| format!("invalid response: {err}"))?;
    if parsed.code != 0 {
      if is_token_invalid_code(parsed.code) {
        force_refresh = true;
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

  Err("重试后仍无法更新记录".to_string())
}

fn has_feishu_field(field_names: &[String], field_name: &str) -> bool {
  field_names.iter().any(|name| name == field_name)
}

fn insert_feishu_field_if_exists(
  fields: &mut Value,
  field_names: &[String],
  field_name: &str,
  value: Value,
) {
  if !has_feishu_field(field_names, field_name) {
    return;
  }
  if let Some(map) = fields.as_object_mut() {
    map.insert(field_name.to_string(), value);
  }
}

fn parse_feishu_field_options(item: &Value) -> Vec<String> {
  item
    .get("property")
    .and_then(|property| property.get("options"))
    .and_then(Value::as_array)
    .map(|options| {
      options
        .iter()
        .filter_map(|option| {
          option
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| option.as_str())
        })
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
    })
    .unwrap_or_default()
}

async fn feishu_list_fields(app: &AppHandle) -> Result<Vec<FeishuFieldPayload>, String> {
  let config = load_app_config_from_file(app)?;
  let endpoint = format!(
    "https://open.feishu.cn/open-apis/bitable/v1/apps/{}/tables/{}/fields",
    config.feishu.app_token.trim(),
    config.feishu.table_id.trim()
  );
  let client = reqwest::Client::builder()
    .use_rustls_tls()
    .build()
    .map_err(|err| format!("http client init failed: {err}"))?;

  'token_attempts: for attempt in 0..2 {
    let token = get_tenant_access_token(app, attempt == 1).await?;
    let mut fields = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
      let mut query = vec![("page_size", "100".to_string())];
      if let Some(value) = page_token.as_ref().filter(|value| !value.trim().is_empty()) {
        query.push(("page_token", value.clone()));
      }
      let response = client
        .get(&endpoint)
        .query(&query)
        .header("Authorization", format!("Bearer {}", token))
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
              continue 'token_attempts;
            }
          }
        }
        return Err(build_feishu_http_error(status, &body));
      }

      let parsed: Value =
        serde_json::from_str(&body).map_err(|err| format!("invalid response: {err}"))?;
      let code = parsed.get("code").and_then(Value::as_i64).unwrap_or(-1);
      if code != 0 {
        if is_token_invalid_code(code as i32) && attempt == 0 {
          continue 'token_attempts;
        }
        return Err(format!(
          "飞书返回错误 code={} {}",
          code,
          parsed.get("msg").map(value_to_display_string).unwrap_or_default()
        ));
      }

      let data = parsed.get("data").cloned().unwrap_or(Value::Null);
      let items = data
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
      fields.extend(items.iter().filter_map(|item| {
        let field_name = item.get("field_name")?.as_str()?.trim().to_string();
        if field_name.is_empty() {
          return None;
        }
        Some(FeishuFieldPayload {
          field_name,
          field_type: item.get("type").and_then(Value::as_i64).unwrap_or_default(),
          options: parse_feishu_field_options(item),
        })
      }));

      if !data.get("has_more").and_then(Value::as_bool).unwrap_or(false) {
        return Ok(fields);
      }
      let next_page_token = data
        .get("page_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
      if next_page_token.is_empty() {
        return Err("飞书字段分页返回缺少 page_token".to_string());
      }
      page_token = Some(next_page_token);
    }
  }

  Err("刷新 token 后仍无法读取字段列表".to_string())
}

async fn feishu_list_field_names(app: &AppHandle) -> Result<Vec<String>, String> {
  Ok(feishu_list_fields(app)
    .await?
    .into_iter()
    .map(|field| field.field_name)
    .collect())
}

async fn feishu_create_text_field(app: &AppHandle, field_name: &str) -> Result<(), String> {
  let config = load_app_config_from_file(app)?;
  let endpoint = format!(
    "https://open.feishu.cn/open-apis/bitable/v1/apps/{}/tables/{}/fields",
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
      .json(&json!({ "field_name": field_name, "type": 1 }))
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

    let parsed: Value =
      serde_json::from_str(&body).map_err(|err| format!("invalid response: {err}"))?;
    let code = parsed.get("code").and_then(Value::as_i64).unwrap_or(-1);
    if code != 0 {
      if is_token_invalid_code(code as i32) && attempt == 0 {
        continue;
      }
      return Err(format!(
        "飞书返回错误 code={} {}",
        code,
        parsed.get("msg").map(value_to_display_string).unwrap_or_default()
      ));
    }
    return Ok(());
  }

  Err("刷新 token 后仍无法创建字段".to_string())
}

async fn feishu_prepare_write_field_names(app: &AppHandle, ensure_tags: bool) -> Result<Vec<String>, String> {
  let config = load_app_config_from_file(app)?;
  let mut field_names = feishu_list_field_names(app).await?;
  let tag_field_name = configured_tag_field_name(&config.feishu).to_string();

  if ensure_tags && !has_feishu_field(&field_names, &tag_field_name) {
    if config.feishu.tag_field_name.trim().is_empty() {
      feishu_create_text_field(app, &tag_field_name).await?;
      field_names.push(tag_field_name);
    } else {
      return Err(format!("已绑定的标签字段不存在：{}", tag_field_name));
    }
  }

  Ok(field_names)
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

  let should_sync_tags = !normalize_tag_values(&task.tags).is_empty();
  let field_names = match feishu_prepare_write_field_names(app, should_sync_tags).await {
    Ok(names) => names,
    Err(err) if should_sync_tags => return Err(err),
    Err(err) => {
      eprintln!("[Rust] list feishu fields failed, use core fields only: {}", err);
      Vec::new()
    }
  };
  let priority_field_name = configured_priority_field_name(&config.feishu).to_string();
  let tag_field_name = configured_tag_field_name(&config.feishu).to_string();
  let mut fields = json!({
    "任务名称": task.name,
    "状态": task.status,
  });
  if let Some(map) = fields.as_object_mut() {
    map.insert(
      priority_field_name,
      Value::String(to_feishu_priority_value(&config.feishu, &task.priority)),
    );
  }
  insert_feishu_field_if_exists(&mut fields, &field_names, "类型", Value::String(task.task_type.clone()));
  if !task.notes.trim().is_empty() {
    insert_feishu_field_if_exists(&mut fields, &field_names, "备注/收获", Value::String(task.notes.clone()));
  }
  let tags = normalize_tag_values(&task.tags);
  if !tags.is_empty() {
    insert_feishu_field_if_exists(
      &mut fields,
      &field_names,
      &tag_field_name,
      local_tags_to_feishu_value(&config.feishu, &task.tags),
    );
  }

  let mut force_refresh = false;
  for _ in 0..3 {
    let token = get_tenant_access_token(app, force_refresh).await?;
    let response = client
      .post(&endpoint)
      .query(&[("client_token", idempotency_token(&task.record_id))])
      .header("Content-Type", "application/json")
      .header("Authorization", format!("Bearer {}", token))
      .json(&json!({ "fields": fields.clone() }))
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
          if is_token_invalid_code(code as i32) {
            force_refresh = true;
            continue;
          }
        }
      }
      return Err(build_feishu_http_error(status, &text));
    }

    let parsed: FeishuGenericResponse =
      serde_json::from_str(&text).map_err(|err| format!("invalid response: {err}"))?;

    if parsed.code != 0 {
      if is_token_invalid_code(parsed.code) {
        force_refresh = true;
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

  Err("重试后仍无法创建记录".to_string())
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
        "SELECT record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, sub_tasks, tags, due_date, recurrence_rule, recurrence_parent_id, recurrence_index, reminder_before, reminder_notified, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
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
        "SELECT record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, sub_tasks, tags, due_date, recurrence_rule, recurrence_parent_id, recurrence_index, reminder_before, reminder_notified, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
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
    let mut conn = open_db(&db_path)?;

    if remote_ids.is_empty() {
      conn
        .execute(
          "DELETE FROM tasks WHERE source = 'feishu' AND sync_status NOT IN ('pending', 'failed')",
          [],
        )
        .map_err(|err| format!("prune stale feishu tasks failed: {err}"))?;
      return Ok::<(), String>(());
    }

    conn
      .execute(
        "CREATE TEMP TABLE IF NOT EXISTS remote_feishu_ids (record_id TEXT PRIMARY KEY)",
        [],
      )
      .map_err(|err| format!("prepare stale prune failed: {err}"))?;
    conn
      .execute("DELETE FROM remote_feishu_ids", [])
      .map_err(|err| format!("prepare stale prune failed: {err}"))?;

    {
      let tx = conn
        .transaction()
        .map_err(|err| format!("prepare stale prune failed: {err}"))?;
      {
        let mut stmt = tx
          .prepare("INSERT OR IGNORE INTO remote_feishu_ids (record_id) VALUES (?1)")
          .map_err(|err| format!("prepare stale prune failed: {err}"))?;
        for id in remote_ids.iter().filter(|id| !id.trim().is_empty()) {
          stmt
            .execute(params![id])
            .map_err(|err| format!("prepare stale prune failed: {err}"))?;
        }
      }
      tx
        .commit()
        .map_err(|err| format!("prepare stale prune failed: {err}"))?;
    }

    conn
      .execute(
        "DELETE FROM tasks
       WHERE source = 'feishu'
         AND sync_status NOT IN ('pending', 'failed')
         AND NOT EXISTS (
           SELECT 1 FROM remote_feishu_ids remote
           WHERE remote.record_id = tasks.record_id
         )",
        [],
      )
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
      "标签" => "tags",
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
        "SELECT record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, sub_tasks, tags, due_date, recurrence_rule, recurrence_parent_id, recurrence_index, reminder_before, reminder_notified, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
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
    || text.contains("fieldnamenotfound")
    || text.contains("code=1254043")
    || text.contains("code=1254045")
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
    let existing: Option<String> = conn
      .query_row(
        "SELECT record_id FROM tasks WHERE record_id = ?1",
        params![new_id.clone()],
        |row| row.get(0),
      )
      .optional()
      .map_err(|err| format!("query replacement record failed: {err}"))?;

    if existing.is_some() {
      conn
        .execute(
          "UPDATE tasks
           SET sync_status = 'synced',
               last_synced_at = ?1,
               retry_count = 0,
               last_error = '',
               last_retry_at = ''
           WHERE record_id = ?2",
          params![now_iso(), new_id],
        )
        .map_err(|err| format!("mark existing replacement synced failed: {err}"))?;
      conn
        .execute("DELETE FROM tasks WHERE record_id = ?1", params![old_id])
        .map_err(|err| format!("delete old temp record failed: {err}"))?;
      return Ok::<(), String>(());
    }

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
        "SELECT record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, sub_tasks, tags, due_date, recurrence_rule, recurrence_parent_id, recurrence_index, reminder_before, reminder_notified, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
         FROM tasks WHERE source = 'local' ORDER BY sort_order DESC, updated_at DESC",
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
    sub_tasks: "[]".to_string(),
    tags: "[]".to_string(),
    due_date: String::new(),
    recurrence_rule: String::new(),
    recurrence_parent_id: String::new(),
    recurrence_index: None,
    reminder_before: None,
    reminder_notified: false,
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
        "SELECT record_id, id, name, status, priority, task_type, time_spent, created_at, updated_at, completed_at, notes, sort_order, sub_tasks, tags, due_date, recurrence_rule, recurrence_parent_id, recurrence_index, reminder_before, reminder_notified, source, feishu_record_id, sync_status, last_synced_at, retry_count, last_error, last_retry_at
         FROM tasks WHERE id = ?1 OR record_id = ?1",
        params![id.clone()],
        task_from_row,
      )
      .map_err(|err| format!("任务不存在: {err}"))?;
    let original_sync_status = task.sync_status.clone();
    let mut sync_affecting = false;

    if let Some(v) = fields.get("name") {
      if !v.trim().is_empty() {
        task.name = v.trim().to_string();
        sync_affecting = true;
      }
    }
    if let Some(v) = fields.get("status") {
      task.status = v.trim().to_string();
      sync_affecting = true;
      if task.status == "已完成" {
        task.completed_at = now_iso();
      } else {
        task.completed_at = String::new();
      }
    }
    if let Some(v) = fields.get("priority") {
      task.priority = v.trim().to_string();
      sync_affecting = true;
    }
    if let Some(v) = fields.get("task_type") {
      task.task_type = v.trim().to_string();
      sync_affecting = true;
    }
    if let Some(v) = fields.get("notes") {
      task.notes = v.to_string();
      sync_affecting = true;
    }
    if let Some(v) = fields.get("sort_order") {
      task.sort_order = v.parse::<i64>().unwrap_or(task.sort_order);
    }
    if let Some(v) = fields.get("sub_tasks") {
      task.sub_tasks = if v.trim().is_empty() {
        "[]".to_string()
      } else {
        v.to_string()
      };
    }
    if let Some(v) = fields.get("tags") {
      task.tags = if v.trim().is_empty() {
        "[]".to_string()
      } else {
        v.to_string()
      };
      sync_affecting = true;
    }
    if let Some(v) = fields.get("due_date") {
      task.due_date = v.trim().to_string();
    }
    if let Some(v) = fields.get("recurrence_rule") {
      task.recurrence_rule = v.trim().to_string();
    }
    if let Some(v) = fields.get("recurrence_parent_id") {
      task.recurrence_parent_id = v.trim().to_string();
    }
    if let Some(v) = fields.get("recurrence_index") {
      task.recurrence_index = v.trim().parse::<i64>().ok();
    }
    if let Some(v) = fields.get("reminder_before") {
      task.reminder_before = if v.trim().is_empty() {
        None
      } else {
        v.trim().parse::<i64>().ok()
      };
    }
    if let Some(v) = fields.get("reminder_notified") {
      task.reminder_notified = matches!(v.trim(), "1" | "true" | "yes");
    }
    task.updated_at = now_iso();
    task.sync_status = if task.source == "feishu" && !sync_affecting {
      original_sync_status
    } else if task.source == "feishu" {
      "pending".to_string()
    } else {
      "synced".to_string()
    };

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

fn habit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Habit> {
  Ok(Habit {
    id: row.get(0)?,
    name: row.get(1)?,
    emoji: row.get(2)?,
    color: row.get(3)?,
    frequency_type: row.get(4)?,
    frequency_days: row.get(5)?,
    remind_time: row.get(6)?,
    target_days: row.get(7)?,
    is_archived: row.get::<_, i64>(8)? != 0,
    created_at: row.get(9)?,
    updated_at: row.get(10)?,
  })
}

fn habit_log_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HabitLog> {
  Ok(HabitLog {
    id: row.get(0)?,
    habit_id: row.get(1)?,
    checked_at: row.get(2)?,
    checked_time: row.get(3)?,
    note: row.get(4)?,
  })
}

#[tauri::command]
async fn get_habits(app: AppHandle) -> Result<Vec<Habit>, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let mut stmt = conn
      .prepare("SELECT id, name, emoji, color, frequency_type, COALESCE(frequency_days, ''), COALESCE(remind_time, ''), target_days, is_archived, created_at, updated_at FROM habits ORDER BY created_at DESC")
      .map_err(|err| format!("prepare habits failed: {err}"))?;
    let rows = stmt.query_map([], habit_from_row).map_err(|err| format!("query habits failed: {err}"))?;
    let mut habits = Vec::new();
    for row in rows {
      habits.push(row.map_err(|err| format!("decode habit failed: {err}"))?);
    }
    Ok::<Vec<Habit>, String>(habits)
  })
  .await
  .map_err(|err| format!("db habit join failed: {err}"))?
}

#[tauri::command]
async fn get_habit_logs(app: AppHandle) -> Result<Vec<HabitLog>, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let mut stmt = conn
      .prepare("SELECT id, habit_id, checked_at, COALESCE(checked_time, ''), COALESCE(note, '') FROM habit_logs ORDER BY checked_at DESC")
      .map_err(|err| format!("prepare habit logs failed: {err}"))?;
    let rows = stmt.query_map([], habit_log_from_row).map_err(|err| format!("query habit logs failed: {err}"))?;
    let mut logs = Vec::new();
    for row in rows {
      logs.push(row.map_err(|err| format!("decode habit log failed: {err}"))?);
    }
    Ok::<Vec<HabitLog>, String>(logs)
  })
  .await
  .map_err(|err| format!("db habit log join failed: {err}"))?
}

#[tauri::command]
async fn create_habit(app: AppHandle, fields: HashMap<String, String>) -> Result<Habit, String> {
  let name = fields.get("name").map(|v| v.trim()).unwrap_or("");
  if name.is_empty() {
    return Err("习惯名称不能为空".to_string());
  }
  let id = uuid::Uuid::new_v4().to_string();
  let now = now_iso();
  let habit = Habit {
    id: id.clone(),
    name: name.to_string(),
    emoji: fields.get("emoji").cloned().unwrap_or_else(|| "🎯".to_string()),
    color: fields.get("color").cloned().unwrap_or_else(|| "#10b981".to_string()),
    frequency_type: fields.get("frequency_type").cloned().unwrap_or_else(|| "daily".to_string()),
    frequency_days: fields.get("frequency_days").cloned().unwrap_or_default(),
    remind_time: fields.get("remind_time").cloned().unwrap_or_default(),
    target_days: fields.get("target_days").and_then(|v| v.trim().parse::<i64>().ok()),
    is_archived: false,
    created_at: now.clone(),
    updated_at: now,
  };
  let db_path = db_file_path(&app)?;
  let save = habit.clone();
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    conn.execute(
      "INSERT INTO habits (id, name, emoji, color, frequency_type, frequency_days, remind_time, target_days, is_archived, created_at, updated_at)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
      params![save.id, save.name, save.emoji, save.color, save.frequency_type, save.frequency_days, save.remind_time, save.target_days, 0, save.created_at, save.updated_at],
    ).map_err(|err| format!("insert habit failed: {err}"))?;
    Ok::<(), String>(())
  }).await.map_err(|err| format!("db habit join failed: {err}"))??;
  Ok(habit)
}

#[tauri::command]
async fn update_habit(app: AppHandle, id: String, fields: HashMap<String, String>) -> Result<Habit, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let mut habit: Habit = conn.query_row(
      "SELECT id, name, emoji, color, frequency_type, COALESCE(frequency_days, ''), COALESCE(remind_time, ''), target_days, is_archived, created_at, updated_at FROM habits WHERE id = ?1",
      params![id.clone()],
      habit_from_row,
    ).map_err(|err| format!("习惯不存在: {err}"))?;

    if let Some(v) = fields.get("name") {
      if !v.trim().is_empty() { habit.name = v.trim().to_string(); }
    }
    if let Some(v) = fields.get("emoji") { habit.emoji = v.to_string(); }
    if let Some(v) = fields.get("color") { habit.color = v.to_string(); }
    if let Some(v) = fields.get("frequency_type") { habit.frequency_type = v.to_string(); }
    if let Some(v) = fields.get("frequency_days") { habit.frequency_days = v.to_string(); }
    if let Some(v) = fields.get("remind_time") { habit.remind_time = v.to_string(); }
    if let Some(v) = fields.get("target_days") { habit.target_days = v.trim().parse::<i64>().ok(); }
    if let Some(v) = fields.get("is_archived") { habit.is_archived = matches!(v.trim(), "1" | "true" | "yes"); }
    habit.updated_at = now_iso();

    conn.execute(
      "UPDATE habits SET name=?1, emoji=?2, color=?3, frequency_type=?4, frequency_days=?5, remind_time=?6, target_days=?7, is_archived=?8, updated_at=?9 WHERE id=?10",
      params![habit.name, habit.emoji, habit.color, habit.frequency_type, habit.frequency_days, habit.remind_time, habit.target_days, if habit.is_archived { 1 } else { 0 }, habit.updated_at, habit.id],
    ).map_err(|err| format!("update habit failed: {err}"))?;
    Ok::<Habit, String>(habit)
  }).await.map_err(|err| format!("db habit join failed: {err}"))?
}

#[tauri::command]
async fn delete_habit(app: AppHandle, id: String) -> Result<bool, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    conn.execute("DELETE FROM habit_logs WHERE habit_id = ?1", params![id.clone()]).map_err(|err| format!("delete habit logs failed: {err}"))?;
    let affected = conn.execute("DELETE FROM habits WHERE id = ?1", params![id]).map_err(|err| format!("delete habit failed: {err}"))?;
    Ok::<bool, String>(affected > 0)
  }).await.map_err(|err| format!("db habit join failed: {err}"))?
}

#[tauri::command]
async fn check_in_habit(app: AppHandle, habit_id: String, checked_at: String, checked_time: String, note: String) -> Result<HabitLog, String> {
  let log = HabitLog {
    id: uuid::Uuid::new_v4().to_string(),
    habit_id,
    checked_at,
    checked_time,
    note,
  };
  let db_path = db_file_path(&app)?;
  let save = log.clone();
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    conn.execute(
      "INSERT OR IGNORE INTO habit_logs (id, habit_id, checked_at, checked_time, note) VALUES (?1, ?2, ?3, ?4, ?5)",
      params![save.id, save.habit_id, save.checked_at, save.checked_time, save.note],
    ).map_err(|err| format!("insert habit log failed: {err}"))?;
    Ok::<(), String>(())
  }).await.map_err(|err| format!("db habit log join failed: {err}"))??;
  Ok(log)
}

#[tauri::command]
async fn uncheck_in_habit(app: AppHandle, habit_id: String, checked_at: String) -> Result<bool, String> {
  let db_path = db_file_path(&app)?;
  tokio::task::spawn_blocking(move || {
    let conn = open_db(&db_path)?;
    let affected = conn.execute("DELETE FROM habit_logs WHERE habit_id = ?1 AND checked_at = ?2", params![habit_id, checked_at]).map_err(|err| format!("delete habit log failed: {err}"))?;
    Ok::<bool, String>(affected > 0)
  }).await.map_err(|err| format!("db habit log join failed: {err}"))?
}

fn init_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
  let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
  let mini_item = MenuItem::with_id(app, "mini", "迷你模式", true, None::<&str>)?;
  let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
  let menu = Menu::with_items(app, &[&show_item, &mini_item, &quit_item])?;

  let app_for_menu = app.clone();
  let app_for_tray = app.clone();

  let tray_builder = TrayIconBuilder::new()
    .menu(&menu)
    .show_menu_on_left_click(false)
    .title("Topdo")
    .tooltip("Topdo")
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
        if let Some(window) = app_for_tray.get_webview_window(MAIN_WINDOW_LABEL) {
          let _ = window.unminimize();
          let _ = window.show();
          let _ = window.set_focus();
        }
      }
    });

  let _ = tray_builder.build(app)?;
  Ok(())
}

fn create_system_windows(app: &mut tauri::App) -> tauri::Result<()> {
  if app.get_webview_window(QUICK_CAPTURE_WINDOW_LABEL).is_none() {
    WebviewWindowBuilder::new(
      app,
      QUICK_CAPTURE_WINDOW_LABEL,
      WebviewUrl::App("index.html?window=capture".into()),
    )
    .title("")
    .inner_size(560.0, 56.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .visible(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .center()
    .build()?;
  }
  Ok(())
}

fn shortcut_conflict_label(shortcut: &tauri_plugin_global_shortcut::Shortcut) -> Option<&'static str> {
    const RESERVED: [(&str, &str); 11] = [
    ("Cmd+N", "新建任务"),
    ("Cmd+,", "打开设置"),
    ("Cmd+K", "快捷键面板"),
    ("Cmd+F", "搜索任务"),
    ("Cmd+J", "任务 / 习惯切换"),
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

fn register_quick_capture_shortcut(
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
    guard.quick_capture.clone()
  };

  register_shortcut_with_rollback(&manager, old, shortcut.clone())?;

  let mut guard = state
    .lock()
    .map_err(|_| "failed to lock shortcut state".to_string())?;
  guard.quick_capture = Some(shortcut);
  guard.quick_capture_text = label;
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
              return;
            }
            if state
              .quick_capture
              .as_ref()
              .map(|shortcut| shortcut.id() == shortcut_id)
              .unwrap_or(false)
            {
              let _ = toggle_quick_capture_window(&app_for_handler);
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

  let raw_quick_capture_shortcut = cfg.shortcut.quick_capture.trim();
  let quick_capture_shortcut_text = if raw_quick_capture_shortcut.is_empty() {
    DEFAULT_QUICK_CAPTURE_SHORTCUT
  } else {
    raw_quick_capture_shortcut
  };
  match parse_shortcut_input(quick_capture_shortcut_text) {
    Ok((shortcut, label)) => {
      if let Err(err) = register_quick_capture_shortcut(app, shortcut, label.clone()) {
        eprintln!("[Rust] register configured quick capture shortcut failed: {}", err);
      }
    }
    Err(err) => {
      eprintln!("[Rust] parse configured quick capture shortcut failed: {}", err);
      if let Ok((fallback_shortcut, fallback_label)) = parse_shortcut_input(DEFAULT_QUICK_CAPTURE_SHORTCUT) {
        let _ = register_quick_capture_shortcut(app, fallback_shortcut, fallback_label);
      }
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
fn show_main_window(app: AppHandle) -> Result<(), String> {
  let window = get_main_window(&app)?;
  set_window_mode_internal(&app, "panel")?;
  window.unminimize().map_err(|err| err.to_string())?;
  window.show().map_err(|err| err.to_string())?;
  window.set_focus().map_err(|err| err.to_string())?;
  Ok(())
}

#[tauri::command]
fn show_quick_capture(app: AppHandle) -> Result<(), String> {
  toggle_quick_capture_window(&app).map_err(|err| err.to_string())
}

#[tauri::command]
fn export_data_file(app: AppHandle, format: String, content: String) -> Result<String, String> {
  let normalized = match format.trim() {
    "json" => "json",
    "csv" => "csv",
    "markdown" | "md" => "md",
    _ => return Err("导出格式不支持".to_string()),
  };
  let dir = export_dir(&app)?;
  fs::create_dir_all(&dir).map_err(|err| format!("create export dir failed: {err}"))?;
  let filename = format!("topdo-export-{}.{}", now_unix_seconds(), normalized);
  let path = dir.join(filename);
  fs::write(&path, content).map_err(|err| format!("write export file failed: {err}"))?;
  Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn export_daily_receipt_image(app: AppHandle, bytes: Vec<u8>) -> Result<String, String> {
  const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
  if bytes.len() < PNG_SIGNATURE.len() || &bytes[..PNG_SIGNATURE.len()] != PNG_SIGNATURE {
    return Err("生成的小票不是有效 PNG 图片".to_string());
  }
  if bytes.len() > 10 * 1024 * 1024 {
    return Err("小票图片过大，无法保存".to_string());
  }
  let dir = export_dir(&app)?;
  fs::create_dir_all(&dir).map_err(|err| format!("create export dir failed: {err}"))?;
  let path = dir.join(format!("Topdo-今日小票-{}.png", chrono_like_date()));
  fs::write(&path, bytes).map_err(|err| format!("write receipt image failed: {err}"))?;
  Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn run_backup(app: AppHandle, content: String, retention_days: Option<i64>) -> Result<String, String> {
  let dir = backup_dir(&app)?;
  fs::create_dir_all(&dir).map_err(|err| format!("create backup dir failed: {err}"))?;
  let today = chrono_like_date();
  let path = dir.join(format!("topdo-backup-{today}.json"));
  if !path.exists() {
    fs::write(&path, content).map_err(|err| format!("write backup failed: {err}"))?;
  }
  let keep = retention_days.unwrap_or(7).max(1) as usize;
  let mut backups: Vec<_> = fs::read_dir(&dir)
    .map_err(|err| format!("read backup dir failed: {err}"))?
    .filter_map(Result::ok)
    .filter(|entry| entry.file_name().to_string_lossy().starts_with("topdo-backup-"))
    .collect();
  backups.sort_by_key(|entry| entry.file_name());
  if backups.len() > keep {
    let remove_count = backups.len().saturating_sub(keep);
    for entry in backups.into_iter().take(remove_count) {
      let _ = fs::remove_file(entry.path());
    }
  }
  Ok(path.to_string_lossy().to_string())
}

fn chrono_like_date() -> String {
  // Avoid adding a date crate: use local system date command on macOS and fall back to unix day.
  if let Ok(output) = std::process::Command::new("date").arg("+%Y-%m-%d").output() {
    if output.status.success() {
      let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
      if !text.is_empty() {
        return text;
      }
    }
  }
  let days = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_secs()
    / 86_400;
  format!("day-{days}")
}

#[tauri::command]
fn open_backup_folder(app: AppHandle) -> Result<(), String> {
  let dir = backup_dir(&app)?;
  fs::create_dir_all(&dir).map_err(|err| format!("create backup dir failed: {err}"))?;
  #[cfg(target_os = "macos")]
  {
    std::process::Command::new("open")
      .arg(&dir)
      .spawn()
      .map_err(|err| format!("open backup folder failed: {err}"))?;
  }
  #[cfg(not(target_os = "macos"))]
  {
    let _ = dir;
  }
  Ok(())
}

#[tauri::command]
fn open_export_folder(app: AppHandle) -> Result<(), String> {
  let dir = export_dir(&app)?;
  fs::create_dir_all(&dir).map_err(|err| format!("create export dir failed: {err}"))?;
  #[cfg(target_os = "macos")]
  {
    std::process::Command::new("open")
      .arg(&dir)
      .spawn()
      .map_err(|err| format!("open export folder failed: {err}"))?;
  }
  #[cfg(not(target_os = "macos"))]
  {
    let _ = dir;
  }
  Ok(())
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
  let priority_field_name = configured_priority_field_name(&cfg.feishu).to_string();
  let priority_urgent_value = to_feishu_priority_value(&cfg.feishu, "紧急");
  let priority_important_value = to_feishu_priority_value(&cfg.feishu, "重要");
  let priority_normal_value = to_feishu_priority_value(&cfg.feishu, "普通");

  Ok(ConfigPayload {
    mode,
    app_id: cfg.feishu.app_id,
    app_token: cfg.feishu.app_token,
    table_id: cfg.feishu.table_id,
    folder_token: cfg.feishu.folder_token,
    collaborator_email: cfg.feishu.collaborator_email,
    has_secret: !cfg.feishu.encrypted_app_secret.trim().is_empty(),
    tag_field_name: cfg.feishu.tag_field_name,
    priority_field_name,
    priority_urgent_value,
    priority_important_value,
    priority_normal_value,
  })
}

#[tauri::command]
async fn get_feishu_fields(app: AppHandle) -> Result<Vec<FeishuFieldPayload>, String> {
  feishu_list_fields(&app).await
}

#[tauri::command]
async fn save_feishu_field_mapping(
  app: AppHandle,
  tag_field_name: String,
  priority_field_name: String,
  priority_urgent_value: String,
  priority_important_value: String,
  priority_normal_value: String,
) -> Result<(), String> {
  let tag_field_name = tag_field_name.trim().to_string();
  let priority_field_name = priority_field_name.trim().to_string();
  let priority_values = [
    priority_urgent_value.trim().to_string(),
    priority_important_value.trim().to_string(),
    priority_normal_value.trim().to_string(),
  ];

  if priority_field_name.is_empty() {
    return Err("请选择飞书优先级单选字段".to_string());
  }
  if priority_values.iter().any(|value| value.is_empty()) {
    return Err("请为紧急、重要、普通分别选择飞书枚举值".to_string());
  }
  if priority_values[0] == priority_values[1]
    || priority_values[0] == priority_values[2]
    || priority_values[1] == priority_values[2]
  {
    return Err("三个优先级需要绑定不同的飞书枚举值".to_string());
  }

  let fields = feishu_list_fields(&app).await?;
  if !tag_field_name.is_empty() {
    let tag_field = fields
      .iter()
      .find(|field| field.field_name == tag_field_name)
      .ok_or_else(|| format!("未找到标签字段：{}", tag_field_name))?;
    if tag_field.field_type != 4 {
      return Err("标签只能绑定飞书多选字段".to_string());
    }
  }

  let priority_field = fields
    .iter()
    .find(|field| field.field_name == priority_field_name)
    .ok_or_else(|| format!("未找到优先级字段：{}", priority_field_name))?;
  if priority_field.field_type != 3 {
    return Err("优先级只能绑定飞书单选字段".to_string());
  }
  for value in &priority_values {
    if !priority_field.options.iter().any(|option| option == value) {
      return Err(format!(
        "枚举值“{}”不在字段“{}”中，请先在飞书中创建该选项",
        value, priority_field_name
      ));
    }
  }

  let mut cfg = load_app_config_from_file(&app)?;
  cfg.feishu.tag_field_name = tag_field_name;
  cfg.feishu.priority_field_name = priority_field_name;
  cfg.feishu.priority_urgent_value = priority_values[0].clone();
  cfg.feishu.priority_important_value = priority_values[1].clone();
  cfg.feishu.priority_normal_value = priority_values[2].clone();
  save_app_config_to_file(&app, &cfg)
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
fn get_system_settings(app: AppHandle) -> Result<SystemSettingsPayload, String> {
  let cfg = load_app_config_from_file(&app)?;
  Ok(SystemSettingsPayload {
    menu_bar_enabled: cfg.system.menu_bar_enabled,
    close_to_menu_bar: cfg.system.close_to_menu_bar,
    hide_dock_icon: cfg.system.hide_dock_icon,
    quick_capture_shortcut: cfg.shortcut.quick_capture,
    quick_capture_notify: cfg.system.quick_capture_notify,
    auto_backup: cfg.system.auto_backup,
    backup_retention_days: cfg.system.backup_retention_days,
  })
}

#[tauri::command]
fn save_system_settings(
  app: AppHandle,
  menu_bar_enabled: bool,
  close_to_menu_bar: bool,
  hide_dock_icon: bool,
  quick_capture_shortcut: String,
  quick_capture_notify: bool,
  auto_backup: bool,
  backup_retention_days: i64,
) -> Result<SystemSettingsPayload, String> {
  let previous = {
    let state = app.state::<Mutex<GlobalShortcutState>>();
    let guard = state
      .lock()
      .map_err(|_| "failed to lock shortcut state".to_string())?;
    (guard.quick_capture.clone(), guard.quick_capture_text.clone())
  };
  let (shortcut, label) = parse_shortcut_input(&quick_capture_shortcut)?;
  register_quick_capture_shortcut(&app, shortcut, label.clone())?;

  let mut cfg = load_app_config_from_file(&app)?;
  cfg.system.menu_bar_enabled = menu_bar_enabled;
  cfg.system.close_to_menu_bar = close_to_menu_bar;
  cfg.system.hide_dock_icon = hide_dock_icon;
  cfg.system.quick_capture_shortcut = label.clone();
  cfg.system.quick_capture_notify = quick_capture_notify;
  cfg.system.auto_backup = auto_backup;
  cfg.system.backup_retention_days = if backup_retention_days <= 0 { 7 } else { backup_retention_days };
  cfg.shortcut.quick_capture = label.clone();

  if let Err(err) = save_app_config_to_file(&app, &cfg) {
    if let Some(prev_shortcut) = previous.0 {
      let rollback_label = if previous.1.trim().is_empty() {
        format_shortcut(&prev_shortcut)
      } else {
        previous.1
      };
      let _ = register_quick_capture_shortcut(&app, prev_shortcut, rollback_label);
    }
    return Err(err);
  }

  get_system_settings(app)
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
        params![(ids.len() - idx) as i64, id],
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

  let config = load_app_config_from_file(&app)?;
  let (remote_field_name, remote_value) = match field_name.trim() {
    "优先级" => (
      configured_priority_field_name(&config.feishu).to_string(),
      Value::String(to_feishu_priority_value(&config.feishu, &value)),
    ),
    "标签" => (
      configured_tag_field_name(&config.feishu).to_string(),
      local_tags_to_feishu_value(&config.feishu, &value),
    ),
    _ => (field_name.trim().to_string(), Value::String(value.clone())),
  };
  let mut fields = serde_json::Map::new();
  fields.insert(remote_field_name, remote_value);
  let fields = Value::Object(fields);
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
      Ok(UpdateTaskResult {
        success: false,
        message: if retryable {
          format!("离线缓存，待重试：{err}")
        } else {
          format!("同步失败，请检查飞书配置或字段：{err}")
        },
      })
    }
  }
}

#[tauri::command]
async fn sync_task_tags(
  app: AppHandle,
  record_id: String,
  tags: String,
) -> Result<UpdateTaskResult, String> {
  let rid = record_id.trim();
  if rid.is_empty() {
    return Err("record_id 不能为空".to_string());
  }
  if rid.starts_with("temp-") {
    return Ok(UpdateTaskResult {
      success: false,
      message: "任务尚未同步到飞书，稍后会自动重试".to_string(),
    });
  }

  let config = load_app_config_from_file(&app)?;
  let tag_field_name = configured_tag_field_name(&config.feishu).to_string();
  let field_names = match feishu_prepare_write_field_names(&app, true).await {
    Ok(names) => names,
    Err(err) => {
      return Ok(UpdateTaskResult {
        success: false,
        message: format!("标签同步失败：{err}"),
      })
    }
  };
  if !has_feishu_field(&field_names, &tag_field_name) {
    return Ok(UpdateTaskResult {
      success: false,
      message: format!("未能创建或找到飞书标签字段：{}", tag_field_name),
    });
  }

  let mut fields = serde_json::Map::new();
  fields.insert(
    tag_field_name,
    local_tags_to_feishu_value(&config.feishu, &tags),
  );
  let fields = Value::Object(fields);
  match feishu_update_record_fields(&app, rid, fields).await {
    Ok(_) => Ok(UpdateTaskResult {
      success: true,
      message: "标签同步成功".to_string(),
    }),
    Err(err) => Ok(UpdateTaskResult {
      success: false,
      message: format!("标签同步失败：{err}"),
    }),
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
    sub_tasks: "[]".to_string(),
    tags: "[]".to_string(),
    due_date: String::new(),
    recurrence_rule: String::new(),
    recurrence_parent_id: String::new(),
    recurrence_index: None,
    reminder_before: None,
    reminder_notified: false,
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

    let config = load_app_config_from_file(&app)?;
    let priority_field_name = configured_priority_field_name(&config.feishu).to_string();
    let tag_field_name = configured_tag_field_name(&config.feishu).to_string();
    let should_sync_tags = !normalize_tag_values(&task.tags).is_empty()
      || !config.feishu.tag_field_name.trim().is_empty();
    let field_names = match feishu_prepare_write_field_names(&app, should_sync_tags).await {
      Ok(names) => names,
      Err(err) if should_sync_tags => {
        let retryable = !is_non_retryable_sync_error(&err);
        let _ = db_mark_push_result(app.clone(), task.record_id.clone(), err, retryable).await;
        continue;
      }
      Err(err) => {
        eprintln!("[Rust] list feishu fields failed during sync, use core fields only: {}", err);
        Vec::new()
      }
    };

    let mut fields = json!({
      "任务名称": task.name,
      "状态": task.status,
    });
    if let Some(map) = fields.as_object_mut() {
      map.insert(
        priority_field_name,
        Value::String(to_feishu_priority_value(&config.feishu, &task.priority)),
      );
    }
    insert_feishu_field_if_exists(&mut fields, &field_names, "类型", Value::String(task.task_type.clone()));
    if !task.notes.trim().is_empty() {
      insert_feishu_field_if_exists(&mut fields, &field_names, "备注/收获", Value::String(task.notes.clone()));
    }
    if has_feishu_field(&field_names, &tag_field_name) {
      insert_feishu_field_if_exists(
        &mut fields,
        &field_names,
        &tag_field_name,
        local_tags_to_feishu_value(&config.feishu, &task.tags),
      );
    }

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
    .plugin(tauri_plugin_notification::init())
    .manage(Mutex::new(UiState {
      mini_mode: false,
      always_on_top: true,
    }))
    .manage(Mutex::new(ConfigIoLock))
    .manage(Mutex::new(GlobalShortcutState::default()))
    .manage(tokio::sync::RwLock::new(TokenManager::default()))
    .manage(tokio::sync::Mutex::new(()))
    .setup(|app| {
      create_system_windows(app)?;
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
    .on_window_event(|window, event| {
      if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == MAIN_WINDOW_LABEL {
          let should_hide = load_app_config_from_file(window.app_handle())
            .map(|cfg| cfg.system.close_to_menu_bar)
            .unwrap_or(true);
          if should_hide {
            api.prevent_close();
            let _ = window.hide();
          }
        }
      }
    })
    .invoke_handler(tauri::generate_handler![
      check_feishu_api_client,
      get_window_state,
      reapply_window_traits,
      toggle_always_on_top,
      enter_mini_mode,
      restore_normal_mode,
      hide_window_to_tray,
      show_main_window,
      show_quick_capture,
      export_data_file,
      export_daily_receipt_image,
      run_backup,
      open_backup_folder,
      open_export_folder,
      save_config,
      load_config,
      get_feishu_fields,
      save_feishu_field_mapping,
      get_shortcut_config,
      set_shortcut_config,
      get_mode_shortcut_config,
      set_mode_shortcut_config,
      get_system_settings,
      save_system_settings,
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
      sync_task_tags,
      create_task,
      delete_task,
      sync_tasks,
      get_habits,
      get_habit_logs,
      create_habit,
      update_habit,
      delete_habit,
      check_in_habit,
      uncheck_in_habit
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
  use super::*;

  fn mapped_feishu_config() -> FeishuConfig {
    FeishuConfig {
      tag_field_name: "业务标签".to_string(),
      priority_field_name: "任务等级".to_string(),
      priority_urgent_value: "P0".to_string(),
      priority_important_value: "P1".to_string(),
      priority_normal_value: "P2".to_string(),
      ..FeishuConfig::default()
    }
  }

  #[test]
  fn priority_mapping_is_bidirectional() {
    let config = mapped_feishu_config();
    assert_eq!(to_feishu_priority_value(&config, "紧急"), "P0");
    assert_eq!(to_feishu_priority_value(&config, "重要"), "P1");
    assert_eq!(to_feishu_priority_value(&config, "普通"), "P2");
    assert_eq!(to_feishu_priority_value(&config, "P0"), "P0");
    assert_eq!(from_feishu_priority_value(&config, "P0"), "紧急");
    assert_eq!(from_feishu_priority_value(&config, "P1"), "重要");
    assert_eq!(from_feishu_priority_value(&config, "P2"), "普通");
  }

  #[test]
  fn bound_tags_use_multi_select_array() {
    let config = mapped_feishu_config();
    assert_eq!(
      local_tags_to_feishu_value(&config, r#"["需求","oncall"]"#),
      json!(["需求", "oncall"])
    );
    assert_eq!(
      feishu_tags_value_to_local_json(Some(&json!(["需求", "oncall"]))),
      r#"["需求","oncall"]"#
    );
  }

  #[test]
  fn legacy_tags_remain_text() {
    let config = FeishuConfig::default();
    assert_eq!(
      local_tags_to_feishu_value(&config, r#"["需求","oncall"]"#),
      Value::String("需求, oncall".to_string())
    );
  }

  #[test]
  fn field_options_are_read_from_feishu_property() {
    let field = json!({
      "field_name": "任务等级",
      "type": 3,
      "property": { "options": [{ "name": "P0" }, { "name": "P1" }] }
    });
    assert_eq!(parse_feishu_field_options(&field), vec!["P0", "P1"]);
  }
}
