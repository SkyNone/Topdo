<template>
  <main class="h-full w-full bg-transparent text-[color:var(--text-primary)]">
    <section class="app-container relative mx-auto flex h-full w-full flex-col" :class="{ 'app-container-mini': isMiniMode }">
      <div
        v-if="isMiniMode"
        class="mini-shell"
        :class="{ pressed: miniPressed, dragging: miniDragging }"
        @mousedown="onMiniMouseDown"
      >
        <CatPet v-if="showMiniPet" :show-badge="petStore.showBadge" :animations="petStore.animations" />
        <button v-else type="button" class="mini-pill" @click.stop="restoreNormalMode">
          <span class="mini-pill__brand">Topdo</span>
          <span class="mini-pill__meta">
            <span class="mini-pill__count">{{ taskStore.todoCount }}</span>
            <span class="mini-pill__label">待办</span>
          </span>
        </button>
      </div>

      <template v-else>
        <TitleBar
          :always-on-top="isAlwaysOnTop"
          :resolved-theme="resolvedTheme"
          @settings="toggleSettingsView"
          @toggle-pin="onTogglePin"
          @toggle-theme="onToggleTheme"
          @mini="onEnterMiniMode"
          @close-to-tray="onHideToTray"
        />

        <section v-if="currentView === 'welcome'" class="min-h-0 flex-1">
          <Welcome @select-local="onSelectLocal" @select-feishu="onSelectFeishu" />
        </section>

        <section v-else-if="currentView === 'settings'" class="min-h-0 flex-1">
          <Settings ref="settingsRef" @back="currentView = 'main'" @saved="onSettingsSaved" />
        </section>

        <section v-else-if="taskStore.loading && taskStore.tasks.length === 0" class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 bg-transparent">
          <span class="h-7 w-7 animate-spin rounded-full border-2 border-[color:var(--primary)] border-t-transparent"></span>
          <p class="text-[var(--font-size-base)] text-[color:var(--text-secondary)]">正在加载任务...</p>
        </section>

        <section v-else-if="taskStore.error && taskStore.tasks.length === 0" class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 bg-transparent px-4 text-center">
          <p class="text-[var(--font-size-base)] text-[#FA5252]">{{ taskStore.error }}</p>
          <button type="button" class="rounded-[var(--radius-btn)] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-3 py-1 text-[var(--font-size-sm)] text-[color:var(--text-secondary)] hover:bg-[color:var(--bg-hover)]" @click="onRetry">
            重试
          </button>
        </section>

        <HabitView v-else-if="appStore.currentMode === 'habits'" />

        <section v-else class="flex min-h-0 flex-1 flex-col bg-transparent">
          <div class="px-3 pt-2">
            <StatsBar @add="openCreateTask()" />
          </div>
          <div v-if="searchQueryLabel" class="px-3 pt-2">
            <div class="search-state-bar">
              <div class="search-state-bar__main">
                <span class="search-state-bar__badge">搜索中</span>
                <span class="search-state-bar__text">“{{ searchQueryLabel }}”</span>
                <span class="search-state-bar__count">{{ searchResultCount }} 个结果</span>
              </div>
              <div class="search-state-bar__actions">
                <button type="button" class="search-state-bar__btn" @click="openSearch">编辑搜索</button>
                <button type="button" class="search-state-bar__btn search-state-bar__btn--primary" @click="clearSearch">清空</button>
              </div>
            </div>
          </div>
          <OnboardingBar
            v-if="showOnboarding"
            :steps="onboardingSteps"
            @dismiss="dismissOnboarding"
          />

          <TaskList
            ref="taskListRef"
            :mode="taskStore.mode"
            :creating="createInlineVisible"
            :create-template="createTemplate"
            :status-sync-state="taskStore.statusSyncState"
            :notes-sync-state="taskStore.notesSyncState"
            @cancel-create="closeCreateTask"
            @created="onTaskCreated"
            @create-template="openCreateTask"
            @create-habit-template="openHabitTemplate"
            @error="showError"
            @request-delete="openDeleteDialog"
          />

          <div v-if="firstReminderNudge" class="first-reminder-nudge">
            <div class="first-reminder-nudge__copy">
              <strong>要不要给「{{ firstReminderNudge.name }}」加个提醒？</strong>
              <span>{{ firstReminderNudge.dueDate ? '到时间后会通过系统通知和应用内提醒提示。' : '可以先设一个今天 23:59 的提醒，避免写完就忘。' }}</span>
            </div>
            <div class="first-reminder-nudge__actions">
              <button type="button" class="first-reminder-nudge__primary" @click="applyFirstReminderNudge('primary')">
                {{ firstReminderNudge.dueDate ? '到期时提醒' : '今天 23:59' }}
              </button>
              <button type="button" @click="applyFirstReminderNudge('secondary')">
                {{ firstReminderNudge.dueDate ? '提前30分钟' : '明天 10:00' }}
              </button>
              <button type="button" class="first-reminder-nudge__ghost" @click="dismissFirstReminderNudge">稍后</button>
            </div>
          </div>

          <PanelCatCorner
            v-if="petStore.enabled"
            class="panel-corner"
            :show-badge="petStore.showBadge"
            :animations="petStore.animations"
            @collapse="onEnterMiniMode"
          />

          <StatusBar
            :mode="taskStore.mode"
            :task-count="taskStore.tasks.length"
            :is-syncing="taskStore.isSyncing"
            :offline-mode="taskStore.offlineMode"
            :last-sync-time="taskStore.lastSyncTime"
            :pending-count="taskStore.pendingCount"
            :failed-count="taskStore.failedCount"
            :last-sync-error-summary="taskStore.lastSyncErrorSummary"
            @sync="onManualSync"
            @open-shortcuts="shortcutSheetVisible = true"
          />
        </section>
      </template>

      <ConfirmDialog
        v-model="deleteDialogVisible"
        title="删除任务"
        :message="`确定删除「${pendingDeleteTask?.name || '该任务'}」？`"
        confirm-text="删除"
        cancel-text="取消"
        :danger="true"
        @confirm="confirmDelete"
      />

    <ShortcutSheet
      v-if="shortcutSheetVisible && currentView === 'main' && !isMiniMode"
      @close="shortcutSheetVisible = false"
    />

      <ReminderToast
        v-if="!isMiniMode"
        :items="reminderToasts"
        @open="openReminderToast"
        @dismiss="dismissReminderToast"
      />

      <div
        v-if="taskStore.searchOpen && currentView === 'main' && !isMiniMode"
        class="search-overlay"
        @click.self="closeSearch"
      >
        <div class="search-panel">
          <div class="search-panel__header">
            <span class="search-panel__title">搜索任务</span>
            <div class="search-panel__actions">
              <button
                v-if="taskStore.searchQuery.trim()"
                type="button"
                class="search-panel__action-btn"
                @click="clearSearch"
              >
                清空
              </button>
              <button type="button" class="search-panel__close" @click="closeSearch">Esc</button>
            </div>
          </div>
          <input
            ref="searchInputRef"
            :value="taskStore.searchQuery"
            type="text"
            class="search-panel__input"
            placeholder="搜索任务名称或备注"
            @input="taskStore.setSearchQuery(($event.target as HTMLInputElement).value)"
          />
          <div class="search-panel__meta">
            <span v-if="taskStore.searchQuery.trim()">{{ taskStore.filteredTasks.length }} 个结果</span>
            <span v-else>输入后即时过滤当前列表</span>
            <span>Cmd+F 打开，Esc 关闭</span>
          </div>
        </div>
      </div>

      <div v-if="toast" class="pointer-events-none absolute bottom-3 left-1/2 z-40 -translate-x-1/2 rounded-[var(--radius-btn)] bg-[#212529] px-3 py-1.5 text-[var(--font-size-sm)] text-white">
        {{ toast }}
      </div>
    </section>
  </main>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import ConfirmDialog from './components/ConfirmDialog.vue';
import CatPet from './components/CatPet/CatPet.vue';
import OnboardingBar from './components/OnboardingBar.vue';
import PanelCatCorner from './components/PanelCatCorner.vue';
import ReminderToast from './components/ReminderToast.vue';
import Settings from './components/Settings.vue';
import ShortcutSheet from './components/ShortcutSheet.vue';
import StatsBar from './components/StatsBar.vue';
import StatusBar from './components/StatusBar.vue';
import TaskList from './components/TaskList.vue';
import TitleBar from './components/TitleBar.vue';
import Welcome from './components/Welcome.vue';
import HabitView from './views/HabitView.vue';
import { useAppStore } from './stores/appStore';
import { useHabitStore } from './stores/habitStore';
import { usePetStore } from './stores/petStore';
import { useTaskStore } from './stores/taskStore';
import type { TaskFilter } from './stores/taskStore';
import type { RecurrenceRule, Task } from './types';
import { WindowMode } from './types/pet';
import { startHabitReminderService, startReminderService, type InAppReminder } from './utils/reminder';
import { initializeTheme, toggleThemeQuickly, useThemeState } from './utils/theme';

type ViewType = 'welcome' | 'main' | 'settings';
const NORMAL_MIN_WIDTH = 320;
const NORMAL_MIN_HEIGHT = 300;

interface WindowStatePayload {
  mini_mode: boolean;
  always_on_top: boolean;
}

interface WindowSizePayload {
  width: number;
  height: number;
}

interface WindowModeChangedPayload {
  mode: 'panel' | 'cat';
  mini_mode: boolean;
}

interface QuickTaskTemplate {
  name: string;
  priority?: string;
  dueDate?: string;
  dueTime?: string;
  recurrenceRule?: RecurrenceRule | null;
  reminderBefore?: number | null;
  notes?: string;
  expand?: 'priority' | 'date' | 'repeat' | 'reminder' | null;
}

interface CreatedTaskPayload {
  recordId: string;
  name: string;
  dueDate: string;
  reminderBefore: number | null;
}

const taskStore = useTaskStore();
const appStore = useAppStore();
const habitStore = useHabitStore();
const petStore = usePetStore();
const appWindow = getCurrentWindow();

const currentView = ref<ViewType>('main');
const isMiniMode = ref(false);
const isAlwaysOnTop = ref(true);
const createInlineVisible = ref(false);
const createTemplate = ref<QuickTaskTemplate | null>(null);
const shortcutSheetVisible = ref(false);
const miniPressed = ref(false);
const miniDragging = ref(false);
const searchInputRef = ref<HTMLInputElement | null>(null);

let miniStartPoint: { x: number; y: number } | null = null;
let miniMouseMoveHandler: ((event: MouseEvent) => void) | null = null;
let miniMouseUpHandler: (() => void) | null = null;

const toast = ref('');
const reminderToasts = ref<InAppReminder[]>([]);
let toastTimer: ReturnType<typeof setTimeout> | null = null;
const reminderToastTimers = new Map<string, ReturnType<typeof setTimeout>>();
let resizeTimer: ReturnType<typeof setTimeout> | null = null;
let unlistenResized: (() => void) | null = null;
let unlistenWindowModeChanged: UnlistenFn | null = null;
let unlistenFocusChanged: (() => void) | null = null;
let unlistenTasksUpdated: UnlistenFn | null = null;
let initialTraitsRetryTimer: ReturnType<typeof setTimeout> | null = null;

const deleteDialogVisible = ref(false);
const pendingDeleteTask = ref<Task | null>(null);
const taskListRef = ref<any>(null);
const settingsRef = ref<any>(null);

const ONBOARDING_KEY = 'topdo_onboarding_v1_dismissed';
const SHORTCUT_TIP_KEY = 'topdo_shortcut_tip_seen_v1';
const FIRST_REMINDER_NUDGE_KEY = 'topdo_first_reminder_nudge_seen_v1';
const onboardingPendingFromFirstLaunch = ref(false);
const showOnboarding = ref(false);
const firstReminderNudge = ref<CreatedTaskPayload | null>(null);
const showMiniPet = computed(() => isMiniMode.value && petStore.enabled);
const searchQueryLabel = computed(() => taskStore.searchQuery.trim());
const searchResultCount = computed(() => taskStore.filteredTasks.length);

const { resolvedTheme } = useThemeState();
const onboardingSteps = computed(() => ({
  createDone: taskStore.totalTaskCount > 0,
  progressDone: taskStore.inProgressTaskCount > 0,
  doneDone: taskStore.completedTaskCount > 0
}));

function showError(message: string) {
  showToast(message);
}

function showToast(message: string, duration = 2500) {
  if (!message) return;
  toast.value = message;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.value = '';
  }, duration);
}

function dismissReminderToast(id: string) {
  const timer = reminderToastTimers.get(id);
  if (timer) {
    clearTimeout(timer);
    reminderToastTimers.delete(id);
  }
  reminderToasts.value = reminderToasts.value.filter((item) => item.id !== id);
}

function showReminderToast(reminder: InAppReminder) {
  reminderToasts.value = [reminder, ...reminderToasts.value.filter((item) => item.id !== reminder.id)].slice(0, 3);
  const existing = reminderToastTimers.get(reminder.id);
  if (existing) clearTimeout(existing);
  reminderToastTimers.set(
    reminder.id,
    setTimeout(() => {
      dismissReminderToast(reminder.id);
    }, 6000)
  );
}

async function openReminderToast(reminder: InAppReminder) {
  dismissReminderToast(reminder.id);
  if (isMiniMode.value) {
    await restoreNormalMode();
  }
  currentView.value = 'main';
  if (reminder.kind === 'habit') {
    appStore.switchMode('habits');
    return;
  }
  appStore.switchMode('tasks');
  taskStore.clearSearch();
  taskStore.setFilter('all');
  await nextTick();
  taskListRef.value?.openTask?.(reminder.targetId);
}

async function syncWindowState() {
  try {
    const state = await invoke<WindowStatePayload>('get_window_state');
    isMiniMode.value = state.mini_mode;
    isAlwaysOnTop.value = state.always_on_top;
  } catch {
    // ignore
  }
}

async function reapplyWindowTraits() {
  try {
    await invoke('reapply_window_traits');
  } catch {
    // ignore
  }
}

async function ensureInitialWindowTraitsApplied() {
  try {
    await invoke('reapply_window_traits');
  } catch (error) {
    console.warn('首次应用窗口 traits 失败，准备重试:', error);
    if (initialTraitsRetryTimer) {
      clearTimeout(initialTraitsRetryTimer);
    }
    initialTraitsRetryTimer = setTimeout(() => {
      void reapplyWindowTraits().then(() => reconcileWindowMode());
    }, 450);
  }
}

async function reconcileWindowMode() {
  try {
    const state = await invoke<WindowStatePayload>('get_window_state');
    isMiniMode.value = state.mini_mode;
    const mode = state.mini_mode ? WindowMode.Cat : WindowMode.Panel;
    if (petStore.windowMode !== mode) {
      petStore.windowMode = mode;
      await petStore.save();
    }
    if (state.mini_mode) {
      await applyPetPosition();
    }
  } catch {
    // ignore
  }
}

async function bootstrap() {
  await taskStore.initMode();
  if (taskStore.firstLaunch) {
    onboardingPendingFromFirstLaunch.value = true;
    currentView.value = 'welcome';
    return;
  }

  currentView.value = 'main';
  await taskStore.fetchTasks();
}

async function onSelectLocal() {
  try {
    await taskStore.setMode('local');
    currentView.value = 'main';
    maybeShowOnboarding();
    maybeShowShortcutTip();
  } catch (error) {
    showError(String(error));
  }
}

async function onSelectFeishu() {
  try {
    await invoke('set_app_mode', { mode: 'feishu' });
    await taskStore.initMode();
    currentView.value = 'settings';
  } catch (error) {
    showError(String(error));
  }
}

async function onSettingsSaved(mode: 'local' | 'feishu') {
  // 先固定 UI 模式，避免因后续网络/配置错误回退到本地模式
  taskStore.setModeState(mode);
  taskStore.error = null;
  taskStore.offlineMode = false;
  currentView.value = 'main';
  maybeShowOnboarding();
  maybeShowShortcutTip();

  try {
    await taskStore.fetchTasks();
  } catch (error) {
    // 保持当前模式，仅提示错误
    showError(String(error));
  }
}

function openCreateTask(template: QuickTaskTemplate | null = null) {
  appStore.switchMode('tasks');
  currentView.value = 'main';
  createTemplate.value = template;
  createInlineVisible.value = true;
}

function closeCreateTask() {
  createInlineVisible.value = false;
  createTemplate.value = null;
}

function wasFirstReminderNudgeSeen(): boolean {
  try {
    return localStorage.getItem(FIRST_REMINDER_NUDGE_KEY) === '1';
  } catch {
    return false;
  }
}

function markFirstReminderNudgeSeen() {
  try {
    localStorage.setItem(FIRST_REMINDER_NUDGE_KEY, '1');
  } catch {
    // ignore
  }
}

function onTaskCreated(payload: CreatedTaskPayload) {
  closeCreateTask();
  if (!payload.recordId) return;
  if (payload.reminderBefore !== null) return;
  if (wasFirstReminderNudgeSeen()) return;
  firstReminderNudge.value = payload;
}

function dismissFirstReminderNudge() {
  firstReminderNudge.value = null;
  markFirstReminderNudgeSeen();
}

function dateKeyWithOffset(days: number): string {
  const date = new Date();
  date.setDate(date.getDate() + days);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

async function applyFirstReminderNudge(kind: 'primary' | 'secondary') {
  const nudge = firstReminderNudge.value;
  if (!nudge) return;

  const patch = nudge.dueDate
    ? { reminder_before: kind === 'primary' ? 0 : 30 }
    : {
        due_date: kind === 'primary'
          ? `${dateKeyWithOffset(0)}T23:59`
          : `${dateKeyWithOffset(1)}T10:00`,
        reminder_before: 0
      };

  try {
    await taskStore.updateTaskDetails(nudge.recordId, patch);
    showToast('提醒已设置');
    dismissFirstReminderNudge();
  } catch (error) {
    showError(`设置提醒失败：${String(error)}`);
  }
}

function openHabitTemplate() {
  appStore.setHabitModuleEnabled(true);
  appStore.switchMode('habits');
  currentView.value = 'main';
  closeCreateTask();
  showToast('已打开习惯页，可以点击 + 创建「每天喝水」');
}

async function ensureNotificationPermission() {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === 'granted';
    }
    if (!granted) {
      console.warn('Topdo 通知权限未开启');
    }
  } catch (error) {
    console.warn('请求通知权限失败:', error);
  }
}

function onToggleTheme() {
  toggleThemeQuickly();
}

async function onTogglePin() {
  try {
    const pinned = await invoke<boolean>('toggle_always_on_top');
    isAlwaysOnTop.value = pinned;
  } catch (error) {
    showError(String(error));
  }
}

async function onEnterMiniMode() {
  try {
    shortcutSheetVisible.value = false;
    isMiniMode.value = true;
    petStore.windowMode = WindowMode.Cat;
    await invoke('set_window_mode', { mode: 'cat' });
    await petStore.save();
    await applyPetPosition();
  } catch (error) {
    isMiniMode.value = false;
    petStore.windowMode = WindowMode.Panel;
    showError(String(error));
  }
}

async function restoreNormalMode() {
  try {
    await invoke('set_window_mode', { mode: 'panel' });
    isMiniMode.value = false;
    petStore.windowMode = WindowMode.Panel;
    await petStore.save();
  } catch (error) {
    showError(String(error));
  }
}

async function persistPetPosition() {
  try {
    const position = await appWindow.outerPosition();
    petStore.catPosition = {
      x: Number(position.x ?? 0),
      y: Number(position.y ?? 0),
    };
    await petStore.save();
  } catch {
    // ignore position persistence failure
  }
}

async function applyPetPosition() {
  const x = Number(petStore.catPosition.x ?? 0);
  const y = Number(petStore.catPosition.y ?? 0);
  if (!Number.isFinite(x) || !Number.isFinite(y) || (x === 0 && y === 0)) return;
  try {
    await appWindow.setPosition(new PhysicalPosition(x, y));
  } catch {
    // ignore invalid saved position
  }
}

function clearMiniDragListeners() {
  if (miniMouseMoveHandler) {
    window.removeEventListener('mousemove', miniMouseMoveHandler);
    miniMouseMoveHandler = null;
  }
  if (miniMouseUpHandler) {
    window.removeEventListener('mouseup', miniMouseUpHandler);
    miniMouseUpHandler = null;
  }
  window.removeEventListener('blur', cancelMiniInteraction);
  document.removeEventListener('mouseleave', cancelMiniInteraction);
}

function cancelMiniInteraction() {
  clearMiniDragListeners();
  miniStartPoint = null;
  miniPressed.value = false;
  miniDragging.value = false;
}

function onMiniMouseDown(event: MouseEvent) {
  if (event.button !== 0) return;
  miniPressed.value = true;
  miniDragging.value = false;
  miniStartPoint = { x: event.clientX, y: event.clientY };

  clearMiniDragListeners();
  miniMouseMoveHandler = (moveEvent: MouseEvent) => {
    if (!miniStartPoint || miniDragging.value) return;
    const dx = moveEvent.clientX - miniStartPoint.x;
    const dy = moveEvent.clientY - miniStartPoint.y;
    if (Math.hypot(dx, dy) < 3) return;
    miniDragging.value = true;
    clearMiniDragListeners();
    void appWindow
      .startDragging()
      .catch((error) => {
        showError(String(error));
      })
      .finally(() => {
        window.setTimeout(() => {
          miniStartPoint = null;
          miniPressed.value = false;
          miniDragging.value = false;
          void persistPetPosition();
        }, 120);
      });
  };

  miniMouseUpHandler = () => {
    const shouldRestore = !miniDragging.value;
    clearMiniDragListeners();
    miniStartPoint = null;
    if (shouldRestore) {
      void restoreNormalMode();
    } else {
      void persistPetPosition();
    }
    window.setTimeout(() => {
      miniPressed.value = false;
      miniDragging.value = false;
    }, 60);
  };

  window.addEventListener('mousemove', miniMouseMoveHandler);
  window.addEventListener('mouseup', miniMouseUpHandler);
  window.addEventListener('blur', cancelMiniInteraction);
  document.addEventListener('mouseleave', cancelMiniInteraction);
}

async function onHideToTray() {
  try {
    await invoke('hide_window_to_tray');
  } catch (error) {
    showError(String(error));
  }
}

async function onRetry() {
  try {
    await taskStore.fetchTasks();
  } catch (error) {
    showError(String(error));
  }
}

async function onManualSync() {
  if (taskStore.mode !== 'feishu') return;
  try {
    await taskStore.triggerSync();
  } catch (error) {
    showError(String(error));
  }
}

function onVisibilityChange() {
  if (document.visibilityState === 'visible') {
    void reapplyWindowTraits().then(() => reconcileWindowMode());
    if (taskStore.mode === 'feishu' && currentView.value === 'main') {
      void taskStore.triggerSync().catch((error) => showError(String(error)));
    }
  }
}

function wasOnboardingDismissed(): boolean {
  try {
    return localStorage.getItem(ONBOARDING_KEY) === '1';
  } catch {
    return false;
  }
}

function dismissOnboarding() {
  showOnboarding.value = false;
  try {
    localStorage.setItem(ONBOARDING_KEY, '1');
  } catch {
    // ignore
  }
}

function maybeShowOnboarding() {
  if (!onboardingPendingFromFirstLaunch.value) return;
  if (currentView.value !== 'main') return;
  if (wasOnboardingDismissed()) return;
  showOnboarding.value = true;
}

function wasShortcutTipSeen(): boolean {
  try {
    return localStorage.getItem(SHORTCUT_TIP_KEY) === '1';
  } catch {
    return false;
  }
}

function markShortcutTipSeen() {
  try {
    localStorage.setItem(SHORTCUT_TIP_KEY, '1');
  } catch {
    // ignore
  }
}

function maybeShowShortcutTip() {
  if (currentView.value !== 'main') return;
  if (wasShortcutTipSeen()) return;
  showToast('按 ⌘K 查看全部快捷键', 2200);
  markShortcutTipSeen();
}

function toggleShortcutSheet() {
  if (currentView.value !== 'main') return;
  shortcutSheetVisible.value = !shortcutSheetVisible.value;
}

function toggleSettingsView() {
  currentView.value = currentView.value === 'settings' ? 'main' : 'settings';
}

function openSearch() {
  if (currentView.value !== 'main' || isMiniMode.value) return;
  taskStore.openSearch();
  void nextTick(() => {
    searchInputRef.value?.focus();
    searchInputRef.value?.select();
  });
}

function closeSearch() {
  taskStore.closeSearch();
}

function clearSearch() {
  taskStore.clearSearch();
}

function openDeleteDialog(task: Task) {
  pendingDeleteTask.value = task;
  deleteDialogVisible.value = true;
}

async function confirmDelete() {
  const task = pendingDeleteTask.value;
  if (!task) {
    deleteDialogVisible.value = false;
    return;
  }

  try {
    await taskStore.deleteTask(task.record_id);
    deleteDialogVisible.value = false;
    pendingDeleteTask.value = null;
  } catch (error) {
    showError(String(error));
  }
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName.toLowerCase();
  if (tag === 'input' || tag === 'textarea' || target.isContentEditable) return true;
  return Boolean(target.closest('input, textarea, [contenteditable="true"]'));
}

function onGlobalKeydown(event: KeyboardEvent) {
  const isMeta = event.metaKey;
  const isShift = event.shiftKey;
  const key = event.key.toLowerCase();
  const editable = isEditableTarget(event.target);

  if (event.key === 'Escape') {
    if (taskStore.searchOpen) {
      event.preventDefault();
      closeSearch();
      return;
    }
    event.preventDefault();
    if (shortcutSheetVisible.value) {
      shortcutSheetVisible.value = false;
      return;
    }
    if (deleteDialogVisible.value) {
      deleteDialogVisible.value = false;
      pendingDeleteTask.value = null;
      return;
    }
    if (currentView.value === 'settings') {
      const consumed = settingsRef.value?.handleEsc?.() === true;
      if (!consumed) currentView.value = 'main';
      return;
    }
    if (createInlineVisible.value) {
      closeCreateTask();
      return;
    }
    return;
  }

  if (isMeta && key === 'n') {
    event.preventDefault();
    if (currentView.value !== 'main') return;
    openCreateTask();
    return;
  }

  if (isMeta && !isShift && key === 'f') {
    event.preventDefault();
    openSearch();
    return;
  }

  if (isMeta && !isShift && key === 'j') {
    event.preventDefault();
    appStore.toggleMode();
    return;
  }

  if (isMeta && !isShift && key === 'k') {
    event.preventDefault();
    toggleShortcutSheet();
    return;
  }

  if (isMeta && event.key === ',') {
    event.preventDefault();
    currentView.value = currentView.value === 'settings' ? 'main' : 'settings';
    return;
  }

  if (isMeta && isShift && key === 'r') {
    event.preventDefault();
    if (currentView.value === 'main' && taskStore.mode === 'feishu') {
      void onManualSync();
    }
    return;
  }

  if (isMeta && isShift && key === 'l') {
    event.preventDefault();
    onToggleTheme();
    return;
  }

  if (isMeta && !isShift && ['1', '2', '3', '4'].includes(key)) {
    event.preventDefault();
    if (currentView.value !== 'main') return;
    const mapping: Record<string, TaskFilter> = {
      '1': 'pending',
      '2': 'in_progress',
      '3': 'done',
      '4': 'all'
    };
    taskStore.setFilter(mapping[key]);
    return;
  }

  if (currentView.value !== 'main') return;
  if (editable && !isMeta) return;

  if (event.key === 'ArrowUp') {
    event.preventDefault();
    taskListRef.value?.moveFocus?.(-1);
    return;
  }
  if (event.key === 'ArrowDown') {
    event.preventDefault();
    taskListRef.value?.moveFocus?.(1);
    return;
  }
  if (event.key === 'Enter' && isMeta) {
    event.preventDefault();
    void taskListRef.value?.toggleFocusedStatus?.();
    return;
  }
  if (event.key === 'Enter') {
    event.preventDefault();
    void taskListRef.value?.toggleFocusedExpand?.();
    return;
  }
  if ((event.key === 'Backspace' || event.key === 'Delete') && taskStore.mode === 'local') {
    event.preventDefault();
    taskListRef.value?.requestDeleteFocused?.();
  }
}

onMounted(async () => {
  initializeTheme();
  appStore.load();
  await habitStore.fetchHabits().catch((error) => showError(String(error)));
  await petStore.load().catch(() => undefined);
  await syncWindowState();
  unlistenWindowModeChanged = await listen<WindowModeChangedPayload>('window-mode-changed', (event) => {
    const payload = event.payload;
    isMiniMode.value = payload.mini_mode;
    petStore.windowMode = payload.mode === 'cat' ? WindowMode.Cat : WindowMode.Panel;
    void petStore.save();
    if (payload.mode === 'cat') {
      void applyPetPosition();
    }
  });
  unlistenTasksUpdated = await listen('tasks-updated', () => {
    void taskStore.fetchTasks().catch((error) => showError(String(error)));
  });
  try {
    const savedSize = await invoke<WindowSizePayload | null>('get_window_size');
    if (!isMiniMode.value && savedSize && savedSize.width > 0 && savedSize.height > 0) {
      await appWindow.setSize(new LogicalSize(
        Math.max(savedSize.width, NORMAL_MIN_WIDTH),
        Math.max(savedSize.height, NORMAL_MIN_HEIGHT)
      ));
    }
  } catch (error) {
    console.warn('恢复窗口尺寸失败:', error);
  }

  unlistenResized = await appWindow.onResized(({ payload: size }) => {
    if (resizeTimer) {
      clearTimeout(resizeTimer);
    }

    resizeTimer = setTimeout(async () => {
      try {
        if (isMiniMode.value) return;
        await invoke('save_window_size', {
          width: Math.max(size.width, NORMAL_MIN_WIDTH),
          height: Math.max(size.height, NORMAL_MIN_HEIGHT)
        });
      } catch (error) {
        console.warn('保存窗口尺寸失败:', error);
      }
    }, 500);
  });
  unlistenFocusChanged = await appWindow.onFocusChanged(({ payload }) => {
    if (payload) {
      void reapplyWindowTraits().then(() => reconcileWindowMode());
      if (currentView.value === 'main') void taskStore.fetchTasks().catch((error) => showError(String(error)));
    }
  });

  await bootstrap();
  if (petStore.enabled && (isMiniMode.value || petStore.windowMode === WindowMode.Cat)) {
    isMiniMode.value = true;
    petStore.windowMode = WindowMode.Cat;
    await applyPetPosition();
  } else {
    if (isMiniMode.value) {
      await restoreNormalMode();
    }
    isMiniMode.value = false;
    petStore.windowMode = WindowMode.Panel;
  }
  await petStore.save();
  await taskStore.initRecurringTasks().catch((error) => showError(String(error)));
  await ensureNotificationPermission();
  taskStore.startDailyRecurrenceCheck();
  startReminderService(
    () => (appStore.reminderEnabled ? taskStore.tasks : []),
    (recordId) => taskStore.markTaskReminderNotified(recordId),
    showReminderToast
  );
  startHabitReminderService(() => habitStore.habits, () => habitStore.logs, showReminderToast);
  await ensureInitialWindowTraitsApplied();
  maybeShowOnboarding();
  maybeShowShortcutTip();
  document.addEventListener('visibilitychange', onVisibilityChange);
  document.addEventListener('keydown', onGlobalKeydown);
});

onUnmounted(() => {
  document.removeEventListener('visibilitychange', onVisibilityChange);
  document.removeEventListener('keydown', onGlobalKeydown);
  clearMiniDragListeners();
  if (unlistenResized) {
    unlistenResized();
    unlistenResized = null;
  }
  if (unlistenWindowModeChanged) {
    unlistenWindowModeChanged();
    unlistenWindowModeChanged = null;
  }
  if (unlistenFocusChanged) {
    unlistenFocusChanged();
    unlistenFocusChanged = null;
  }
  if (unlistenTasksUpdated) {
    unlistenTasksUpdated();
    unlistenTasksUpdated = null;
  }
  if (resizeTimer) {
    clearTimeout(resizeTimer);
    resizeTimer = null;
  }
  if (initialTraitsRetryTimer) {
    clearTimeout(initialTraitsRetryTimer);
    initialTraitsRetryTimer = null;
  }
  if (toastTimer) {
    clearTimeout(toastTimer);
    toastTimer = null;
  }
  reminderToastTimers.forEach((timer) => clearTimeout(timer));
  reminderToastTimers.clear();
});

watch(
  () => onboardingSteps.value,
  (steps) => {
    if (!showOnboarding.value) return;
    if (steps.createDone && steps.progressDone && steps.doneDone) {
      dismissOnboarding();
    }
  },
  { deep: true }
);

watch(
  () => petStore.enabled,
  () => {
    if (!isMiniMode.value) return;
    void invoke('set_window_mode', { mode: 'cat' }).catch((error) => {
      showError(String(error));
    });
  }
);

watch(
  () => taskStore.searchOpen,
  (open) => {
    if (!open) return;
    void nextTick(() => {
      searchInputRef.value?.focus();
      searchInputRef.value?.select();
    });
  }
);
</script>

<style scoped>
.app-container {
  width: 100%;
  height: 100%;
  background: var(--bg-solid);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  border-radius: 12px;
  border: 0.5px solid var(--border-light);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  box-shadow:
    var(--shadow-md),
    var(--shadow-sm);
}

.app-container-mini {
  background: transparent;
  border: none;
  box-shadow: none;
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  overflow: visible;
}

.first-reminder-nudge {
  margin: 0 12px 8px;
  padding: 10px 12px;
  display: grid;
  gap: 10px;
  border: 1px solid color-mix(in srgb, var(--primary) 24%, var(--border));
  border-radius: 14px;
  background: linear-gradient(135deg, color-mix(in srgb, var(--primary) 8%, var(--bg-solid)), var(--bg-solid));
  box-shadow: var(--shadow-sm);
}

.first-reminder-nudge__copy {
  display: grid;
  gap: 3px;
}

.first-reminder-nudge__copy strong {
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 650;
  line-height: 1.35;
}

.first-reminder-nudge__copy span {
  color: var(--text-tertiary);
  font-size: 11px;
  line-height: 1.35;
}

.first-reminder-nudge__actions {
  display: flex;
  gap: 6px;
  align-items: center;
}

.first-reminder-nudge__actions button {
  height: 28px;
  padding: 0 10px;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--bg-solid);
  color: var(--text-secondary);
  font-family: var(--font-family);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
}

.first-reminder-nudge__actions .first-reminder-nudge__primary {
  border-color: var(--primary);
  background: var(--primary);
  color: white;
}

.first-reminder-nudge__actions .first-reminder-nudge__ghost {
  margin-left: auto;
  border-color: transparent;
  background: transparent;
}

.mini-shell {
  margin: 0;
  height: 100%;
  width: 100%;
  padding: 8px;
  box-sizing: border-box;
  border: none;
  background: transparent;
  box-shadow: none;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0;
  cursor: grab;
  user-select: none;
}

.mini-pill {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  width: 100%;
  min-height: 40px;
  padding: 0 16px;
  border: 0.5px solid var(--border-light);
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-solid) 96%, transparent);
  box-shadow: var(--shadow-md);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  font-weight: 500;
  cursor: pointer;
}

.mini-pill__brand {
  font-size: 15px;
  font-weight: 650;
  letter-spacing: -0.02em;
}

.mini-pill__meta {
  display: inline-flex;
  align-items: baseline;
  gap: 4px;
  flex-shrink: 0;
  white-space: nowrap;
}

.mini-pill__count {
  font-size: 18px;
  line-height: 1;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--text-primary);
}

.mini-pill__label {
  font-size: 11px;
  color: var(--text-secondary);
}

.mini-shell.pressed {
  transform: scale(0.98);
}

.mini-shell.dragging {
  cursor: grabbing;
}

.panel-corner {
  position: absolute;
  left: 8px;
  bottom: 34px;
  z-index: 30;
}

.search-state-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  border: 0.5px solid color-mix(in srgb, var(--primary) 16%, var(--border));
  border-radius: 12px;
  background: color-mix(in srgb, var(--primary) 6%, var(--bg-solid));
}

.search-state-bar__main {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.search-state-bar__badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 20px;
  padding: 0 8px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--primary) 14%, var(--bg-solid));
  color: var(--primary);
  font-size: 11px;
  font-weight: 600;
}

.search-state-bar__text {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.search-state-bar__count {
  font-size: 11px;
  color: var(--text-secondary);
}

.search-state-bar__actions {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.search-state-bar__btn {
  border: 0.5px solid var(--border);
  border-radius: 8px;
  background: var(--bg-solid);
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 1;
  padding: 6px 10px;
}

.search-state-bar__btn--primary {
  color: var(--primary);
  border-color: color-mix(in srgb, var(--primary) 22%, var(--border));
}

.search-overlay {
  position: absolute;
  inset: 0;
  z-index: 45;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: 56px 16px 16px;
  background: color-mix(in srgb, var(--bg-secondary) 28%, transparent);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
}

.search-panel {
  width: min(100%, 420px);
  border-radius: 14px;
  border: 0.5px solid var(--border-light);
  background: color-mix(in srgb, var(--bg-solid) 96%, transparent);
  box-shadow: var(--shadow-md);
  padding: 14px;
}

.search-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}

.search-panel__actions {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.search-panel__title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.search-panel__action-btn {
  border: 0.5px solid var(--border);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 1;
  padding: 5px 10px;
}

.search-panel__close {
  border: 0.5px solid var(--border);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 1;
  padding: 5px 8px;
}

.search-panel__input {
  width: 100%;
  border-radius: 10px;
  border: 0.5px solid var(--border-light);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 14px;
  padding: 10px 12px;
  outline: none;
}

.search-panel__input:focus {
  border-color: color-mix(in srgb, var(--primary) 55%, var(--border));
}

.search-panel__meta {
  margin-top: 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 11px;
  color: var(--text-tertiary);
}
</style>
