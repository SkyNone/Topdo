<template>
  <section class="task-scrollbar flex h-full min-h-0 flex-col overflow-y-auto bg-[color:var(--bg-secondary)] p-4 text-[color:var(--text-primary)]">
    <h2 class="text-lg font-semibold">设置</h2>

    <div class="mt-4 rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
      <h3 class="text-sm font-semibold text-[color:var(--text-primary)]">运行模式</h3>
      <div class="mt-3 space-y-2 text-sm">
        <label class="flex items-start gap-2">
          <input v-model="selectedMode" type="radio" value="local" class="mt-1" />
          <span>本地模式 — 数据存本地</span>
        </label>
        <label class="flex items-start gap-2">
          <input v-model="selectedMode" type="radio" value="feishu" class="mt-1" />
          <span>飞书同步 — 连接多维表格</span>
        </label>
      </div>
      <p class="mt-2 text-[11px] text-[color:var(--text-tertiary)]">⚠ 切换模式不会删除已有数据</p>
    </div>

    <div class="mt-3 rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
      <h3 class="text-sm font-semibold text-[color:var(--text-primary)]">全局唤起快捷键</h3>
      <p class="mt-1 text-[11px] text-[color:var(--text-secondary)]">
        用于显示/隐藏 Topdo 窗口。示例：
        <code class="rounded bg-[color:var(--bg-secondary)] px-1">Cmd+Shift+T</code>、
        <code class="rounded bg-[color:var(--bg-secondary)] px-1">Cmd+Alt+T</code>
      </p>
      <div class="mt-2 flex items-center gap-2">
        <input
          v-model="shortcutDraft"
          type="text"
          class="flex-1 rounded-[8px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-3 py-2 text-sm text-[color:var(--text-primary)] outline-none placeholder:text-[color:var(--text-tertiary)] focus:border-[color:var(--primary)] focus:ring-2 focus:ring-[color:var(--primary-light)]"
          placeholder="Cmd+Shift+T"
        />
        <button type="button" class="action-btn px-3 py-2" :disabled="busy" @click="onSaveShortcut">保存快捷键</button>
      </div>
      <p v-if="appliedShortcut" class="mt-2 text-[11px] text-[color:var(--text-tertiary)]">
        当前生效：<span class="font-medium text-[color:var(--text-secondary)]">{{ appliedShortcut }}</span>
      </p>
    </div>

    <div class="mt-3 rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
      <h3 class="text-sm font-semibold text-[color:var(--text-primary)]">形态切换快捷键（2.0）</h3>
      <p class="mt-1 text-[11px] text-[color:var(--text-secondary)]">
        用于在「面板态 / 猫咪态」间切换。默认：
        <code class="rounded bg-[color:var(--bg-secondary)] px-1">Alt+T</code>
      </p>
      <div class="mt-2 flex items-center gap-2">
        <input
          v-model="modeShortcutDraft"
          type="text"
          class="flex-1 rounded-[8px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-3 py-2 text-sm text-[color:var(--text-primary)] outline-none placeholder:text-[color:var(--text-tertiary)] focus:border-[color:var(--primary)] focus:ring-2 focus:ring-[color:var(--primary-light)]"
          placeholder="Alt+T"
        />
        <button type="button" class="action-btn px-3 py-2" :disabled="busy" @click="onSaveModeShortcut">保存快捷键</button>
      </div>
      <p v-if="appliedModeShortcut" class="mt-2 text-[11px] text-[color:var(--text-tertiary)]">
        当前生效：<span class="font-medium text-[color:var(--text-secondary)]">{{ appliedModeShortcut }}</span>
      </p>
    </div>

    <div class="mt-3 rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
      <h3 class="text-sm font-semibold text-[color:var(--text-primary)]">宠物模式（2.0）</h3>
      <div class="mt-3 space-y-2 text-sm">
        <label class="flex items-center gap-2">
          <input v-model="petEnabled" type="checkbox" />
          <span>启用宠物模式</span>
        </label>
        <label class="flex items-center gap-2">
          <input v-model="petShowBadge" type="checkbox" :disabled="!petEnabled" />
          <span>显示角标（未完成数量）</span>
        </label>
        <label class="flex items-center gap-2">
          <input v-model="petAnimations" type="checkbox" :disabled="!petEnabled" />
          <span>启用动画效果</span>
        </label>
      </div>
    </div>

    <div class="mt-3 rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
      <h3 class="text-sm font-semibold text-[color:var(--text-primary)]">外观</h3>
      <div class="mt-3 space-y-2 text-sm">
        <label class="flex items-start gap-2">
          <input v-model="themePreferenceValue" type="radio" value="system" class="mt-1" />
          <span>跟随系统</span>
        </label>
        <label class="flex items-start gap-2">
          <input v-model="themePreferenceValue" type="radio" value="light" class="mt-1" />
          <span>浅色模式</span>
        </label>
        <label class="flex items-start gap-2">
          <input v-model="themePreferenceValue" type="radio" value="dark" class="mt-1" />
          <span>深色模式</span>
        </label>
      </div>
      <p class="mt-2 text-[11px] text-[color:var(--text-tertiary)]">当前生效：{{ resolvedThemeLabel }}</p>
    </div>

    <div class="mt-3 rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
      <div class="flex items-center justify-between">
        <div>
          <h3 class="text-sm font-semibold text-[color:var(--text-primary)]">开机自动启动</h3>
          <p class="mt-1 text-[11px] text-[color:var(--text-secondary)]">登录 macOS 后自动启动 Topdo</p>
        </div>
        <label class="inline-flex cursor-pointer items-center">
          <input v-model="autostartEnabled" type="checkbox" class="peer sr-only" :disabled="busy || autostartLoading" />
          <span
            class="relative h-6 w-11 rounded-full bg-[color:var(--bg-tertiary)] transition-colors after:absolute after:left-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:bg-[color:var(--bg-solid)] after:transition-all after:content-[''] peer-checked:bg-[color:var(--primary)] peer-checked:after:translate-x-5"
          />
        </label>
      </div>
    </div>

    <template v-if="selectedMode === 'feishu'">
      <div class="mt-4 space-y-4 rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
        <h3 class="text-sm font-semibold text-[color:var(--text-primary)]">飞书同步设置</h3>

        <div class="rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
          <div class="flex items-center justify-between">
            <h4 class="text-sm font-semibold text-[color:var(--text-primary)]">Step ① 创建任务表格</h4>
            <span v-if="stepState.templateReady" class="text-xs font-semibold text-[color:var(--status-done)]">✅ 已完成</span>
          </div>
          <p class="mt-2 text-xs text-[color:var(--text-secondary)]">如果你还没有多维表格，可直接打开 Topdo 模板链接并创建自己的文档。</p>
          <button
            type="button"
            class="mt-3 rounded-[8px] border border-[color:var(--primary)] bg-[color:var(--bg-solid)] px-3 py-1.5 text-xs font-medium text-[color:var(--primary)] transition-colors hover:bg-[color:var(--primary-light)]"
            @click="onOpenTemplateLink"
          >
            🔗 打开 Topdo 模板
          </button>
          <p class="mt-2 text-xs text-[color:var(--text-secondary)]">已有表格？可直接进入下一步。</p>
        </div>

        <div class="rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
          <div class="flex items-center justify-between">
            <h4 class="text-sm font-semibold text-[color:var(--text-primary)]">Step ② 粘贴表格链接</h4>
            <span v-if="stepState.linkParsed" class="text-xs font-semibold text-[color:var(--status-done)]">✅ 已完成</span>
          </div>
          <div class="mt-2 flex gap-2">
            <input
              v-model="form.bitableUrl"
              type="text"
              class="flex-1 rounded-[8px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-3 py-2 text-sm text-[color:var(--text-primary)] outline-none placeholder:text-[color:var(--text-tertiary)] focus:border-[color:var(--primary)] focus:ring-2 focus:ring-[color:var(--primary-light)]"
              placeholder="https://xxx.larkoffice.com/base/..."
              @blur="parseBitableUrl(true)"
            />
            <button
              type="button"
              class="rounded-[8px] border border-[color:var(--primary)] px-3 py-2 text-xs font-medium text-[color:var(--primary)] transition-colors hover:bg-[color:var(--primary-light)]"
              @click="parseBitableUrl(false)"
            >
              解析
            </button>
          </div>
          <p class="mt-2 text-xs" :class="stepState.linkParsed ? 'text-[color:var(--status-done)]' : 'text-[color:var(--text-secondary)]'">
            {{ stepState.linkParsed ? '✅ 已识别 App Token 和 Table ID' : '自动提取 base 后的 App Token 和 table 参数' }}
          </p>

          <div class="mt-3 space-y-3">
            <label class="block">
              <span class="mb-1 block text-sm text-[color:var(--text-secondary)]">App Token</span>
              <input
                v-model="form.appToken"
                type="text"
                class="w-full rounded-[8px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-3 py-2 text-sm text-[color:var(--text-primary)] outline-none placeholder:text-[color:var(--text-tertiary)] focus:border-[color:var(--primary)] focus:ring-2 focus:ring-[color:var(--primary-light)]"
                placeholder="请输入 App Token"
              />
              <p class="mt-1 text-xs text-[color:var(--text-secondary)]">从多维表格 URL 中 /base/ 后的标识</p>
            </label>

            <label class="block">
              <span class="mb-1 block text-sm text-[color:var(--text-secondary)]">Table ID</span>
              <input
                v-model="form.tableId"
                type="text"
                class="w-full rounded-[8px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-3 py-2 text-sm text-[color:var(--text-primary)] outline-none placeholder:text-[color:var(--text-tertiary)] focus:border-[color:var(--primary)] focus:ring-2 focus:ring-[color:var(--primary-light)]"
                placeholder="请输入 Table ID"
              />
              <p class="mt-1 text-xs text-[color:var(--text-secondary)]">从多维表格 URL 中 table= 后的部分</p>
            </label>
          </div>
        </div>

        <div class="rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
          <div class="flex items-center justify-between">
            <h4 class="text-sm font-semibold text-[color:var(--text-primary)]">Step ③ 填写应用凭证</h4>
            <span v-if="stepState.credentialReady" class="text-xs font-semibold text-[color:var(--status-done)]">✅ 已完成</span>
          </div>

          <div class="mt-3 space-y-3">
            <label class="block">
              <span class="mb-1 block text-sm text-[color:var(--text-secondary)]">App ID</span>
              <input
                v-model="form.appId"
                type="text"
                class="w-full rounded-[8px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-3 py-2 text-sm text-[color:var(--text-primary)] outline-none placeholder:text-[color:var(--text-tertiary)] focus:border-[color:var(--primary)] focus:ring-2 focus:ring-[color:var(--primary-light)]"
                placeholder="请输入 App ID（如 cli_xxx）"
              />
            </label>

            <label class="block">
              <span class="mb-1 block text-sm text-[color:var(--text-secondary)]">App Secret</span>
              <input
                v-model="form.appSecret"
                type="password"
                class="w-full rounded-[8px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-3 py-2 text-sm text-[color:var(--text-primary)] outline-none placeholder:text-[color:var(--text-tertiary)] focus:border-[color:var(--primary)] focus:ring-2 focus:ring-[color:var(--primary-light)]"
                placeholder="留空表示保留已配置的 Secret"
              />
            </label>
          </div>

          <div class="mt-3 rounded-[8px] border border-[color:var(--border)] bg-[color:var(--bg-secondary)] px-3 py-2 text-[12px] text-[color:var(--text-secondary)]">
            <p class="font-medium text-[color:var(--text-secondary)]">💡 在飞书开放平台创建企业自建应用，即可获取 App ID 和 App Secret</p>
            <div class="mt-2 flex flex-wrap items-center gap-2">
              <button
                type="button"
                class="rounded-[6px] border border-[color:var(--primary)] bg-[color:var(--bg-solid)] px-2.5 py-1 text-[11px] font-medium text-[color:var(--primary)] transition-colors hover:bg-[color:var(--primary-light)]"
                @click="onOpenTutorialLink"
              >
                📖 打开教程
              </button>
              <span v-if="stepState.tutorialCopied" class="text-[11px] font-semibold text-[color:var(--status-done)]">
                ✅ 已在浏览器打开
              </span>
            </div>
            <p class="mt-2 text-[11px] text-[color:var(--text-secondary)]">点击按钮后会直接用默认浏览器打开教程。</p>
          </div>
        </div>
      </div>
    </template>

    <div class="mt-4 rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3">
      <h3 class="text-sm font-semibold text-[color:var(--text-primary)]">关于 / 反馈</h3>
      <p class="mt-1 text-[11px] text-[color:var(--text-secondary)]">遇到问题或想提建议，可直接打开 GitHub 查看项目或提交反馈。</p>
      <div class="mt-3 grid grid-cols-2 gap-2">
        <button type="button" class="action-btn px-3 py-2" @click="onOpenGitHub">GitHub 主页</button>
        <button type="button" class="action-btn px-3 py-2" @click="onOpenFeedback">反馈地址</button>
      </div>
    </div>

    <p
      v-if="statusMessage"
      class="mt-4 rounded-[8px] px-3 py-2 text-sm"
      :class="
        statusType === 'success'
          ? 'bg-[color:var(--primary-light)] text-[color:var(--status-done)]'
          : 'bg-[color:var(--bg-tertiary)] text-[color:var(--priority-high)]'
      "
    >
      {{ statusMessage }}
    </p>
    <div
      v-if="statusType === 'error' && statusDetail"
      class="task-scrollbar mt-2 max-h-36 rounded-[8px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-2"
    >
      <div class="mb-2 flex items-center justify-end">
        <button type="button" class="action-btn px-2 py-1 text-[11px]" @click="onCopyErrorDetail">复制错误详情</button>
        <span v-if="errorDetailCopied" class="ml-2 text-[11px] text-[color:var(--status-done)]">已复制</span>
      </div>
      <pre class="whitespace-pre-wrap break-words font-mono text-[11px] leading-4 text-[color:var(--text-secondary)]">{{ statusDetail }}</pre>
    </div>

    <div class="mt-4 grid grid-cols-3 gap-2">
      <button type="button" class="action-btn" :disabled="busy || selectedMode !== 'feishu'" @click="onTestConnection">
        测试连接
      </button>
      <button type="button" class="action-btn" :disabled="busy" @click="onSave">保存</button>
      <button type="button" class="action-btn" :disabled="busy" @click="$emit('back')">返回</button>
    </div>

    <div class="mt-3 flex items-center justify-between">
      <button type="button" class="action-btn px-3 py-2" @click="showLogs = !showLogs">
        {{ showLogs ? '隐藏日志' : '查看日志' }}
      </button>
      <span class="text-[11px] text-[color:var(--text-tertiary)]">最近 {{ logs.length }} / 50 条</span>
    </div>

    <div v-if="showLogs" class="mt-3 rounded-[8px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-2">
      <div class="mb-2 flex items-center justify-end gap-2">
        <button type="button" class="action-btn px-2 py-1 text-[11px]" @click="onClearLogs">清除日志</button>
        <button type="button" class="action-btn px-2 py-1 text-[11px]" @click="onCopyLogs">复制全部</button>
      </div>
      <div class="task-scrollbar max-h-48 overflow-y-auto rounded-[6px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-2 py-1">
        <p
          v-for="(entry, idx) in logs"
          :key="`${entry.timestamp}-${entry.tag}-${idx}`"
          class="border-b border-[color:var(--border-light)] py-1 font-mono text-[11px] leading-4 text-[color:var(--text-primary)] last:border-b-0"
        >
          {{ formatLogLine(entry) }}
        </p>
        <p v-if="logs.length === 0" class="py-2 font-mono text-[11px] text-[color:var(--text-tertiary)]">暂无日志</p>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import { onMounted, reactive, ref, watch } from 'vue';
import { disable as disableAutostart, enable as enableAutostart, isEnabled as isAutostartEnabled } from '@tauri-apps/plugin-autostart';
import { usePetStore } from '../stores/petStore';
import { WindowMode } from '../types/pet';
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

const GITHUB_REPO_URL = 'https://github.com/SkyNone/Topdo';
const GITHUB_FEEDBACK_URL = 'https://github.com/SkyNone/Topdo/issues';

const emit = defineEmits<{
  (event: 'back'): void;
  (event: 'saved', mode: AppMode): void;
}>();

const petStore = usePetStore();
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

const form = reactive<FormState>({
  bitableUrl: '',
  appId: '',
  appSecret: '',
  appToken: '',
  tableId: ''
});

const FEISHU_TEMPLATE_URL =
  'https://s7wd8lze1s.feishu.cn/base/QR7rbtLf0adg0gsFun7cKnYOnGd?table=tblSeF0WH71ITCe7&view=vewMSNDmR0';
const FEISHU_TUTORIAL_URL = 'https://open.feishu.cn/app';

const stepState = reactive({
  templateReady: false,
  linkParsed: false,
  credentialReady: false,
  tutorialCopied: false
});

const { themePreference, resolvedTheme } = useThemeState();
const themePreferenceValue = ref<ThemePreference>(themePreference.value);
const resolvedThemeLabel = ref<'浅色' | '深色'>(resolvedTheme.value === 'dark' ? '深色' : '浅色');

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
  if (!raw) {
    return { appToken: '', tableId: '', error: '链接为空，请先粘贴多维表格链接' };
  }

  let normalized = raw;
  if (!/^https?:\/\//i.test(normalized)) {
    normalized = `https://${normalized}`;
  }

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

  if (!appToken && !tableId) {
    return { appToken: '', tableId: '', error: '未找到 App Token 和 Table ID，请确认是多维表格链接' };
  }
  if (!appToken) {
    return { appToken: '', tableId: '', error: '未找到 App Token（base 后的标识）' };
  }
  if (!tableId) {
    return { appToken: '', tableId: '', error: '未找到 Table ID（table=...）' };
  }

  return { appToken, tableId };
}

function parseBitableUrl(silentOnBlur: boolean) {
  const result = parseFeishuBitableUrl(form.bitableUrl);
  if (result.error) {
    stepState.linkParsed = false;
    if (!silentOnBlur) {
      setStatus('error', result.error);
    }
    return;
  }

  form.appToken = result.appToken;
  form.tableId = result.tableId;
  stepState.linkParsed = true;
  setStatus('success', '✅ 已识别 App Token 和 Table ID');
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
    form.bitableUrl =
      config.app_token && config.table_id ? `https://www.feishu.cn/base/${config.app_token}?table=${config.table_id}` : '';
    form.appSecret = '';

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

async function onSave() {
  busy.value = true;
  try {
    console.log('[Settings] 保存模式:', selectedMode.value);
    const effectiveWindowMode = petEnabled.value ? petWindowMode.value : WindowMode.Panel;

    const saveResult = await invoke('save_config', buildSaveConfigParams());
    console.log('[Settings] save_config 返回:', saveResult);
    if (selectedMode.value === 'feishu') {
      form.appSecret = '';
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
    console.log('[Settings] 保存后读取模式:', verifiedMode);
    if (verifiedMode !== selectedMode.value) {
      throw new Error(`模式保存失败，期望 ${selectedMode.value}，实际 ${verifiedMode || '空值'}`);
    }
    initialMode.value = selectedMode.value;

    if (autostartEnabled.value !== initialAutostartEnabled.value) {
      if (autostartEnabled.value) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
      initialAutostartEnabled.value = autostartEnabled.value;
    }

    setStatus('success', '设置保存成功');
    emit('saved', selectedMode.value);
  } catch (error) {
    setStatus('error', String(error));
  } finally {
    busy.value = false;
  }
}

async function onTestConnection() {
  busy.value = true;
  try {
    if (selectedMode.value !== 'feishu') {
      throw new Error('请先切换到飞书同步模式');
    }
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

onMounted(() => {
  void loadConfig();
  void loadShortcutConfig();
  void loadModeShortcutConfig();
  void loadPetSettings();
  void loadAutostartState();
});

function handleEsc(): boolean {
  if (showLogs.value) {
    showLogs.value = false;
    return true;
  }
  return false;
}

defineExpose({
  handleEsc
});

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
    resolvedThemeLabel.value = resolvedTheme.value === 'dark' ? '深色' : '浅色';
  }
);

watch(
  () => resolvedTheme.value,
  (value) => {
    resolvedThemeLabel.value = value === 'dark' ? '深色' : '浅色';
  }
);
</script>

<style scoped>
.action-btn {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-solid);
  color: var(--text-secondary);
  font-size: 12px;
  padding: 8px 4px;
  transition: all 0.15s ease;
}

.action-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.action-btn:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

</style>
