<template>
  <main class="h-full w-full bg-transparent text-[color:var(--text-primary)]">
    <section class="app-container relative mx-auto flex h-full w-full flex-col">
      <div
        v-if="isMiniMode"
        class="mini-shell"
        :class="{ pressed: miniPressed, dragging: miniDragging }"
        @mousedown="onMiniMouseDown"
      >
        <span class="mini-dot" aria-hidden="true"></span>
        <div class="mini-content">
          <span class="mini-count">{{ taskStore.todoCount }}</span>
          <span class="mini-label">待办</span>
        </div>
      </div>

      <template v-else>
        <TitleBar
          :always-on-top="isAlwaysOnTop"
          :resolved-theme="resolvedTheme"
          @settings="currentView = 'settings'"
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

        <section v-else-if="taskStore.loading" class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 bg-transparent">
          <span class="h-7 w-7 animate-spin rounded-full border-2 border-[color:var(--primary)] border-t-transparent"></span>
          <p class="text-[var(--font-size-base)] text-[color:var(--text-secondary)]">正在加载任务...</p>
        </section>

        <section v-else-if="taskStore.error && taskStore.tasks.length === 0" class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 bg-transparent px-4 text-center">
          <p class="text-[var(--font-size-base)] text-[#FA5252]">{{ taskStore.error }}</p>
          <button type="button" class="rounded-[var(--radius-btn)] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-3 py-1 text-[var(--font-size-sm)] text-[color:var(--text-secondary)] hover:bg-[color:var(--bg-hover)]" @click="onRetry">
            重试
          </button>
        </section>

        <section v-else class="flex min-h-0 flex-1 flex-col bg-transparent">
          <div class="px-3 pt-2">
            <StatsBar @add="createInlineVisible = true" />
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
            :status-sync-state="taskStore.statusSyncState"
            :notes-sync-state="taskStore.notesSyncState"
            @cancel-create="createInlineVisible = false"
            @created="createInlineVisible = false"
            @error="showError"
            @request-delete="openDeleteDialog"
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
        v-if="shortcutSheetVisible && currentView === 'main'"
        @close="shortcutSheetVisible = false"
      />

      <div v-if="toast" class="pointer-events-none absolute bottom-3 left-1/2 z-40 -translate-x-1/2 rounded-[var(--radius-btn)] bg-[#212529] px-3 py-1.5 text-[var(--font-size-sm)] text-white">
        {{ toast }}
      </div>
    </section>
  </main>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { LogicalSize } from '@tauri-apps/api/dpi';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import ConfirmDialog from './components/ConfirmDialog.vue';
import OnboardingBar from './components/OnboardingBar.vue';
import Settings from './components/Settings.vue';
import ShortcutSheet from './components/ShortcutSheet.vue';
import StatsBar from './components/StatsBar.vue';
import StatusBar from './components/StatusBar.vue';
import TaskList from './components/TaskList.vue';
import TitleBar from './components/TitleBar.vue';
import Welcome from './components/Welcome.vue';
import { useTaskStore } from './stores/taskStore';
import type { TaskFilter } from './stores/taskStore';
import type { Task } from './types';
import { initializeTheme, toggleThemeQuickly, useThemeState } from './utils/theme';

type ViewType = 'welcome' | 'main' | 'settings';

interface WindowStatePayload {
  mini_mode: boolean;
  always_on_top: boolean;
}

interface WindowSizePayload {
  width: number;
  height: number;
}

const taskStore = useTaskStore();
const appWindow = getCurrentWindow();

const currentView = ref<ViewType>('main');
const isMiniMode = ref(false);
const isAlwaysOnTop = ref(true);
const createInlineVisible = ref(false);
const shortcutSheetVisible = ref(false);
const miniPressed = ref(false);
const miniDragging = ref(false);

let miniStartPoint: { x: number; y: number } | null = null;
let miniMouseMoveHandler: ((event: MouseEvent) => void) | null = null;
let miniMouseUpHandler: (() => void) | null = null;

const toast = ref('');
let toastTimer: ReturnType<typeof setTimeout> | null = null;
let resizeTimer: ReturnType<typeof setTimeout> | null = null;
let unlistenResized: (() => void) | null = null;

const deleteDialogVisible = ref(false);
const pendingDeleteTask = ref<Task | null>(null);
const taskListRef = ref<any>(null);
const settingsRef = ref<any>(null);

const ONBOARDING_KEY = 'topdo_onboarding_v1_dismissed';
const SHORTCUT_TIP_KEY = 'topdo_shortcut_tip_seen_v1';
const onboardingPendingFromFirstLaunch = ref(false);
const showOnboarding = ref(false);

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

async function syncWindowState() {
  try {
    const state = await invoke<WindowStatePayload>('get_window_state');
    isMiniMode.value = state.mini_mode;
    isAlwaysOnTop.value = state.always_on_top;
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
    await invoke('enter_mini_mode');
    isMiniMode.value = true;
    isAlwaysOnTop.value = true;
  } catch (error) {
    showError(String(error));
  }
}

async function restoreNormalMode() {
  try {
    await invoke('restore_normal_mode');
    isMiniMode.value = false;
  } catch (error) {
    showError(String(error));
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
    void appWindow.startDragging().catch((error) => {
      showError(String(error));
    });
  };

  miniMouseUpHandler = () => {
    const shouldRestore = !miniDragging.value;
    clearMiniDragListeners();
    miniStartPoint = null;
    if (shouldRestore) {
      void restoreNormalMode();
    }
    window.setTimeout(() => {
      miniPressed.value = false;
      miniDragging.value = false;
    }, 60);
  };

  window.addEventListener('mousemove', miniMouseMoveHandler);
  window.addEventListener('mouseup', miniMouseUpHandler);
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
  if (document.visibilityState === 'visible' && taskStore.mode === 'feishu' && currentView.value === 'main') {
    void taskStore.triggerSync().catch((error) => showError(String(error)));
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
      createInlineVisible.value = false;
      return;
    }
    return;
  }

  if (isMeta && key === 'n') {
    event.preventDefault();
    if (currentView.value !== 'main') return;
    createInlineVisible.value = true;
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
  await syncWindowState();
  try {
    const savedSize = await invoke<WindowSizePayload | null>('get_window_size');
    if (savedSize && savedSize.width > 0 && savedSize.height > 0) {
      await appWindow.setSize(new LogicalSize(savedSize.width, savedSize.height));
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
        await invoke('save_window_size', {
          width: size.width,
          height: size.height
        });
      } catch (error) {
        console.warn('保存窗口尺寸失败:', error);
      }
    }, 500);
  });

  await bootstrap();
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
  if (resizeTimer) {
    clearTimeout(resizeTimer);
    resizeTimer = null;
  }
  if (toastTimer) {
    clearTimeout(toastTimer);
    toastTimer = null;
  }
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
</script>

<style scoped>
.app-container {
  width: 100%;
  height: 100%;
  background: var(--bg-primary);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border-radius: 12px;
  border: 0.5px solid var(--border-light);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  box-shadow:
    var(--shadow-md),
    var(--shadow-sm);
}

.mini-shell {
  margin: 2px;
  height: calc(100% - 4px);
  width: calc(100% - 4px);
  border-radius: 999px;
  border: 0.5px solid color-mix(in srgb, var(--border) 82%, transparent);
  background: color-mix(in srgb, var(--bg-solid) 94%, transparent);
  box-shadow: var(--shadow-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  cursor: grab;
  user-select: none;
}

.mini-shell.pressed {
  background: color-mix(in srgb, var(--primary) 9%, var(--bg-solid));
}

.mini-shell.dragging {
  cursor: grabbing;
}

.mini-dot {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background: var(--status-pending);
}

.mini-content {
  display: inline-flex;
  align-items: baseline;
  gap: 4px;
}

.mini-count {
  font-size: 26px;
  line-height: 1;
  font-weight: 700;
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.mini-label {
  font-size: 11px;
  line-height: 1;
  color: var(--text-secondary);
}
</style>
