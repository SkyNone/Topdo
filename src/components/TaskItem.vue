<template>
  <div class="list-none">
    <article
      class="task-card"
      :class="[
        completed ? 'completed' : '',
        statusAnimating ? 'scale-[1.01]' : 'scale-100',
        focused ? 'focused' : ''
      ]"
      :data-task-id="task.record_id"
      @contextmenu.prevent="openContextMenu"
      @click="emit('focus', task.record_id)"
    >
      <div v-if="priorityColor" class="priority-bar" :style="{ background: priorityColor }"></div>

      <button
        type="button"
        class="task-checkbox"
        :class="checkboxClass"
        :title="`切换状态: ${displayStatus(task.status)}`"
        @click.stop="onToggleStatus"
      >
        <svg v-if="statusKey === 'completed'" viewBox="0 0 12 12" width="10" height="10" aria-hidden="true">
          <path
            d="M2.5 6l2.5 2.5 4.5-5"
            fill="none"
            stroke="white"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <svg v-else-if="statusKey === 'in_progress'" class="progress-wave-icon" viewBox="0 0 18 18" aria-hidden="true">
          <defs>
            <clipPath id="progress-wave-clip">
              <circle cx="9" cy="9" r="8" />
            </clipPath>
            <linearGradient id="progress-wave-base" x1="0" y1="8.5" x2="0" y2="18" gradientUnits="userSpaceOnUse">
              <stop offset="0%" stop-color="color-mix(in srgb, var(--primary) 85%, white)" />
              <stop offset="100%" stop-color="var(--primary)" />
            </linearGradient>
          </defs>
          <g clip-path="url(#progress-wave-clip)">
            <rect x="1" y="9" width="16" height="8" fill="url(#progress-wave-base)" />
            <path
              class="progress-wave back"
              d="M-7 9 C-5 7.8 -3 10.2 -1 9 C1 7.8 3 10.2 5 9 C7 7.8 9 10.2 11 9 C13 7.8 15 10.2 17 9 C19 7.8 21 10.2 23 9 V18 H-7 Z"
            />
            <path
              class="progress-wave front"
              d="M-7 9 C-5 8.1 -3 9.9 -1 9 C1 8.1 3 9.9 5 9 C7 8.1 9 9.9 11 9 C13 8.1 15 9.9 17 9 C19 8.1 21 9.9 23 9 V18 H-7 Z"
            />
            <path
              class="progress-wave-sheen"
              d="M1 8.65 C3.2 8 5.3 9.3 7.4 8.65 C9.3 8.05 11.2 9.25 13.1 8.7 C14.5 8.3 15.8 8.9 17 8.8"
            />
          </g>
        </svg>
      </button>

      <button
        type="button"
        class="task-content text-left"
        @click="onTaskContentClick"
        @dblclick.stop.prevent="startInlineEdit"
      >
        <input
          v-if="inlineEditing"
          ref="inlineNameInputRef"
          v-model.trim="inlineNameDraft"
          type="text"
          class="task-name-inline-input"
          placeholder="请输入任务名称"
          @click.stop
          @blur="commitInlineEdit"
          @keydown.enter.prevent="commitInlineEdit"
          @keydown.esc.prevent="cancelInlineEdit"
        />
        <p v-else class="task-name" :title="task.name || '未命名任务'">
          <template v-for="(segment, index) in highlightedNameSegments" :key="`${segment.text}-${index}`">
            <mark v-if="segment.match" class="task-name-highlight">{{ segment.text }}</mark>
            <span v-else>{{ segment.text }}</span>
          </template>
        </p>
      </button>

      <div class="task-right">
        <span v-if="mode === 'feishu' && task.sync_status === 'pending'" class="sync-badge pending">待同步</span>
        <button
          v-if="mode === 'feishu' && task.sync_status === 'failed'"
          type="button"
          class="sync-badge failed"
          :title="task.last_error || '同步失败，点击重试'"
          @click.stop="onRetrySync"
        >
          重试
        </button>
        <span class="task-time">{{ formatTime(task.created_at) }}</span>
      </div>
    </article>

    <Transition name="context-menu">
      <div
        v-if="menuVisible"
        class="task-context-menu"
        :style="{ left: `${menuX}px`, top: `${menuY}px` }"
      >
        <button type="button" class="menu-item danger" @click.stop="onContextDelete">
          删除任务
        </button>
      </div>
    </Transition>

    <Transition name="expand">
      <div v-if="expanded" class="mx-[10px] mt-1 rounded-[var(--radius-card)] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-3 text-xs text-[color:var(--text-secondary)]">
        <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_120px]">
          <label class="block">
            <span class="mb-1 block">任务名称</span>
            <input
              v-model.trim="nameDraft"
              type="text"
              class="task-edit-field"
              placeholder="请输入任务名称"
              @blur="onNameCommit"
              @keydown.enter.prevent="onNameCommit"
            />
            <div class="mt-1 text-[10px] text-[color:var(--text-tertiary)]">
              <span v-if="nameSaving">保存中</span>
              <span v-else>回车或失焦保存</span>
            </div>
          </label>

          <label class="block">
            <span class="mb-1 block">重要性</span>
            <div class="priority-options mt-1">
              <button
                v-for="option in priorityOptions"
                :key="option.value"
                type="button"
                class="priority-btn"
                :class="[
                  `priority-btn--${option.tone}`,
                  { active: priorityDraft === option.value }
                ]"
                @click="onPrioritySelect(option.value)"
              >
                <span class="priority-dot" :style="{ background: option.color }"></span>
                {{ option.label }}
              </button>
            </div>
            <div class="mt-1 text-[10px] text-[color:var(--text-tertiary)]">
              <span v-if="prioritySaving">保存中</span>
              <span v-else>修改后立即保存</span>
            </div>
          </label>
        </div>

        <label class="block">
          <span class="mb-1 block">备注/收获</span>
          <textarea
            v-model="notesDraft"
            class="mt-1 min-h-14 w-full resize-y rounded-[var(--radius-btn)] border border-[color:var(--border)] bg-[color:var(--bg-secondary)] px-2.5 py-2 text-xs text-[color:var(--text-primary)] outline-none placeholder:text-[color:var(--text-placeholder)] focus:border-[color:var(--primary)]"
            @input="onNotesInput"
          ></textarea>
          <div class="mt-1 flex items-center justify-between text-[10px] text-[color:var(--text-tertiary)]">
            <span>{{ notesDraft.length }} 字</span>
            <span>
              <span v-if="notesSync === 'loading'">保存中</span>
              <span v-else-if="notesSync === 'pending'">待同步</span>
              <span v-else-if="notesSync === 'success'" class="text-[color:var(--status-done)]">已保存</span>
              <span v-else-if="notesSync === 'error'" class="text-[#FF8A80]">保存失败</span>
            </span>
          </div>
        </label>

        <div class="mt-2 space-y-1 text-[11px] text-[color:var(--text-secondary)]">
          <p>创建时间：{{ formatDate(task.created_at) }}</p>
          <p>实际耗时：{{ task.time_spent || '—' }}</p>
          <p v-if="mode === 'feishu' && task.retry_count">重试次数：{{ task.retry_count }}</p>
          <p v-if="mode === 'feishu' && task.last_error">最近错误：{{ task.last_error }}</p>
        </div>

        <div v-if="mode === 'local'" class="mt-2">
          <button type="button" class="text-[11px] text-[#FF8A80] underline" @click="$emit('request-delete', task)">删除任务</button>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useTaskStore } from '../stores/taskStore';
import type { SyncState } from '../stores/taskStore';
import type { Task } from '../types';

const props = defineProps<{
  task: Task;
  mode: 'local' | 'feishu';
  statusSync: SyncState;
  notesSync: SyncState;
  focused?: boolean;
}>();

const emit = defineEmits<{
  (event: 'error', message: string): void;
  (event: 'request-delete', task: Task): void;
  (event: 'focus', recordId: string): void;
}>();

const store = useTaskStore();
const expanded = ref(false);
const nameDraft = ref(props.task.name || '');
const inlineNameDraft = ref(props.task.name || '');
const inlineEditing = ref(false);
const inlineNameInputRef = ref<HTMLInputElement | null>(null);
const notesDraft = ref(props.task.notes || '');
const priorityDraft = ref(normalizePriorityDraft(props.task.priority));
const statusAnimating = ref(false);
const menuVisible = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const nameSaving = ref(false);
const prioritySaving = ref(false);
let notesTimer: ReturnType<typeof setTimeout> | null = null;
const priorityOptions = [
  { value: '普通', label: '普通', color: '#C7C7CC', tone: 'normal' },
  { value: '重要', label: '重要', color: '#007AFF', tone: 'important' },
  { value: '紧急', label: '紧急', color: '#FF3B30', tone: 'urgent' }
];
const searchQueryTrimmed = computed(() => store.searchQuery.trim());

const highlightedNameSegments = computed(() => {
  const source = props.task.name || '未命名任务';
  const query = searchQueryTrimmed.value;
  if (!query) return [{ text: source, match: false }];

  const lowerSource = source.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const segments: Array<{ text: string; match: boolean }> = [];
  let cursor = 0;

  while (cursor < source.length) {
    const nextIndex = lowerSource.indexOf(lowerQuery, cursor);
    if (nextIndex === -1) {
      segments.push({ text: source.slice(cursor), match: false });
      break;
    }
    if (nextIndex > cursor) {
      segments.push({ text: source.slice(cursor, nextIndex), match: false });
    }
    segments.push({ text: source.slice(nextIndex, nextIndex + query.length), match: true });
    cursor = nextIndex + query.length;
  }

  return segments.length ? segments : [{ text: source, match: false }];
});

function normalizePriorityDraft(priority: string | undefined): string {
  const value = (priority || '').trim();
  if (!value) return '普通';
  if (['紧急', '🔴今日必做', '🔴 今日必做', '今日必做'].includes(value)) return '紧急';
  if (['重要', '本周完成', '🟡本周完成', '🟠本周完成', '🔵本周完成', '🟡尽快完成', '🟡重要不紧急'].includes(value)) return '重要';
  return '普通';
}

watch(
  () => props.task.notes,
  (next) => {
    if (next !== notesDraft.value) {
      notesDraft.value = next || '';
    }
  }
);

watch(
  () => props.task.name,
  (next) => {
    if (next !== nameDraft.value) {
      nameDraft.value = next || '';
    }
    if (!inlineEditing.value && next !== inlineNameDraft.value) {
      inlineNameDraft.value = next || '';
    }
  }
);

watch(
  () => props.task.priority,
  (next) => {
    const normalized = normalizePriorityDraft(next);
    if (normalized !== priorityDraft.value) {
      priorityDraft.value = normalized;
    }
  }
);

onBeforeUnmount(() => {
  if (notesTimer) clearTimeout(notesTimer);
  document.removeEventListener('click', closeContextMenu);
  document.removeEventListener('keydown', onGlobalKeydown);
});

onMounted(() => {
  document.addEventListener('click', closeContextMenu);
  document.addEventListener('keydown', onGlobalKeydown);
});

const statusKey = computed(() => {
  const value = props.task.status.trim();
  if (value.includes('进行中')) return 'in_progress';
  if (value.includes('已完成')) return 'completed';
  if (value.includes('待处理') || value.includes('待办') || value.includes('待启动')) return 'todo';
  return 'unknown';
});

const completed = computed(() => statusKey.value === 'completed');

const priorityColor = computed(() => {
  const p = (props.task.priority || '').trim();
  const map: Record<string, string> = {
    '紧急': '#FF3B30',
    '重要': '#007AFF',
    '普通': '',
    '今日必做': '#FF3B30',
    '本周完成': '#007AFF',
    '自由安排': '',
    '🔴 今日必做': '#FF3B30',
    '🔴本周完成': '#007AFF',
    '🔵本周完成': '#007AFF',
    '🟡本周完成': '#007AFF',
    '🟠本周完成': '#007AFF',
    '⚪️自由安排': '',
    '⚪自由安排': '',
    '🔴今日必做': '#FF3B30',
    '🟡尽快完成': '#007AFF',
    '🟡重要不紧急': '#007AFF',
    '🔵有空再说': '',
    '🔵常规任务': ''
  };
  return map[p] || '';
});

const checkboxClass = computed(() => {
  switch (displayStatus(props.task.status)) {
    case '已完成':
      return 'checked';
    case '进行中':
      return 'in-progress';
    default:
      return '';
  }
});

function displayStatus(status: string): string {
  const value = status.trim();
  if (value.includes('进行中')) return '进行中';
  if (value.includes('已完成')) return '已完成';
  if (value.includes('待处理') || value.includes('待办') || value.includes('待启动')) return '待启动';
  return status || '待启动';
}

function nextStatus(status: string): string {
  const display = displayStatus(status);
  if (display === '待启动') return '进行中';
  if (display === '进行中') return '已完成';
  return '待处理';
}

function formatDate(input: string): string {
  const value = (input || '').trim();
  if (!value) return '—';
  const num = Number(value);
  if (Number.isFinite(num)) {
    const ms = num > 1e12 ? num : num * 1000;
    const date = new Date(ms);
    if (!Number.isNaN(date.getTime())) {
      return new Intl.DateTimeFormat('zh-CN', {
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit'
      }).format(date);
    }
  }
  const parsed = new Date(value);
  if (!Number.isNaN(parsed.getTime())) {
    return new Intl.DateTimeFormat('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    }).format(parsed);
  }
  return value;
}

function formatTime(input: string): string {
  const raw = (input || '').trim();
  if (!raw) return '--';

  const asNum = Number(raw);
  const date = Number.isFinite(asNum)
    ? new Date(asNum > 1e12 ? asNum : asNum * 1000)
    : new Date(raw);

  if (Number.isNaN(date.getTime())) return '--';

  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(today.getTime() - 86400000);
  const taskDate = new Date(date.getFullYear(), date.getMonth(), date.getDate());

  if (taskDate.getTime() === today.getTime()) {
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false });
  }
  if (taskDate.getTime() === yesterday.getTime()) {
    return '昨天';
  }
  return `${String(date.getMonth() + 1).padStart(2, '0')}/${String(date.getDate()).padStart(2, '0')}`;
}

async function onToggleStatus() {
  const target = nextStatus(props.task.status);
  statusAnimating.value = true;
  setTimeout(() => {
    statusAnimating.value = false;
  }, 200);

  try {
    await store.updateTaskStatus(props.task.record_id, target);
  } catch (error) {
    emit('error', `状态更新失败：${String(error)}`);
  }
}

async function onRetrySync() {
  try {
    await store.triggerSync();
  } catch (error) {
    emit('error', `重试同步失败：${String(error)}`);
  }
}

async function onNameCommit() {
  const trimmed = nameDraft.value.trim();
  const current = (props.task.name || '').trim();
  if (!trimmed) {
    nameDraft.value = current;
    emit('error', '任务名称不能为空');
    return;
  }
  if (trimmed === current) return;

  nameSaving.value = true;
  try {
    await store.updateTaskName(props.task.record_id, trimmed);
  } catch (error) {
    nameDraft.value = current;
    emit('error', `任务名称保存失败：${String(error)}`);
  } finally {
    nameSaving.value = false;
  }
}

function onTaskContentClick() {
  if (inlineEditing.value) return;
  expanded.value = !expanded.value;
}

function startInlineEdit() {
  if (nameSaving.value) return;
  inlineNameDraft.value = props.task.name || '';
  inlineEditing.value = true;
  void nextTick(() => {
    inlineNameInputRef.value?.focus();
    inlineNameInputRef.value?.select();
  });
}

function cancelInlineEdit() {
  inlineNameDraft.value = props.task.name || '';
  inlineEditing.value = false;
}

async function commitInlineEdit() {
  const trimmed = inlineNameDraft.value.trim();
  const current = (props.task.name || '').trim();

  if (!trimmed) {
    inlineNameDraft.value = current;
    inlineEditing.value = false;
    emit('error', '任务名称不能为空');
    return;
  }

  if (trimmed === current) {
    inlineEditing.value = false;
    return;
  }

  nameSaving.value = true;
  try {
    await store.updateTaskName(props.task.record_id, trimmed);
    inlineNameDraft.value = trimmed;
    nameDraft.value = trimmed;
    inlineEditing.value = false;
  } catch (error) {
    inlineNameDraft.value = current;
    inlineEditing.value = false;
    emit('error', `任务名称保存失败：${String(error)}`);
  } finally {
    nameSaving.value = false;
  }
}

async function onPriorityCommit() {
  const next = (priorityDraft.value || '普通').trim() || '普通';
  const current = normalizePriorityDraft(props.task.priority);
  if (next === current) return;

  prioritySaving.value = true;
  try {
    await store.updateTaskPriority(props.task.record_id, next);
  } catch (error) {
    priorityDraft.value = current;
    emit('error', `重要性保存失败：${String(error)}`);
  } finally {
    prioritySaving.value = false;
  }
}

function onPrioritySelect(priority: string) {
  if (prioritySaving.value || priorityDraft.value === priority) return;
  priorityDraft.value = priority;
  void onPriorityCommit();
}

function onNotesInput() {
  if (notesTimer) clearTimeout(notesTimer);
  notesTimer = setTimeout(async () => {
    try {
      await store.updateTaskNotes(props.task.record_id, notesDraft.value);
    } catch (error) {
      notesDraft.value = props.task.notes || '';
      emit('error', `备注保存失败：${String(error)}`);
    }
  }, 500);
}

function openContextMenu(event: MouseEvent) {
  menuVisible.value = false;
  menuX.value = event.clientX;
  menuY.value = event.clientY;
  menuVisible.value = true;
}

function closeContextMenu() {
  menuVisible.value = false;
}

function onGlobalKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    closeContextMenu();
  }
}

function onContextDelete() {
  closeContextMenu();
  emit('request-delete', props.task);
}

function toggleExpandFromKeyboard() {
  expanded.value = !expanded.value;
}

async function toggleStatusFromKeyboard() {
  await onToggleStatus();
}

function requestDeleteFromKeyboard() {
  emit('request-delete', props.task);
}

defineExpose({
  toggleExpandFromKeyboard,
  toggleStatusFromKeyboard,
  requestDeleteFromKeyboard
});
</script>

<style scoped>
.task-card {
  margin: 8px 10px;
  padding: 10px 12px 10px 16px;
  background: var(--bg-solid, #ffffff);
  border-radius: var(--radius-card, 8px);
  border: 0.5px solid var(--border, #e5e5ea);
  box-shadow: var(--shadow-sm, 0 0.5px 2px rgba(0, 0, 0, 0.04));
  display: flex;
  align-items: center;
  gap: 10px;
  transition: all 0.15s ease;
  position: relative;
  cursor: default;
  height: 44px;
  min-height: 44px;
}

.task-card:hover {
  background: var(--bg-hover);
  box-shadow: var(--shadow-md, 0 2px 8px rgba(0, 0, 0, 0.08));
}

.task-card.focused {
  border-color: color-mix(in srgb, var(--border) 70%, var(--primary) 30%);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--primary) 18%, transparent);
}

.task-card.completed {
  opacity: 0.5;
}

.task-card.completed .task-name {
  text-decoration: line-through;
  color: var(--text-tertiary, #aeaeb2);
}

.priority-bar {
  position: absolute;
  left: 0;
  top: 10px;
  bottom: 10px;
  width: 3px;
  border-radius: 0 2px 2px 0;
}

.task-checkbox {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 1.5px solid #c7c7cc;
  background: white;
  flex-shrink: 0;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.task-checkbox:hover {
  border-color: var(--primary);
  background: var(--primary-light);
}

.task-checkbox.in-progress {
  border-color: var(--primary);
  background: #fff;
  overflow: hidden;
}

.progress-wave-icon {
  width: 100%;
  height: 100%;
  display: block;
}

.progress-wave {
  transform-box: fill-box;
  transform-origin: center;
  will-change: transform;
}

.progress-wave.front {
  fill: color-mix(in srgb, var(--primary) 92%, white);
  opacity: 0.94;
  animation: wave-front-float 1.8s ease-in-out infinite;
}

.progress-wave.back {
  fill: color-mix(in srgb, var(--primary) 70%, white);
  opacity: 0.56;
  animation: wave-back-float 2.6s ease-in-out infinite;
}

.progress-wave-sheen {
  fill: none;
  stroke: color-mix(in srgb, white 80%, var(--primary));
  stroke-width: 0.7;
  stroke-linecap: round;
  opacity: 0.75;
}

@keyframes wave-front-float {
  0% {
    transform: translate(0, 0);
  }
  50% {
    transform: translate(-1.15px, 0.35px);
  }
  100% {
    transform: translate(0, 0);
  }
}

@keyframes wave-back-float {
  0% {
    transform: translate(0.2px, 0.15px);
  }
  50% {
    transform: translate(1px, -0.2px);
  }
  100% {
    transform: translate(0.2px, 0.15px);
  }
}

.task-checkbox.checked {
  border-color: #34c759;
  background: #34c759;
}

.task-content {
  flex: 1;
  min-width: 0;
}

.task-name {
  font-size: var(--font-size-base, 13px);
  font-weight: 400;
  color: var(--text-primary, #1d1d1f);
  line-height: 1.4;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-name-highlight {
  padding: 0 2px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--primary) 14%, transparent);
  color: color-mix(in srgb, var(--primary) 76%, var(--text-primary));
}

.task-name-inline-input {
  width: 100%;
  min-width: 0;
  border: 1px solid color-mix(in srgb, var(--primary) 35%, var(--border));
  border-radius: 8px;
  background: var(--bg-secondary, #f5f5f7);
  padding: 4px 8px;
  font-size: var(--font-size-base, 13px);
  font-weight: 400;
  color: var(--text-primary, #1d1d1f);
  line-height: 1.4;
  outline: none;
}

.task-edit-field {
  width: 100%;
  border-radius: var(--radius-btn);
  border: 1px solid var(--border, #e5e5ea);
  background: var(--bg-secondary, #f5f5f7);
  padding: 8px 10px;
  font-size: 12px;
  color: var(--text-primary, #1d1d1f);
  outline: none;
}

.task-edit-field:focus {
  border-color: var(--primary);
}

.priority-options {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.priority-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 10px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-tag);
  cursor: pointer;
  transition: all 0.15s ease;
}

.priority-btn:hover {
  background: var(--bg-hover);
  border-color: #c7c7cc;
}

.priority-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 0.5px rgba(0, 0, 0, 0.05);
}

.priority-btn--normal.active {
  color: #4b5563;
  background: #eef2f6;
  border-color: #b7c0cc;
}

.priority-btn--important.active {
  color: #0b63ce;
  background: rgba(0, 122, 255, 0.12);
  border-color: rgba(0, 122, 255, 0.28);
}

.priority-btn--urgent.active {
  color: #d93025;
  background: rgba(255, 59, 48, 0.12);
  border-color: rgba(255, 59, 48, 0.24);
}

.task-right {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.task-time {
  font-size: 10px;
  color: var(--text-tertiary, #aeaeb2);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.task-context-menu {
  position: fixed;
  z-index: 80;
  min-width: 120px;
  border-radius: 8px;
  border: 0.5px solid var(--border, #e5e5ea);
  background: var(--bg-solid);
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.12);
  padding: 4px;
}

.menu-item {
  width: 100%;
  border: none;
  background: transparent;
  border-radius: 6px;
  padding: 6px 10px;
  text-align: left;
  font-size: 12px;
  color: var(--text-primary, #1d1d1f);
}

.menu-item:hover {
  background: var(--bg-hover);
}

.menu-item.danger {
  color: #ff8a80;
}

.context-menu-enter-active,
.context-menu-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}

.context-menu-enter-from,
.context-menu-leave-to {
  opacity: 0;
  transform: translateY(-2px);
}

.sync-badge {
  border: none;
  border-radius: 8px;
  padding: 0 6px;
  height: 16px;
  line-height: 16px;
  font-size: 10px;
  color: var(--text-secondary);
  background: var(--bg-secondary);
}

.sync-badge.pending {
  color: var(--status-pending);
  background: color-mix(in srgb, var(--status-pending) 18%, transparent);
}

.sync-badge.failed {
  color: #ff8a80;
  background: color-mix(in srgb, #ff453a 22%, transparent);
  cursor: pointer;
}
</style>
