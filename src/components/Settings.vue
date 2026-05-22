<template>
  <section class="settings-page task-scrollbar">
    <header class="settings-header">
      <div>
        <h2>设置</h2>
      </div>
      <button type="button" class="ghost-btn" @click="$emit('back')">返回任务</button>
    </header>

    <section class="settings-group padded-group">
      <SegmentedControl
        v-model="selectedMode"
        :options="[
          { value: 'local', label: '本地模式', icon: 'cloud-off' },
          { value: 'feishu', label: '飞书同步', icon: 'cloud-sync' }
        ]"
      />
      <div v-if="selectedMode === 'feishu'" class="sync-status" :class="{ error: !feishuConfigured }">
        <span class="sync-dot" />
        <div>
          <p>{{ feishuConfigured ? '飞书同步已配置' : '飞书同步未完成配置' }}</p>
          <span>{{ feishuConfigured ? '可测试连接并保存配置' : '请补充多维表格和应用凭证' }}</span>
        </div>
      </div>
    </section>

    <template v-if="selectedMode === 'feishu'">
      <section class="settings-group">
        <div class="setting-row clickable" @click="bitableExpanded = !bitableExpanded">
          <span class="setting-icon blue"><Icon name="link" :size="18" /></span>
          <div class="setting-text">
            <p class="setting-name">多维表格</p>
          </div>
          <Icon class="setting-arrow" :class="{ open: bitableExpanded }" name="chevron-right" :size="17" />
        </div>
        <Transition name="expand">
          <div v-if="bitableExpanded" class="expand-section">
            <div class="expand-inner">
              <div class="expand-title"><span class="step">1</span> 粘贴并解析多维表格链接</div>
              <label class="form-group">
                <span class="form-label">多维表格链接</span>
                <div class="form-row">
                  <input
                    v-model="form.bitableUrl"
                    type="text"
                    class="form-input"
                    placeholder="https://xxx.feishu.cn/base/..."
                    @blur="parseBitableUrl(true)"
                  />
                  <button type="button" class="btn secondary compact" @click="parseBitableUrl(false)">解析</button>
                </div>
                <span class="form-hint">自动提取 base 后的 App Token 和 table 参数。</span>
              </label>
              <div class="form-row two-col">
                <label class="form-group">
                  <span class="form-label">App Token</span>
                  <input v-model="form.appToken" type="text" class="form-input mono" placeholder="Base Token" />
                </label>
                <label class="form-group">
                  <span class="form-label">Table ID</span>
                  <input v-model="form.tableId" type="text" class="form-input mono" placeholder="tbl..." />
                </label>
              </div>
              <div v-if="stepState.linkParsed" class="parse-result">
                <Icon name="check-circle" :size="15" /> 已识别 App Token 和 Table ID
              </div>
              <div class="btn-row">
                <button type="button" class="btn secondary" @click="onOpenTemplateLink">打开模板</button>
                <button type="button" class="btn ghost" @click="onCancelBitableEdit">取消</button>
                <button type="button" class="btn primary" :disabled="busy" @click="onSaveSection('bitable')">保存并收起</button>
              </div>
            </div>
          </div>
        </Transition>

        <div class="setting-row clickable" @click="credentialExpanded = !credentialExpanded">
          <span class="setting-icon blue"><Icon name="key" :size="18" /></span>
          <div class="setting-text">
            <p class="setting-name">应用凭证</p>
          </div>
          <span v-if="hasSavedSecret" class="encrypted-badge"><Icon name="lock" :size="12" /> 已加密</span>
          <Icon class="setting-arrow" :class="{ open: credentialExpanded }" name="chevron-right" :size="17" />
        </div>
        <Transition name="expand">
          <div v-if="credentialExpanded" class="expand-section">
            <div class="expand-inner">
              <div class="expand-title"><span class="step">2</span> 填写应用凭证</div>
              <label class="form-group">
                <span class="form-label">App ID</span>
                <input v-model="form.appId" type="text" class="form-input mono" placeholder="cli_xxx" />
              </label>
              <label class="form-group">
                <span class="form-label">App Secret</span>
                <input v-model="form.appSecret" type="password" class="form-input mono" placeholder="留空表示保留已保存的 Secret" />
                <span class="form-hint">Secret 保存后由本地配置管理，不在页面明文展示。</span>
              </label>
              <div class="annotation">
                <Icon name="info" :size="15" />
                <span>在飞书开放平台创建企业自建应用，即可获取 App ID 和 App Secret。</span>
                <button type="button" class="text-link" @click="onOpenTutorialLink">打开教程</button>
              </div>
              <div class="btn-row">
                <button type="button" class="btn secondary" :disabled="busy" @click="onTestConnection">测试连接</button>
                <button type="button" class="btn ghost" @click="onCancelCredentialEdit">取消</button>
                <button type="button" class="btn primary" :disabled="busy" @click="onSaveSection('credential')">保存并收起</button>
              </div>
            </div>
          </div>
        </Transition>

        <div class="action-row">
          <button type="button" class="btn secondary" :disabled="busy" @click="onTestConnection">测试连接</button>
          <button type="button" class="btn secondary" @click="onOpenTemplateLink">打开模板</button>
          <button type="button" class="btn secondary" @click="showLogs = !showLogs">{{ showLogs ? '隐藏日志' : '查看日志' }}</button>
        </div>
      </section>
    </template>

    <section class="settings-group">
      <div class="setting-row editable-row">
        <span class="setting-icon blue"><Icon name="keyboard" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">唤起 / 隐藏窗口</p>
        </div>
        <KbdShortcut :value="appliedShortcut" />
      </div>
      <div class="shortcut-editor">
        <input v-model="shortcutDraft" type="text" class="form-input" placeholder="Cmd+Shift+T" />
        <button type="button" class="btn secondary compact" :disabled="busy" @click="onSaveShortcut">保存</button>
      </div>
      <div class="setting-row editable-row">
        <span class="setting-icon blue"><Icon name="cat" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">面板 / 迷你模式切换</p>
        </div>
        <KbdShortcut :value="appliedModeShortcut" />
      </div>
      <div class="shortcut-editor">
        <input v-model="modeShortcutDraft" type="text" class="form-input" placeholder="Alt+T" />
        <button type="button" class="btn secondary compact" :disabled="busy" @click="onSaveModeShortcut">保存</button>
      </div>
    </section>

    <section class="settings-group">
      <div class="setting-row">
        <span class="setting-icon green"><Icon name="habit-mode" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">习惯模块</p>
        </div>
        <label class="toggle"><input v-model="habitModuleEnabledModel" type="checkbox" /><span /></label>
      </div>
      <div class="setting-row">
        <span class="setting-icon orange"><Icon name="bell" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">截止提醒</p>
        </div>
        <label class="toggle"><input v-model="reminderEnabledModel" type="checkbox" /><span /></label>
      </div>
      <div class="setting-row">
        <span class="setting-icon green"><Icon name="cat" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">宠物模式</p>
        </div>
        <label class="toggle"><input v-model="petEnabled" type="checkbox" /><span /></label>
      </div>
      <div v-if="petEnabled" class="nested-options">
        <label><input v-model="petShowBadge" type="checkbox" /> 显示角标（未完成数量）</label>
        <label><input v-model="petAnimations" type="checkbox" /> 启用动画效果</label>
      </div>
    </section>

    <section class="settings-group padded-group">
      <SegmentedControl
        v-model="themePreferenceValue"
        :options="[
          { value: 'system', label: '跟随系统', icon: 'monitor' },
          { value: 'light', label: '浅色', icon: 'sun' },
          { value: 'dark', label: '深色', icon: 'moon' }
        ]"
      />
    </section>

    <section class="settings-group">
      <div class="setting-row">
        <span class="setting-icon orange"><Icon name="rocket" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">开机自启动</p>
        </div>
        <label class="toggle"><input v-model="autostartEnabled" type="checkbox" :disabled="busy || autostartLoading" /><span /></label>
      </div>
      <div class="setting-row">
        <span class="setting-icon gray"><Icon name="update" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">自动检查更新</p>
        </div>
        <label class="toggle"><input v-model="autoUpdateEnabledModel" type="checkbox" /><span /></label>
      </div>
    </section>

    <section class="settings-group">
      <div class="setting-row editable-row">
        <span class="setting-icon orange"><Icon name="keyboard" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">快捷新建任务</p>
        </div>
        <KbdShortcut :value="quickCaptureShortcutDraft" />
      </div>
      <div class="shortcut-editor">
        <input v-model="quickCaptureShortcutDraft" type="text" class="form-input" placeholder="Alt+Space" />
        <button type="button" class="btn secondary compact" :disabled="busy" @click="onSaveSystemSettings">保存</button>
      </div>
    </section>

    <section class="settings-group data-tools-group">
      <div class="setting-row data-tools-header">
        <span class="setting-icon green"><Icon name="download" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">数据导出</p>
        </div>
      </div>
      <div class="data-tools-body">
        <div class="export-actions">
          <button type="button" class="btn secondary compact" :disabled="busy" @click="onExportData('json')">JSON</button>
          <button type="button" class="btn secondary compact" :disabled="busy" @click="onExportData('csv')">CSV</button>
          <button type="button" class="btn secondary compact" :disabled="busy" @click="onExportData('markdown')">Markdown</button>
        </div>
      </div>
      <p v-if="dataActionMessage" class="inline-result" :class="dataActionType">{{ dataActionMessage }}</p>
    </section>

    <section class="settings-group">
      <button type="button" class="setting-row clickable full-row-button" :disabled="busy" @click="onCheckUpdates">
        <span class="setting-icon gray"><Icon name="info" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">Topdo v2.0.1</p>
        </div>
        <Icon name="chevron-right" :size="17" />
      </button>
      <button type="button" class="setting-row clickable full-row-button" @click="onOpenGitHub">
        <span class="setting-icon gray"><Icon name="github" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">GitHub 主页</p>
        </div>
        <Icon name="chevron-right" :size="17" />
      </button>
      <button type="button" class="setting-row clickable full-row-button" @click="onOpenFeedback">
        <span class="setting-icon blue"><Icon name="chat" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">反馈建议</p>
        </div>
        <Icon name="chevron-right" :size="17" />
      </button>
    </section>

    <section class="settings-group support-author-group">
      <button type="button" class="setting-row clickable full-row-button" @click="onOpenSupport">
        <span class="setting-icon orange"><Icon name="star" :size="18" /></span>
        <div class="setting-text">
          <p class="setting-name">支持作者</p>
          <p class="support-copy">核心功能会保持免费；如果 Topdo 帮你减少了任务混乱，可以请作者喝杯咖啡。</p>
        </div>
        <Icon name="chevron-right" :size="17" />
      </button>
      <div class="support-tiers" aria-label="支持作者档位">
        <span>¥6.6 咖啡</span>
        <span>¥19.9 持续维护</span>
        <span>¥49.9 共建者</span>
        <span>¥99 早期赞助者</span>
      </div>
    </section>

    <p v-if="statusMessage" class="status-message" :class="statusType">
      {{ statusMessage }}
    </p>
    <div v-if="statusType === 'error' && statusDetail" class="error-detail">
      <div class="error-actions">
        <button type="button" class="btn secondary compact" @click="onCopyErrorDetail">复制错误详情</button>
        <span v-if="errorDetailCopied">已复制</span>
      </div>
      <pre>{{ statusDetail }}</pre>
    </div>

    <div class="footer-actions">
      <button type="button" class="btn secondary" :disabled="busy || selectedMode !== 'feishu'" @click="onTestConnection">测试连接</button>
      <button type="button" class="btn primary" :disabled="busy" @click="onSave">保存设置</button>
      <button type="button" class="btn ghost" :disabled="busy" @click="$emit('back')">返回</button>
    </div>

    <Transition name="expand">
      <div v-if="showLogs" class="logs-panel">
        <div class="logs-header">
          <span>最近 {{ logs.length }} / 50 条</span>
          <div>
            <button type="button" class="btn ghost compact" @click="onClearLogs">清除</button>
            <button type="button" class="btn ghost compact" @click="onCopyLogs">复制</button>
          </div>
        </div>
        <div class="task-scrollbar logs-list">
          <p v-for="(entry, idx) in logs" :key="`${entry.timestamp}-${entry.tag}-${idx}`">{{ formatLogLine(entry) }}</p>
          <p v-if="logs.length === 0" class="empty-log">暂无日志</p>
        </div>
      </div>
    </Transition>
  </section>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { disable as disableAutostart, enable as enableAutostart, isEnabled as isAutostartEnabled } from '@tauri-apps/plugin-autostart';
import Icon from './Icon.vue';
import KbdShortcut from './KbdShortcut.vue';
import SegmentedControl from './SegmentedControl.vue';
import { useAppStore } from '../stores/appStore';
import { useHabitStore } from '../stores/habitStore';
import { usePetStore } from '../stores/petStore';
import { useTaskStore } from '../stores/taskStore';
import { WindowMode } from '../types/pet';
import { exportDataFile, openExportFolder, type ExportFormat } from '../services/exportService';
import { clearLogs, formatLogLine, logs } from '../utils/logger';
import { setThemePreference, useThemeState, type ThemePreference } from '../utils/theme';

type AppMode = 'local' | 'feishu';
type StatusType = 'success' | 'error';

interface FormState {
  bitableUrl: string;
  appId: string;
  appSecret: string;
  appToken: string;
  tableId: string;
}

interface LoadConfigPayload {
  mode: string;
  app_id: string;
  app_token: string;
  table_id: string;
  folder_token: string;
  collaborator_email: string;
  has_secret: boolean;
}

interface ConnectionResult {
  success: boolean;
  message: string;
}

interface ShortcutConfigPayload {
  toggle_window: string;
}

interface ModeShortcutConfigPayload {
  toggle_mode: string;
}

interface SetShortcutConfigResult {
  success: boolean;
  message: string;
  applied?: string;
}

interface PetSettingsPayload {
  enabled: boolean;
  show_badge: boolean;
  animations: boolean;
  cat_position: { x: number; y: number };
  window_mode: string;
}

interface SystemSettingsPayload {
  menu_bar_enabled: boolean;
  close_to_menu_bar: boolean;
  hide_dock_icon: boolean;
  quick_capture_shortcut: string;
  quick_capture_notify: boolean;
  auto_backup: boolean;
  backup_retention_days: number;
}

const GITHUB_REPO_URL = 'https://github.com/SkyNone/Topdo';
const GITHUB_FEEDBACK_URL = 'https://github.com/SkyNone/Topdo/issues';
const GITHUB_SUPPORT_URL = 'https://github.com/SkyNone/Topdo#支持作者';
const GITHUB_RELEASES_API_URL = 'https://api.github.com/repos/SkyNone/Topdo/releases/latest';
const GITHUB_LATEST_RELEASE_URL = 'https://github.com/SkyNone/Topdo/releases/latest';
const APP_VERSION = '2.0.1';
const FEISHU_TEMPLATE_URL =
  'https://s7wd8lze1s.feishu.cn/base/QR7rbtLf0adg0gsFun7cKnYOnGd?table=tblSeF0WH71ITCe7&view=vewMSNDmR0';
const FEISHU_TUTORIAL_URL = 'https://open.feishu.cn/app';

const emit = defineEmits<{
  (event: 'back'): void;
  (event: 'saved', mode: AppMode): void;
}>();

const petStore = usePetStore();
const appStore = useAppStore();
const taskStore = useTaskStore();
const habitStore = useHabitStore();
const selectedMode = ref<AppMode>('local');
const initialMode = ref<AppMode>('local');
const busy = ref(false);
const showLogs = ref(false);
const statusMessage = ref('');
const statusDetail = ref('');
const statusType = ref<StatusType>('success');
const autostartEnabled = ref(false);
const initialAutostartEnabled = ref(false);
const autostartLoading = ref(false);
const shortcutDraft = ref('');
const appliedShortcut = ref('');
const modeShortcutDraft = ref('');
const appliedModeShortcut = ref('');
const errorDetailCopied = ref(false);
const petEnabled = ref(true);
const petShowBadge = ref(true);
const petAnimations = ref(true);
const petPosition = ref({ x: 0, y: 0 });
const petWindowMode = ref('panel');
const hasSavedSecret = ref(false);
const bitableExpanded = ref(false);
const credentialExpanded = ref(false);
const systemMenuBarEnabled = ref(true);
const systemCloseToMenuBar = ref(true);
const systemHideDockIcon = ref(false);
const quickCaptureShortcutDraft = ref('Alt+Space');
const dataActionMessage = ref('');
const dataActionType = ref<StatusType>('success');

const form = reactive<FormState>({
  bitableUrl: '',
  appId: '',
  appSecret: '',
  appToken: '',
  tableId: ''
});

const stepState = reactive({
  templateReady: false,
  linkParsed: false,
  credentialReady: false,
  tutorialCopied: false
});

const { themePreference } = useThemeState();
const themePreferenceValue = ref<ThemePreference>(themePreference.value);

const feishuConfigured = computed(() => Boolean(form.appToken && form.tableId && form.appId && (form.appSecret || hasSavedSecret.value)));
const habitModuleEnabledModel = computed({
  get: () => appStore.habitModuleEnabled,
  set: (enabled: boolean) => appStore.setHabitModuleEnabled(enabled)
});
const reminderEnabledModel = computed({
  get: () => appStore.reminderEnabled,
  set: (enabled: boolean) => appStore.setReminderEnabled(enabled)
});
const autoUpdateEnabledModel = computed({
  get: () => appStore.autoUpdateEnabled,
  set: (enabled: boolean) => appStore.setAutoUpdateEnabled(enabled)
});

function setStatus(type: StatusType, message: string) {
  statusType.value = type;
  if (type === 'error') {
    const firstLine = message.split('\n').find((line) => line.trim().length > 0) || message;
    statusMessage.value = firstLine.length > 140 ? `${firstLine.slice(0, 140)}...` : firstLine;
    statusDetail.value = message;
  } else {
    statusMessage.value = message;
    statusDetail.value = '';
  }
}

function normalizeVersion(value: string): number[] {
  return value
    .trim()
    .replace(/^v/i, '')
    .split('.')
    .map((item) => Number.parseInt(item, 10))
    .map((item) => (Number.isFinite(item) ? item : 0));
}

function compareVersion(a: string, b: string): number {
  const left = normalizeVersion(a);
  const right = normalizeVersion(b);
  const length = Math.max(left.length, right.length, 3);
  for (let index = 0; index < length; index += 1) {
    const diff = (left[index] || 0) - (right[index] || 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

async function loadShortcutConfig() {
  try {
    const config = await invoke<ShortcutConfigPayload>('get_shortcut_config');
    shortcutDraft.value = config.toggle_window || 'Cmd+Shift+T';
    appliedShortcut.value = shortcutDraft.value;
  } catch (error) {
    setStatus('error', String(error));
  }
}

async function onSaveShortcut() {
  busy.value = true;
  try {
    const result = await invoke<SetShortcutConfigResult>('set_shortcut_config', {
      toggle_window: shortcutDraft.value,
      toggleWindow: shortcutDraft.value
    });
    if (!result.success) {
      throw new Error(result.message || '快捷键保存失败');
    }
    const applied = result.applied || shortcutDraft.value;
    shortcutDraft.value = applied;
    appliedShortcut.value = applied;
    setStatus('success', `快捷键已更新为 ${applied}`);
  } catch (error) {
    setStatus('error', String(error));
  } finally {
    busy.value = false;
  }
}

async function loadModeShortcutConfig() {
  try {
    const config = await invoke<ModeShortcutConfigPayload>('get_mode_shortcut_config');
    modeShortcutDraft.value = config.toggle_mode || 'Alt+T';
    appliedModeShortcut.value = modeShortcutDraft.value;
  } catch (error) {
    setStatus('error', String(error));
  }
}

async function onSaveModeShortcut() {
  busy.value = true;
  try {
    const result = await invoke<SetShortcutConfigResult>('set_mode_shortcut_config', {
      toggle_mode: modeShortcutDraft.value,
      toggleMode: modeShortcutDraft.value,
    });
    if (!result.success) {
      throw new Error(result.message || '形态快捷键保存失败');
    }
    const applied = result.applied || modeShortcutDraft.value;
    modeShortcutDraft.value = applied;
    appliedModeShortcut.value = applied;
    setStatus('success', `形态切换快捷键已更新为 ${applied}`);
  } catch (error) {
    setStatus('error', String(error));
  } finally {
    busy.value = false;
  }
}

async function loadPetSettings() {
  try {
    const payload = await invoke<PetSettingsPayload>('get_pet_settings');
    petEnabled.value = payload.enabled;
    petShowBadge.value = payload.show_badge;
    petAnimations.value = payload.animations;
    petPosition.value = payload.cat_position || { x: 0, y: 0 };
    petWindowMode.value = payload.window_mode || 'panel';
  } catch (error) {
    setStatus('error', String(error));
  }
}

async function loadSystemSettings() {
  try {
    const payload = await invoke<SystemSettingsPayload>('get_system_settings');
    systemMenuBarEnabled.value = payload.menu_bar_enabled;
    systemCloseToMenuBar.value = payload.close_to_menu_bar;
    systemHideDockIcon.value = payload.hide_dock_icon;
    quickCaptureShortcutDraft.value = payload.quick_capture_shortcut || 'Alt+Space';
  } catch (error) {
    setStatus('error', String(error));
  }
}

async function persistSystemSettings(showMessage = false) {
  const payload = await invoke<SystemSettingsPayload>('save_system_settings', {
    menuBarEnabled: systemMenuBarEnabled.value,
    menu_bar_enabled: systemMenuBarEnabled.value,
    closeToMenuBar: systemCloseToMenuBar.value,
    close_to_menu_bar: systemCloseToMenuBar.value,
    hideDockIcon: systemHideDockIcon.value,
    hide_dock_icon: systemHideDockIcon.value,
    quickCaptureShortcut: quickCaptureShortcutDraft.value,
    quick_capture_shortcut: quickCaptureShortcutDraft.value,
    quickCaptureNotify: false,
    quick_capture_notify: false,
    autoBackup: false,
    auto_backup: false,
    backupRetentionDays: 7,
    backup_retention_days: 7
  });
  systemMenuBarEnabled.value = payload.menu_bar_enabled;
  systemCloseToMenuBar.value = payload.close_to_menu_bar;
  systemHideDockIcon.value = payload.hide_dock_icon;
  quickCaptureShortcutDraft.value = payload.quick_capture_shortcut || quickCaptureShortcutDraft.value;
  if (showMessage) setStatus('success', '系统级设置已保存');
}

async function onSaveSystemSettings() {
  busy.value = true;
  try {
    await persistSystemSettings(true);
  } catch (error) {
    setStatus('error', String(error));
  } finally {
    busy.value = false;
  }
}

function buildSaveConfigParams(): Record<string, unknown> {
  return {
    mode: selectedMode.value,
    appId: form.appId,
    app_id: form.appId,
    appSecret: form.appSecret,
    app_secret: form.appSecret,
    appToken: form.appToken,
    app_token: form.appToken,
    tableId: form.tableId,
    table_id: form.tableId,
    syncInterval: 30,
    sync_interval: 30
  };
}

function parseFeishuBitableUrl(urlText: string): { appToken: string; tableId: string; error?: string } {
  const raw = urlText.trim();
  if (!raw) return { appToken: '', tableId: '', error: '链接为空，请先粘贴多维表格链接' };

  let normalized = raw;
  if (!/^https?:\/\//i.test(normalized)) normalized = `https://${normalized}`;

  let parsedUrl: URL;
  try {
    parsedUrl = new URL(normalized);
  } catch {
    return { appToken: '', tableId: '', error: '链接格式不正确，请粘贴完整飞书多维表格链接' };
  }

  const pathSegments = parsedUrl.pathname.split('/').filter(Boolean);
  const tokenPathIndex = pathSegments.findIndex((segment) => segment.toLowerCase() === 'base');
  const hasWikiPath = pathSegments.some((segment) => segment.toLowerCase() === 'wiki');
  if (hasWikiPath && tokenPathIndex < 0) {
    return { appToken: '', tableId: '', error: '当前仅支持 /base/ 链接，请在飞书中复制多维表格 base 链接' };
  }
  const tokenFromPath = tokenPathIndex >= 0 ? pathSegments[tokenPathIndex + 1] ?? '' : '';
  const tokenMatch = normalized.match(/Basc[a-zA-Z0-9]+/);
  const appToken = tokenFromPath || tokenMatch?.[0] || '';
  const tableId = parsedUrl.searchParams.get('table')?.trim() ?? '';

  if (!appToken && !tableId) return { appToken: '', tableId: '', error: '未找到 App Token 和 Table ID，请确认是多维表格链接' };
  if (!appToken) return { appToken: '', tableId: '', error: '未找到 App Token（base 后的标识）' };
  if (!tableId) return { appToken: '', tableId: '', error: '未找到 Table ID（table=...）' };

  return { appToken, tableId };
}

function parseBitableUrl(silentOnBlur: boolean) {
  const result = parseFeishuBitableUrl(form.bitableUrl);
  if (result.error) {
    stepState.linkParsed = false;
    if (!silentOnBlur) setStatus('error', result.error);
    return;
  }

  form.appToken = result.appToken;
  form.tableId = result.tableId;
  stepState.linkParsed = true;
  setStatus('success', '已识别 App Token 和 Table ID');
}

async function openTemplateLink() {
  try {
    await open(FEISHU_TEMPLATE_URL);
    stepState.templateReady = true;
    setStatus('success', '已在浏览器打开 Topdo 模板');
  } catch (error) {
    setStatus('error', `打开模板失败: ${String(error)}`);
  }
}

function onOpenTemplateLink() {
  void openTemplateLink();
}

async function openTutorialLink() {
  try {
    await open(FEISHU_TUTORIAL_URL);
    stepState.tutorialCopied = true;
    setStatus('success', '已在浏览器打开教程');
  } catch (error) {
    stepState.tutorialCopied = false;
    setStatus('error', `打开教程失败: ${String(error)}`);
  }
}

function onOpenTutorialLink() {
  void openTutorialLink();
}

async function loadConfig() {
  try {
    const config = await invoke<LoadConfigPayload>('load_config');
    const mode: AppMode = config.mode === 'feishu' ? 'feishu' : 'local';
    selectedMode.value = mode;
    initialMode.value = mode;
    form.appId = config.app_id;
    form.appToken = config.app_token;
    form.tableId = config.table_id;
    form.bitableUrl = config.app_token && config.table_id ? `https://www.feishu.cn/base/${config.app_token}?table=${config.table_id}` : '';
    form.appSecret = '';
    hasSavedSecret.value = Boolean(config.has_secret);
    stepState.linkParsed = Boolean(config.app_token && config.table_id);
    stepState.credentialReady = Boolean(config.app_id && config.has_secret);

    if (config.has_secret && mode === 'feishu') {
      setStatus('success', '已检测到已保存的 App Secret（加密）');
    }
  } catch (error) {
    setStatus('error', String(error));
  }
}

async function loadAutostartState() {
  autostartLoading.value = true;
  try {
    const enabled = await isAutostartEnabled();
    autostartEnabled.value = enabled;
    initialAutostartEnabled.value = enabled;
  } catch {
    // ignore when unavailable
  } finally {
    autostartLoading.value = false;
  }
}

async function saveSettings(emitSaved = true): Promise<boolean> {
  busy.value = true;
  try {
    const effectiveWindowMode = petEnabled.value ? petWindowMode.value : WindowMode.Panel;

    if (selectedMode.value === 'feishu') {
      await invoke('save_config', buildSaveConfigParams());
      if (form.appSecret) hasSavedSecret.value = true;
      form.appSecret = '';
    } else {
      await invoke('set_app_mode', { mode: 'local' });
    }

    await invoke('save_pet_settings', {
      enabled: petEnabled.value,
      showBadge: petShowBadge.value,
      show_badge: petShowBadge.value,
      animations: petAnimations.value,
      catX: petPosition.value.x,
      cat_x: petPosition.value.x,
      catY: petPosition.value.y,
      cat_y: petPosition.value.y,
      windowMode: effectiveWindowMode,
      window_mode: effectiveWindowMode,
    });
    await petStore.save({
      enabled: petEnabled.value,
      showBadge: petShowBadge.value,
      animations: petAnimations.value,
      catPosition: { ...petPosition.value },
      windowMode: effectiveWindowMode === WindowMode.Cat ? WindowMode.Cat : WindowMode.Panel
    });

    const verifiedMode = await invoke<string>('get_app_mode');
    if (verifiedMode !== selectedMode.value) {
      throw new Error(`模式保存失败，期望 ${selectedMode.value}，实际 ${verifiedMode || '空值'}`);
    }
    initialMode.value = selectedMode.value;

    if (autostartEnabled.value !== initialAutostartEnabled.value) {
      if (autostartEnabled.value) await enableAutostart();
      else await disableAutostart();
      initialAutostartEnabled.value = autostartEnabled.value;
    }
    await persistSystemSettings(false);

    setStatus('success', '设置保存成功');
    if (emitSaved) {
      emit('saved', selectedMode.value);
    }
    return true;
  } catch (error) {
    setStatus('error', String(error));
    return false;
  } finally {
    busy.value = false;
  }
}

async function onSave() {
  await saveSettings(true);
}

async function onSaveSection(section: 'bitable' | 'credential') {
  const saved = await saveSettings(false);
  if (saved) {
    if (section === 'bitable') bitableExpanded.value = false;
    if (section === 'credential') credentialExpanded.value = false;
  }
}

async function onCancelBitableEdit() {
  await loadConfig();
  bitableExpanded.value = false;
}

async function onCancelCredentialEdit() {
  await loadConfig();
  credentialExpanded.value = false;
}

async function onTestConnection() {
  busy.value = true;
  try {
    if (selectedMode.value !== 'feishu') throw new Error('请先切换到飞书同步模式');
    await invoke('save_config', buildSaveConfigParams());
    const result = await invoke<ConnectionResult>('test_connection');
    stepState.credentialReady = result.success;
    setStatus(result.success ? 'success' : 'error', result.message);
  } catch (error) {
    stepState.credentialReady = false;
    setStatus('error', String(error));
  } finally {
    busy.value = false;
  }
}

function onClearLogs() {
  clearLogs();
}

async function onCopyLogs() {
  const text = logs.value.map((entry) => formatLogLine(entry)).join('\n');
  try {
    await navigator.clipboard.writeText(text);
    setStatus('success', '日志已复制到剪贴板');
  } catch (error) {
    setStatus('error', `复制失败: ${String(error)}`);
  }
}

async function onCopyErrorDetail() {
  if (!statusDetail.value) return;
  try {
    await navigator.clipboard.writeText(statusDetail.value);
    errorDetailCopied.value = true;
    setTimeout(() => {
      errorDetailCopied.value = false;
    }, 1500);
  } catch (error) {
    setStatus('error', `复制失败: ${String(error)}`);
  }
}

async function onOpenGitHub() {
  try {
    await open(GITHUB_REPO_URL);
  } catch (error) {
    setStatus('error', `打开 GitHub 失败: ${String(error)}`);
  }
}

async function onOpenFeedback() {
  try {
    await open(GITHUB_FEEDBACK_URL);
  } catch (error) {
    setStatus('error', `打开反馈地址失败: ${String(error)}`);
  }
}

async function onOpenSupport() {
  try {
    await open(GITHUB_SUPPORT_URL);
  } catch (error) {
    setStatus('error', `打开支持作者页面失败: ${String(error)}`);
  }
}

async function onCheckUpdates() {
  busy.value = true;
  setStatus('success', '正在检查更新...');
  try {
    const response = await fetch(GITHUB_RELEASES_API_URL, {
      headers: { Accept: 'application/vnd.github+json' }
    });
    if (!response.ok) {
      throw new Error(`GitHub 返回 ${response.status}`);
    }
    const release = await response.json() as { tag_name?: string; html_url?: string };
    const latest = release.tag_name || '';
    if (!latest) throw new Error('未找到最新版本信息');
    if (compareVersion(latest, APP_VERSION) > 0) {
      setStatus('success', `发现新版本 ${latest}，已打开下载页`);
      if (release.html_url) await open(release.html_url);
    } else {
      setStatus('success', `当前已是最新版本：Topdo v${APP_VERSION.replace(/\.0$/, '')}`);
    }
  } catch (error) {
    try {
      await open(GITHUB_LATEST_RELEASE_URL);
      setStatus('error', `自动检查失败，已打开 Release 页面: ${String(error)}`);
    } catch {
      setStatus('error', `检查更新失败: ${String(error)}`);
    }
  } finally {
    busy.value = false;
  }
}

async function ensureExportDataLoaded() {
  if (!taskStore.tasks.length) await taskStore.fetchTasks();
  if (appStore.habitModuleEnabled && !habitStore.habits.length) await habitStore.fetchHabits();
}

async function onExportData(format: ExportFormat) {
  busy.value = true;
  try {
    await ensureExportDataLoaded();
    const path = await exportDataFile(format, taskStore.tasks, habitStore.habits);
    await openExportFolder();
    dataActionType.value = 'success';
    dataActionMessage.value = `已导出：${path}`;
    setStatus('success', `已导出到 ${path}`);
  } catch (error) {
    dataActionType.value = 'error';
    dataActionMessage.value = `导出失败：${String(error)}`;
    setStatus('error', `导出失败: ${String(error)}`);
  } finally {
    busy.value = false;
  }
}

onMounted(() => {
  appStore.load();
  void loadConfig();
  void loadShortcutConfig();
  void loadModeShortcutConfig();
  void loadPetSettings();
  void loadSystemSettings();
  void loadAutostartState();
});

function handleEsc(): boolean {
  if (showLogs.value) {
    showLogs.value = false;
    return true;
  }
  if (credentialExpanded.value) {
    credentialExpanded.value = false;
    return true;
  }
  if (bitableExpanded.value) {
    bitableExpanded.value = false;
    return true;
  }
  return false;
}

defineExpose({ handleEsc });

watch(
  () => themePreference.value,
  (value) => {
    themePreferenceValue.value = value;
  }
);

watch(
  () => themePreferenceValue.value,
  (value) => {
    setThemePreference(value);
  }
);
</script>

<style scoped>
.settings-page {
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  background: var(--bg-secondary);
  color: var(--text-primary);
  padding: 16px;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.settings-header h2 {
  font-size: 20px;
  line-height: 28px;
  font-weight: 700;
}

.settings-group {
  margin-bottom: 12px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-solid);
}

.padded-group {
  padding: 12px;
}

.setting-row {
  min-height: 54px;
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 12px 16px;
  border: 0;
  border-bottom: 1px solid var(--border-light);
  background: transparent;
  color: inherit;
  text-align: left;
}

.setting-row:last-child {
  border-bottom: 0;
}

.setting-row.clickable {
  cursor: pointer;
}

.setting-row.clickable:hover,
.full-row-button:hover {
  background: var(--bg-hover);
}

.setting-icon {
  width: 32px;
  height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border-radius: 8px;
  color: var(--primary);
  background: color-mix(in srgb, var(--primary) 10%, var(--bg-solid));
}

.setting-icon.green {
  color: var(--accent-green);
  background: var(--accent-green-soft);
}

.setting-icon.orange {
  color: var(--accent-amber);
  background: var(--accent-amber-soft);
}

.setting-icon.gray {
  color: var(--text-secondary);
  background: var(--bg-secondary);
}

.setting-text {
  min-width: 0;
  flex: 1;
}

.setting-name {
  color: var(--text-primary);
  font-size: var(--font-size-base, 13px);
  line-height: 18px;
  font-weight: 500;
}

.setting-arrow {
  color: var(--text-tertiary);
  transition: transform 0.16s ease;
}

.setting-arrow.open {
  transform: rotate(90deg);
}

.sync-status {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, var(--accent-green) 25%, var(--border));
  border-radius: 8px;
  background: color-mix(in srgb, var(--accent-green) 10%, var(--bg-solid));
  color: var(--accent-green);
}

.sync-status.error {
  border-color: color-mix(in srgb, var(--priority-high) 25%, var(--border));
  background: color-mix(in srgb, var(--priority-high) 8%, var(--bg-solid));
  color: var(--priority-high);
}

.sync-dot {
  width: 8px;
  height: 8px;
  flex-shrink: 0;
  border-radius: 999px;
  background: currentColor;
}

.sync-status p {
  font-size: 12px;
  line-height: 16px;
  font-weight: 600;
}

.sync-status span:not(.sync-dot) {
  font-size: 11px;
  line-height: 14px;
}

.form-hint {
  margin-top: 8px;
  color: var(--text-tertiary);
  font-size: 11px;
  line-height: 14px;
}

.expand-enter-active,
.expand-leave-active {
  transition: all 0.16s ease;
  overflow: hidden;
}

.expand-enter-from,
.expand-leave-to {
  max-height: 0;
  opacity: 0;
}

.expand-enter-to,
.expand-leave-from {
  max-height: 520px;
  opacity: 1;
}

.expand-section {
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-secondary);
}

.expand-inner {
  padding: 14px 16px 16px;
}

.expand-title {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  color: var(--text-primary);
  font-size: 12px;
  line-height: 16px;
  font-weight: 600;
}

.step {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: var(--primary);
  color: #fff;
  font-size: 10px;
}

.form-group {
  display: block;
  margin-bottom: 12px;
}

.form-label {
  display: block;
  margin-bottom: 4px;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 16px;
  font-weight: 500;
}

.form-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.form-row.two-col .form-group {
  flex: 1;
  min-width: 0;
}

.form-input {
  width: 100%;
  height: 36px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-solid);
  color: var(--text-primary);
  font-size: 13px;
  line-height: 20px;
  outline: none;
  padding: 0 12px;
  transition: border 0.15s ease, box-shadow 0.15s ease;
}

.form-input:focus {
  border-color: var(--primary);
  box-shadow: var(--shadow-focus);
}

.form-input.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.parse-result,
.annotation,
.inline-note {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 8px 0 0;
  border: 1px solid color-mix(in srgb, var(--accent-green) 25%, var(--border));
  border-radius: 8px;
  background: color-mix(in srgb, var(--accent-green) 8%, var(--bg-solid));
  color: var(--accent-green);
  font-size: 12px;
  line-height: 16px;
  padding: 8px 10px;
}

.annotation {
  align-items: flex-start;
  border-color: color-mix(in srgb, var(--primary) 25%, var(--border));
  background: color-mix(in srgb, var(--primary) 8%, var(--bg-solid));
  color: var(--primary);
}

.inline-note {
  margin: 0 16px 12px;
}

.text-link {
  margin-left: auto;
  border: 0;
  background: transparent;
  color: var(--primary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.btn-row,
.action-row,
.footer-actions {
  display: flex;
  gap: 8px;
}

.btn-row,
.action-row {
  padding-top: 12px;
}

.action-row {
  padding: 10px 16px 14px;
}

.action-row.wrap {
  flex-wrap: wrap;
}

.data-tools-group {
  overflow: visible;
}

.support-author-group .setting-row {
  align-items: flex-start;
}

.support-copy {
  margin-top: 2px;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 17px;
}

.support-tiers {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
  padding: 0 16px 16px 60px;
}

.support-tiers span {
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 16px;
  padding: 5px 9px;
  text-align: center;
}

.data-tools-header {
  min-height: 54px;
  border-bottom: 0;
}

.data-tools-body {
  display: grid;
  gap: 10px;
  padding: 0 16px 16px 60px;
}

.export-actions {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.export-actions .btn {
  width: 100%;
  min-width: 0;
  padding: 0 8px;
}

.shortcut-editor {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  padding: 8px 16px 14px 60px;
  border-bottom: 1px solid var(--border-light);
}

.shortcut-editor .form-input {
  min-width: 0;
}

.inline-result {
  margin: 0 16px 16px;
  border-radius: 8px;
  padding: 8px 10px;
  font-size: 12px;
  line-height: 16px;
  word-break: break-word;
}

.inline-result.success {
  background: color-mix(in srgb, var(--accent-green) 10%, var(--bg-solid));
  color: var(--accent-green);
}

.inline-result.error {
  background: color-mix(in srgb, var(--priority-high) 8%, var(--bg-solid));
  color: var(--priority-high);
}

.btn {
  min-height: 36px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: 1px solid transparent;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  padding: 0 14px;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease, border 0.15s ease, transform 0.1s ease;
}

.btn:active:not(:disabled) {
  transform: scale(0.98);
}

.btn:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.btn.primary {
  background: var(--primary);
  color: #fff;
}

.btn.primary:hover:not(:disabled) {
  background: var(--primary-hover);
}

.btn.secondary,
.ghost-btn {
  border-color: var(--border);
  background: var(--bg-solid);
  color: var(--text-secondary);
}

.btn.secondary:hover:not(:disabled),
.ghost-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.btn.ghost {
  border-color: transparent;
  background: transparent;
  color: var(--text-secondary);
}

.btn.ghost:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.btn.compact {
  min-height: 32px;
  padding: 0 12px;
}

.ghost-btn {
  min-height: 32px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  padding: 0 12px;
  cursor: pointer;
}

.encrypted-badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  border: 1px solid color-mix(in srgb, var(--accent-green) 25%, var(--border));
  border-radius: 4px;
  background: color-mix(in srgb, var(--accent-green) 8%, var(--bg-solid));
  color: var(--accent-green);
  font-size: 10px;
  line-height: 14px;
  padding: 1px 5px;
}

.toggle {
  position: relative;
  flex-shrink: 0;
  width: 42px;
  height: 24px;
  cursor: pointer;
}

.toggle input {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}

.toggle span {
  position: absolute;
  inset: 0;
  border-radius: 999px;
  background: var(--bg-tertiary);
  transition: background 0.15s ease;
}

.toggle span::after {
  content: '';
  position: absolute;
  left: 2px;
  top: 2px;
  width: 20px;
  height: 20px;
  border-radius: 999px;
  background: var(--bg-solid);
  box-shadow: var(--shadow-sm);
  transition: transform 0.15s ease;
}

.toggle input:checked + span {
  background: var(--primary);
}

.toggle input:checked + span::after {
  transform: translateX(18px);
}

.toggle input:disabled + span {
  opacity: 0.55;
}

.nested-options {
  display: grid;
  gap: 8px;
  padding: 10px 16px 14px 60px;
  border-top: 1px solid var(--border-light);
  color: var(--text-secondary);
  font-size: 12px;
}

.nested-options label {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-message {
  margin: 12px 0 0;
  border-radius: 8px;
  padding: 10px 12px;
  font-size: 12px;
  line-height: 16px;
}

.status-message.success {
  background: color-mix(in srgb, var(--accent-green) 10%, var(--bg-solid));
  color: var(--accent-green);
}

.status-message.error {
  background: color-mix(in srgb, var(--priority-high) 8%, var(--bg-solid));
  color: var(--priority-high);
}

.error-detail,
.logs-panel {
  margin-top: 10px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-solid);
  padding: 10px;
}

.error-actions,
.logs-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 8px;
  color: var(--text-tertiary);
  font-size: 11px;
}

.error-detail pre,
.logs-list {
  max-height: 160px;
  overflow-y: auto;
  border: 1px solid var(--border-light);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 11px;
  line-height: 16px;
  padding: 8px;
  white-space: pre-wrap;
  word-break: break-word;
}

.logs-list p {
  border-bottom: 1px solid var(--border-light);
  padding: 4px 0;
}

.logs-list p:last-child {
  border-bottom: 0;
}

.empty-log {
  color: var(--text-tertiary);
}

.footer-actions {
  position: sticky;
  bottom: 0;
  z-index: 1;
  margin-top: 16px;
  padding: 12px 0 0;
  background: linear-gradient(to top, var(--bg-secondary) 78%, transparent);
}

.footer-actions .btn {
  flex: 1;
}
</style>
