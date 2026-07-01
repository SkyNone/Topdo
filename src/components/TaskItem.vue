<template>
  <div ref="taskItemRootRef" class="list-none">
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
        ref="checkboxRef"
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
          @blur="handleInlineEditBlur"
          @compositionstart="isInlineNameComposing = true"
          @compositionend="isInlineNameComposing = false"
          @keydown.enter="onInlineNameEnter"
          @keydown.esc.prevent="cancelInlineEdit"
        />
        <p v-else class="task-name" :title="task.name || '未命名任务'">
          <template v-for="(segment, index) in highlightedNameSegments" :key="`${segment.text}-${index}`">
            <mark v-if="segment.match" class="task-name-highlight">{{ segment.text }}</mark>
            <span v-else>{{ segment.text }}</span>
          </template>
        </p>
        <div v-if="!inlineEditing && (dueDateInfo || recurrenceText || subTaskTotal > 0 || taskTags.length)" class="task-meta-line">
          <span v-if="recurrenceText" class="badge badge-recurring">
            <Icon name="recurring" :size="11" />
            <span>{{ recurrenceText }}</span>
          </span>
          <span v-if="dueDateInfo" class="badge" :class="dueDateInfo.tone === 'overdue' ? 'badge-overdue' : 'badge-due'">{{ dueDateInfo.label }}</span>
          <span v-if="subTaskTotal > 0" class="subtask-progress">{{ subTaskDone }}/{{ subTaskTotal }}</span>
          <span
            v-for="tag in taskTags.slice(0, 3)"
            :key="tag"
            class="task-tag"
          >
            {{ tag }}
          </span>
          <span v-if="taskTags.length > 3" class="task-tag task-tag-more">+{{ taskTags.length - 3 }}</span>
        </div>
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
    <div v-if="inlineEditing" class="inline-edit-hint">回车保存 · Esc 取消</div>

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
      <div v-if="expanded" class="task-detail-panel" @click.stop>
        <header class="edit-header">
          <button type="button" class="back-btn" @click="handleBack">
            <Icon name="chevron-left" :size="16" />
            返回
          </button>
          <span class="edit-title">任务详情</span>
          <div class="detail-more-wrap">
            <button type="button" class="edit-action-btn" @click.stop="saveDetailDrafts">
              保存
            </button>
          </div>
        </header>

        <div class="task-status-bar">
          <span class="detail-badge" :class="statusBadgeClass">{{ statusLabel }}</span>
          <span v-if="isOverdue" class="detail-badge detail-badge--overdue">已过期</span>
          <span class="detail-badge" :class="priorityBadgeClass">{{ priorityDraft }}</span>
        </div>

        <div class="detail-body task-scrollbar">
          <div class="task-input-area">
            <input
              v-model.trim="nameDraft"
              class="detail-name-input"
              placeholder="请输入任务名称"
              @blur="handleNameBlur"
              @compositionstart="isDetailNameComposing = true"
              @compositionend="isDetailNameComposing = false"
              @keydown.enter="onDetailNameEnter"
              @keydown.esc.prevent="cancelNameEdit"
            />
            <div class="input-divider" aria-hidden="true"></div>
          </div>

          <section class="detail-option-row detail-option-row--stack" @click="startDateEdit">
            <div class="detail-option-icon detail-option-icon--green"><Icon name="calendar" :size="17" /></div>
            <div class="detail-option-content">
              <span class="detail-option-label">截止时间</span>
              <div v-if="!isEditingDate" class="date-time-display">
                <span class="date-badge">{{ formattedDueDate }}</span>
                <span class="time-badge">{{ formattedDueTime }}</span>
              </div>
              <div v-else class="date-edit-block" @click.stop @focusout="onDateEditFocusOut">
                <ChipSelector :model-value="selectedDateOption" :options="dateOptions" tone="blue" compact @update:model-value="onDateOptionUpdate" />
                <div v-if="showCustomDueDateInput" class="date-edit-inputs">
                  <span>日期</span>
                  <input v-model="dueDateDraft" type="date" @change="onDueDateChange" />
                </div>
                <div class="date-edit-inputs">
                  <span>时间</span>
                  <input v-model="dueTimeDraft" type="time" :disabled="!dueDateDraft" @change="onDueDateChange" />
                  <small>{{ dueDateDraft ? '默认 23:59' : '先选日期' }}</small>
                </div>
              </div>
            </div>
          </section>

          <section class="detail-option-row">
            <div class="detail-option-icon detail-option-icon--blue"><Icon name="priority" :size="17" /></div>
            <div class="detail-option-content">
              <span class="detail-option-label">优先级</span>
              <ChipSelector v-model="priorityDraft" :options="priorityChipOptions" :tone="priorityChipTone" @update:model-value="onPriorityChipUpdate" />
            </div>
          </section>

          <section class="detail-option-row detail-option-row--stack">
            <div class="detail-option-icon detail-option-icon--cyan"><Icon name="tag" :size="17" /></div>
            <div class="detail-option-content">
              <span class="detail-option-label">标签</span>
              <div class="detail-tags">
                <button
                  v-for="tag in tagOptions"
                  :key="tag"
                  type="button"
                  class="detail-tag-chip"
                  :class="{ selected: tagsDraft.includes(tag) }"
                  @click="toggleTag(tag)"
                >
                  {{ tag }}
                </button>
                <p v-if="!tagOptions.length" class="tag-empty-hint">输入后会保留最近 5 个标签</p>
              </div>
              <div class="detail-tag-input-row">
                <input
                  v-model="tagDraft"
                  type="text"
                  placeholder="自定义标签，回车添加"
                  @compositionstart="isTagComposing = true"
                  @compositionend="isTagComposing = false"
                  @keydown.enter="onTagInputEnter"
                />
                <button type="button" @click="addTagDraft">添加</button>
              </div>
            </div>
          </section>

          <section class="detail-option-row detail-option-row--stack">
            <div class="detail-option-icon detail-option-icon--purple"><Icon name="recurring" :size="17" /></div>
            <div class="detail-option-content">
              <span class="detail-option-label">重复</span>
              <RecurrencePanel v-model="recurrenceDraft" embedded />
            </div>
          </section>

          <section class="detail-option-row detail-option-row--stack">
            <div class="detail-option-icon detail-option-icon--amber"><Icon name="bell" :size="17" /></div>
            <div class="detail-option-content">
              <span class="detail-option-label">提醒</span>
              <ReminderSelect v-model="reminderDraft" embedded />
            </div>
          </section>

          <section class="detail-option-row detail-option-row--stack">
            <div class="detail-option-icon detail-option-icon--slate"><Icon name="list" :size="17" /></div>
            <div class="detail-option-content">
              <span class="detail-option-label">子任务 ({{ subTaskDone }}/{{ subTaskTotal }})</span>
              <div class="detail-subtask-section">
                <div v-if="subTaskTotal > 0" class="detail-progress">
                  <div class="detail-progress-head">
                    <span>完成进度</span>
                    <span>{{ subTaskDone }}/{{ subTaskTotal }}</span>
                  </div>
                  <div class="detail-progress-bar">
                    <span :style="{ width: `${subTaskProgress}%` }"></span>
                  </div>
                </div>
                <ul v-if="subTasks.length" class="detail-subtask-list">
                  <li v-for="item in subTasks" :key="item.id" class="detail-subtask-item">
                    <button type="button" class="detail-subtask-checkbox" :class="{ checked: item.done }" @click="toggleSubTask(item.id)"></button>
                    <input
                      v-model="item.text"
                      class="detail-subtask-input"
                      :class="{ done: item.done }"
                      placeholder="子任务名称"
                      @input="markSubTasksDirty"
                      @change="normalizeSubTaskText(item.id)"
                      @compositionstart="isSubTaskComposing = true"
                      @compositionend="isSubTaskComposing = false"
                      @keydown.enter="onExistingSubTaskEnter($event, item.id)"
                    />
                    <button type="button" class="detail-subtask-delete" title="删除子任务" @click="deleteSubTask(item.id)">×</button>
                  </li>
                </ul>
                <div class="detail-subtask-add">
                  <span class="detail-subtask-add-icon">+</span>
                  <input
                    v-model.trim="newSubTaskText"
                    type="text"
                    placeholder="添加子任务，回车确认"
                    @compositionstart="isNewSubTaskComposing = true"
                    @compositionend="isNewSubTaskComposing = false"
                    @keydown.enter="onNewSubTaskEnter"
                  />
                </div>
              </div>
            </div>
          </section>

          <section class="detail-option-row detail-option-row--stack detail-option-row--no-border">
            <div class="detail-option-icon detail-option-icon--cyan"><Icon name="file-text" :size="17" /></div>
            <div class="detail-option-content">
              <span class="detail-option-label">备注/收获</span>
              <textarea
                v-model="notesDraft"
                class="detail-note-textarea"
                placeholder="添加备注或收获..."
                rows="3"
              ></textarea>
              <div class="note-footer">
                <span>{{ notesDraft.length }} 字</span>
                <span>
                  <span v-if="notesSync === 'loading'">保存中</span>
                  <span v-else-if="notesSync === 'pending'">待同步</span>
                  <span v-else-if="notesSync === 'success'" class="saved">已保存</span>
                  <span v-else-if="notesSync === 'error'" class="error">保存失败</span>
                </span>
              </div>
            </div>
          </section>

          <div class="detail-meta-info">
            <div>创建时间：{{ formatDate(task.created_at) }}</div>
            <div>实际耗时：{{ task.time_spent || '未完成，耗时 0 天' }}</div>
            <div v-if="mode === 'feishu' && task.retry_count">重试次数：{{ task.retry_count }}</div>
            <div v-if="mode === 'feishu' && task.last_error">最近错误：{{ task.last_error }}</div>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { useTaskStore } from '../stores/taskStore';
import type { SyncState } from '../stores/taskStore';
import type { RecurrenceRule, SubTask, Task } from '../types';
import { buildDueDateValue, formatDueDate as formatDueDateLabel, splitDueDate } from '../utils/dueDate';
import { isImeComposing } from '../utils/keyboard';
import { recurrenceLabel } from '../utils/recurrence';
import Icon from './Icon.vue';
import RecurrencePanel from './RecurrencePanel.vue';
import ReminderSelect from './ReminderSelect.vue';
import ChipSelector, { type ChipOption } from './ui/ChipSelector.vue';

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
  (event: 'completed', payload: { recordId: string; name: string; x: number; y: number }): void;
}>();

const store = useTaskStore();
const expanded = ref(false);
const nameDraft = ref(props.task.name || '');
const inlineNameDraft = ref(props.task.name || '');
const inlineEditing = ref(false);
const inlineNameInputRef = ref<HTMLInputElement | null>(null);
const taskItemRootRef = ref<HTMLElement | null>(null);
const checkboxRef = ref<HTMLButtonElement | null>(null);
const notesDraft = ref(props.task.notes || '');
const priorityDraft = ref(normalizePriorityDraft(props.task.priority));
const newSubTaskText = ref('');
const initialDueParts = splitDueDate(props.task.due_date);
const dueDateDraft = ref(initialDueParts.date);
const dueTimeDraft = ref(initialDueParts.time);
const recurrenceDraft = ref<RecurrenceRule | null>(props.task.recurrence_rule || null);
const reminderDraft = ref<number | null>(props.task.reminder_before ?? null);
const subTasksDraft = ref<SubTask[]>(cloneSubTasks(props.task.sub_tasks));
const tagsDraft = ref<string[]>(normalizeTags(props.task.tags));
const tagDraft = ref('');
const subTasksDirty = ref(false);
const tagsDirty = ref(false);
const isEditingDate = ref(false);
const selectedDateOption = ref<string | number | null>(initialDueParts.date || null);
const statusAnimating = ref(false);
const menuVisible = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const nameSaving = ref(false);
const isInlineNameComposing = ref(false);
const isDetailNameComposing = ref(false);
const isTagComposing = ref(false);
const isSubTaskComposing = ref(false);
const isNewSubTaskComposing = ref(false);
let inlineBlurTimer: ReturnType<typeof setTimeout> | null = null;
const priorityOptions = [
  { value: '普通', label: '普通', color: '#C7C7CC', tone: 'normal' },
  { value: '重要', label: '重要', color: '#007AFF', tone: 'important' },
  { value: '紧急', label: '紧急', color: '#FF3B30', tone: 'urgent' }
];
const priorityChipOptions: ChipOption[] = [
  { value: '普通', label: '普通', dot: 'var(--text-placeholder)' },
  { value: '重要', label: '重要', dot: 'var(--accent-blue)' },
  { value: '紧急', label: '紧急', dot: 'var(--accent-red)' }
];
const dateOptions: ChipOption[] = [
  { value: todayKey(), label: '今天' },
  { value: offsetDateKey(1), label: '明天' },
  { value: thisFridayKey(), label: '本周五' },
  { value: nextMondayKey(), label: '下周一' },
  { value: 'custom', label: '选择日期' }
];
const searchQueryTrimmed = computed(() => store.searchQuery.trim());
const tagOptions = computed(() => mergeTagOptions(store.recentTags, tagsDraft.value));
const subTasks = computed(() => subTasksDraft.value);
const taskTags = computed(() => normalizeTags(props.task.tags));
const subTaskTotal = computed(() => subTasks.value.length);
const subTaskDone = computed(() => subTasks.value.filter((item) => item.done).length);
const subTaskProgress = computed(() => subTaskTotal.value ? Math.round((subTaskDone.value / subTaskTotal.value) * 100) : 0);
const dueDateInfo = computed(() => formatDueDateLabel(props.task.due_date || ''));
const isOverdue = computed(() => dueDateInfo.value?.tone === 'overdue' && statusKey.value !== 'completed');
const recurrenceText = computed(() => recurrenceLabel(props.task.recurrence_rule));
const statusLabel = computed(() => displayStatus(props.task.status));
const statusDotClass = computed(() => {
  if (statusKey.value === 'completed') return 'done';
  if (statusKey.value === 'in_progress') return 'doing';
  return 'todo';
});
const priorityToneClass = computed(() => {
  if (priorityDraft.value === '紧急') return 'urgent';
  if (priorityDraft.value === '重要') return 'important';
  return 'normal';
});
const priorityChipTone = computed(() => priorityDraft.value === '紧急' ? 'red' : 'blue');
const statusBadgeClass = computed(() => {
  if (statusKey.value === 'completed') return 'detail-badge--done';
  if (statusKey.value === 'in_progress') return 'detail-badge--progress';
  return 'detail-badge--todo';
});
const priorityBadgeClass = computed(() => {
  if (priorityDraft.value === '紧急') return 'detail-badge--urgent';
  if (priorityDraft.value === '重要') return 'detail-badge--important';
  return 'detail-badge--normal';
});
const formattedDueDate = computed(() => {
  if (!dueDateDraft.value) return '无截止日期';
  const date = new Date(`${dueDateDraft.value}T00:00:00`);
  if (Number.isNaN(date.getTime())) return dueDateDraft.value;
  return `${date.getFullYear()}/${String(date.getMonth() + 1).padStart(2, '0')}/${String(date.getDate()).padStart(2, '0')}`;
});
const formattedDueTime = computed(() => dueTimeDraft.value || '23:59');
const showCustomDueDateInput = computed(() => {
  if (!dueDateDraft.value) return false;
  return selectedDateOption.value === 'custom' || !dateOptions.some((option) => option.value === dueDateDraft.value);
});

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

function dateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

function todayKey(): string {
  return dateKey(new Date());
}

function offsetDateKey(days: number): string {
  const date = new Date();
  date.setDate(date.getDate() + days);
  return dateKey(date);
}

function thisFridayKey(): string {
  const date = new Date();
  const day = date.getDay();
  const distance = (5 - day + 7) % 7;
  date.setDate(date.getDate() + distance);
  return dateKey(date);
}

function nextMondayKey(): string {
  const date = new Date();
  const day = date.getDay() || 7;
  date.setDate(date.getDate() + (8 - day));
  return dateKey(date);
}

function cloneSubTasks(value: SubTask[] | undefined): SubTask[] {
  return (value || []).map((item) => ({ ...item }));
}

function normalizeTags(value: unknown): string[] {
  const parsed = typeof value === 'string'
    ? (() => {
        try {
          return JSON.parse(value);
        } catch {
          return value.split(/[，,\s]+/);
        }
      })()
    : value;
  if (!Array.isArray(parsed)) return [];
  const seen = new Set<string>();
  return parsed
    .map((item) => String(item || '').trim().replace(/^#/, '').slice(0, 16))
    .filter(Boolean)
    .filter((item) => {
      const key = item.toLowerCase();
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .slice(0, 8);
}

function mergeTagOptions(...groups: unknown[]): string[] {
  const seen = new Set<string>();
  const options: string[] = [];
  for (const tag of groups.flatMap((group) => normalizeTags(group))) {
    const key = tag.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    options.push(tag);
    if (options.length >= 5) break;
  }
  return options;
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

watch(
  () => props.task.due_date,
  (next) => {
    const parts = splitDueDate(next);
    dueDateDraft.value = parts.date;
    dueTimeDraft.value = parts.time;
    selectedDateOption.value = parts.date || null;
  }
);

watch(
  () => props.task.recurrence_rule,
  (next) => {
    recurrenceDraft.value = next || null;
  },
  { deep: true }
);

watch(
  () => props.task.reminder_before,
  (next) => {
    reminderDraft.value = next ?? null;
  }
);

watch(
  () => props.task.sub_tasks,
  (next) => {
    if (subTasksDirty.value) return;
    subTasksDraft.value = cloneSubTasks(next);
  },
  { deep: true }
);

watch(
  () => props.task.tags,
  (next) => {
    if (tagsDirty.value) return;
    tagsDraft.value = normalizeTags(next);
  },
  { deep: true }
);

onBeforeUnmount(() => {
  if (inlineBlurTimer) clearTimeout(inlineBlurTimer);
  removeContextMenuListeners();
});

watch(menuVisible, (visible) => {
  if (visible) addContextMenuListeners();
  else removeContextMenuListeners();
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
  const fromStatusKey = statusKey.value;
  const target = nextStatus(props.task.status);
  const shouldCelebrate = fromStatusKey === 'in_progress' && displayStatus(target) === '已完成';
  const anchor = getCompletionAnchor();
  statusAnimating.value = true;
  setTimeout(() => {
    statusAnimating.value = false;
  }, 200);

  try {
    const updatePromise = store.updateTaskStatus(props.task.record_id, target);
    if (shouldCelebrate && anchor) {
      emit('completed', {
        recordId: props.task.record_id,
        name: props.task.name || '未命名任务',
        x: anchor.x,
        y: anchor.y
      });
    }
    await updatePromise;
  } catch (error) {
    emit('error', `状态更新失败：${String(error)}`);
  }
}

function getCompletionAnchor(): { x: number; y: number } | null {
  const rect = checkboxRef.value?.getBoundingClientRect();
  if (!rect) return null;
  return {
    x: rect.left + rect.width / 2,
    y: rect.top + rect.height / 2
  };
}

async function onRetrySync() {
  try {
    await store.triggerSync();
  } catch (error) {
    emit('error', `重试同步失败：${String(error)}`);
  }
}

function cancelNameEdit() {
  nameDraft.value = props.task.name || '';
}

function focusInsideTask(target: EventTarget | Element | null): boolean {
  if (!target || !(target instanceof Node)) return false;
  return Boolean(taskItemRootRef.value?.contains(target));
}

function handleNameBlur(event: FocusEvent) {
  if (focusInsideTask(event.relatedTarget)) return;
  if (!nameDraft.value.trim()) cancelNameEdit();
}

function onDetailNameEnter(event: KeyboardEvent) {
  if (isImeComposing(event, isDetailNameComposing.value)) return;
  event.preventDefault();
  void saveDetailDrafts();
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

function handleInlineEditBlur(event: FocusEvent) {
  if (focusInsideTask(event.relatedTarget)) return;
  if (inlineBlurTimer) clearTimeout(inlineBlurTimer);
  inlineBlurTimer = setTimeout(() => {
    if (focusInsideTask(document.activeElement)) return;
    if (!inlineNameDraft.value.trim()) {
      cancelInlineEdit();
      return;
    }
    void commitInlineEdit();
  }, 150);
}

async function commitInlineEdit() {
  const trimmed = inlineNameDraft.value.trim();
  const current = (props.task.name || '').trim();

  if (!trimmed) {
    cancelInlineEdit();
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

function onInlineNameEnter(event: KeyboardEvent) {
  if (isImeComposing(event, isInlineNameComposing.value)) return;
  event.preventDefault();
  void commitInlineEdit();
}

function onPrioritySelect(priority: string) {
  priorityDraft.value = priority;
}

function onPriorityChipUpdate(value: string | number | null) {
  if (typeof value === 'string') onPrioritySelect(value);
}

function addTag(value: string) {
  const tag = value.trim().replace(/^#/, '').slice(0, 16);
  if (!tag) return;
  if (tagsDraft.value.some((item) => item.toLowerCase() === tag.toLowerCase())) return;
  if (tagsDraft.value.length >= 5) return;
  tagsDraft.value = [...tagsDraft.value, tag].slice(0, 5);
  tagsDirty.value = true;
}

function addTagDraft() {
  tagDraft.value
    .split(/[，,\s]+/)
    .forEach((tag) => addTag(tag));
  tagDraft.value = '';
}

function toggleTag(tag: string) {
  if (tagsDraft.value.includes(tag)) {
    removeTag(tag);
    return;
  }
  addTag(tag);
}

function removeTag(tag: string) {
  tagsDraft.value = tagsDraft.value.filter((item) => item !== tag);
  tagsDirty.value = true;
}

function onTagInputEnter(event: KeyboardEvent) {
  if (isImeComposing(event, isTagComposing.value)) return;
  event.preventDefault();
  addTagDraft();
}

function onDueDateChange() {
  if (!dueDateDraft.value) {
    dueTimeDraft.value = '';
  }
  selectedDateOption.value = dueDateDraft.value || null;
}

function startDateEdit() {
  isEditingDate.value = true;
  if (dueDateDraft.value && !dueTimeDraft.value) {
    dueTimeDraft.value = '23:59';
  }
  selectedDateOption.value = dueDateDraft.value || null;
}

function onDateOptionUpdate(value: string | number | null) {
  if (value === 'custom') {
    selectedDateOption.value = 'custom';
    if (!dueDateDraft.value) dueDateDraft.value = todayKey();
    return;
  }
  if (typeof value === 'string') {
    selectedDateOption.value = value;
    dueDateDraft.value = value;
    if (!dueTimeDraft.value) dueTimeDraft.value = '23:59';
  }
}

function onDateEditFocusOut(event: FocusEvent) {
  const nextTarget = event.relatedTarget;
  if (nextTarget instanceof Node && taskItemRootRef.value?.contains(nextTarget)) return;
  isEditingDate.value = false;
}

function markSubTasksDirty() {
  subTasksDirty.value = true;
}

function addSubTask() {
  const text = newSubTaskText.value.trim();
  if (!text) return;
  const item: SubTask = {
    id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
    text,
    done: false,
    created_at: Math.floor(Date.now() / 1000).toString()
  };
  subTasksDraft.value = [...subTasks.value, item];
  subTasksDirty.value = true;
  newSubTaskText.value = '';
}

function toggleSubTask(id: string) {
  subTasksDraft.value = subTasks.value.map((item) => (item.id === id ? { ...item, done: !item.done } : item));
  subTasksDirty.value = true;
}

function normalizeSubTaskText(id: string) {
  const current = subTasks.value.find((item) => item.id === id);
  if (!current) return;
  const nextText = current.text.trim();
  if (!nextText) {
    deleteSubTask(id);
    return;
  }
  if (current.text !== nextText) {
    current.text = nextText;
  }
  subTasksDirty.value = true;
}

function onExistingSubTaskEnter(event: KeyboardEvent, id: string) {
  if (isImeComposing(event, isSubTaskComposing.value)) return;
  event.preventDefault();
  normalizeSubTaskText(id);
}

function onNewSubTaskEnter(event: KeyboardEvent) {
  if (isImeComposing(event, isNewSubTaskComposing.value)) return;
  event.preventDefault();
  addSubTask();
}

function deleteSubTask(id: string) {
  subTasksDraft.value = subTasks.value.filter((item) => item.id !== id);
  subTasksDirty.value = true;
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

function addContextMenuListeners() {
  document.addEventListener('click', closeContextMenu);
  document.addEventListener('keydown', onGlobalKeydown);
}

function removeContextMenuListeners() {
  document.removeEventListener('click', closeContextMenu);
  document.removeEventListener('keydown', onGlobalKeydown);
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

function handleBack() {
  expanded.value = false;
}

async function saveDetailDrafts() {
  try {
    addSubTask();
    addTagDraft();
    await store.updateTaskDetails(props.task.record_id, {
      name: nameDraft.value,
      priority: priorityDraft.value,
      due_date: buildDueDateValue(dueDateDraft.value, dueTimeDraft.value),
      recurrence_rule: recurrenceDraft.value,
      reminder_before: reminderDraft.value,
      sub_tasks: subTasks.value,
      tags: tagsDraft.value,
      notes: notesDraft.value
    });
    subTasksDraft.value = cloneSubTasks(subTasks.value);
    subTasksDirty.value = false;
    tagsDraft.value = normalizeTags(tagsDraft.value);
    tagsDirty.value = false;
    expanded.value = false;
  } catch (error) {
    emit('error', `任务保存失败：${String(error)}`);
  }
}

async function markStatusFromMenu(status: string) {
  try {
    await store.updateTaskStatus(props.task.record_id, status);
  } catch (error) {
    emit('error', `状态更新失败：${String(error)}`);
  }
}

async function duplicateTask() {
  try {
    await store.createTask({
      name: `${props.task.name || '未命名任务'} 副本`,
      priority: normalizePriorityDraft(props.task.priority),
      status: '待处理',
      notes: props.task.notes || '',
      sub_tasks: subTasks.value.map((item) => ({
        ...item,
        id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
        done: false
      })),
      tags: tagsDraft.value,
      due_date: props.task.due_date || '',
      recurrence_rule: props.task.recurrence_rule || null,
      reminder_before: props.task.reminder_before ?? null
    });
  } catch (error) {
    emit('error', `复制任务失败：${String(error)}`);
  }
}

function deleteFromDetailMenu() {
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

function openFromReminder() {
  expanded.value = true;
  emit('focus', props.task.record_id);
  void nextTick(() => {
    taskItemRootRef.value?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  });
}

defineExpose({
  toggleExpandFromKeyboard,
  toggleStatusFromKeyboard,
  requestDeleteFromKeyboard,
  openFromReminder
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
  height: auto;
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
  border: 1.5px solid var(--text-placeholder);
  background: var(--bg-solid);
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
  background: var(--bg-solid);
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
  border-color: var(--status-done);
  background: var(--status-done);
}

.task-content {
  flex: 1;
  min-width: 0;
  overflow: hidden;
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

.task-meta-line {
  margin-top: 2px;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  color: var(--text-tertiary);
  line-height: 1.1;
  max-width: 100%;
  overflow: hidden;
}

.badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 1px 6px;
  border-radius: 999px;
  white-space: nowrap;
  min-width: 0;
  overflow: hidden;
}

.badge span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.badge-recurring {
  color: var(--accent-purple);
  border: 1px solid color-mix(in srgb, var(--accent-purple) 24%, transparent);
  background: color-mix(in srgb, var(--accent-purple) 10%, var(--bg-solid));
}

.badge-due {
  color: var(--accent-amber);
  background: color-mix(in srgb, var(--accent-amber) 16%, var(--bg-solid));
}

.badge-overdue {
  color: var(--priority-high);
  background: color-mix(in srgb, var(--priority-high) 13%, var(--bg-solid));
}

.subtask-progress {
  color: var(--text-secondary);
  flex-shrink: 0;
}

.task-tag {
  max-width: 72px;
  padding: 1px 6px;
  border-radius: 999px;
  color: var(--accent-cyan);
  background: color-mix(in srgb, var(--accent-cyan) 10%, var(--bg-solid));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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

.inline-edit-hint {
  margin: -4px 18px 6px 48px;
  font-size: 10px;
  color: var(--text-tertiary);
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

.task-extra-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 10px;
  margin: 10px 0 8px;
}

.due-edit-field {
  display: grid;
  gap: 5px;
  color: var(--text-secondary);
  font-size: 11px;
}

.due-edit-inputs {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 96px;
  gap: 8px;
}

.due-edit-field input {
  height: 28px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-btn);
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.due-edit-field input:disabled {
  opacity: 0.45;
}

.due-edit-hint {
  color: var(--text-tertiary);
  font-size: 10px;
}

.subtasks-section {
  margin: 10px 0 8px;
  padding: 9px;
  border: 1px solid color-mix(in srgb, var(--border) 75%, transparent);
  border-radius: var(--radius-btn);
  background: color-mix(in srgb, var(--bg-secondary) 70%, transparent);
}

.subtasks-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
}

.subtasks-list {
  display: grid;
  gap: 5px;
  margin-bottom: 7px;
}

.subtask-row {
  display: grid;
  grid-template-columns: 16px minmax(0, 1fr) 20px;
  align-items: center;
  gap: 6px;
  color: var(--text-primary);
}

.subtask-row span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.subtask-row span.done {
  color: var(--text-tertiary);
  text-decoration: line-through;
}

.subtask-delete {
  width: 20px;
  height: 20px;
  border: 0;
  border-radius: 50%;
  color: var(--text-tertiary);
  background: transparent;
  cursor: pointer;
}

.subtask-delete:hover {
  color: var(--accent-red);
  background: color-mix(in srgb, var(--accent-red) 10%, transparent);
}

.subtask-add {
  display: flex;
  gap: 6px;
}

.subtask-add input {
  flex: 1;
  min-width: 0;
  height: 26px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-btn);
  background: var(--bg-solid);
  color: var(--text-primary);
  outline: none;
}

.subtask-add input:focus {
  border-color: var(--primary);
}

.subtask-add button {
  width: 26px;
  border: 1px solid var(--border);
  border-radius: var(--radius-btn);
  background: var(--bg-solid);
  color: var(--primary);
  cursor: pointer;
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
  border-color: color-mix(in srgb, var(--text-secondary) 36%, var(--border));
}

.priority-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 0.5px color-mix(in srgb, var(--text-primary) 8%, transparent);
}

.priority-btn--normal.active {
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--text-tertiary) 10%, var(--bg-solid));
  border-color: color-mix(in srgb, var(--text-tertiary) 48%, var(--border));
}

.priority-btn--important.active {
  color: var(--priority-medium);
  background: color-mix(in srgb, var(--priority-medium) 12%, var(--bg-solid));
  border-color: color-mix(in srgb, var(--priority-medium) 36%, var(--border));
}

.priority-btn--urgent.active {
  color: var(--priority-high);
  background: color-mix(in srgb, var(--priority-high) 12%, var(--bg-solid));
  border-color: color-mix(in srgb, var(--priority-high) 34%, var(--border));
}

.task-right {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 3px;
  flex-shrink: 0;
  width: 42px;
  min-width: 42px;
  align-self: stretch;
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
  color: var(--accent-red);
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
  border-radius: 999px;
  padding: 0 5px;
  height: 15px;
  line-height: 15px;
  font-size: 9px;
  color: var(--text-secondary);
  background: var(--bg-secondary);
  white-space: nowrap;
}

.sync-badge.pending {
  color: var(--status-pending);
  background: color-mix(in srgb, var(--status-pending) 18%, transparent);
}

.sync-badge.failed {
  color: var(--accent-red);
  background: color-mix(in srgb, var(--accent-red) 18%, transparent);
  cursor: pointer;
}

.task-detail-panel {
  margin: 8px 10px 10px;
  max-height: min(540px, calc(100vh - 132px));
  display: flex;
  flex-direction: column;
  font-size: 14px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 16px;
  background: var(--bg-solid);
  box-shadow: 0 18px 38px rgba(15, 23, 42, 0.12);
}

.edit-header {
  height: 44px;
  padding: 0 14px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border-light);
}

.back-btn,
.more-btn {
  border: 0;
  background: transparent;
  font-family: var(--font-family);
  cursor: pointer;
}

.back-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--accent-blue);
  font-size: 12px;
  font-weight: 650;
}

.edit-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
}

.detail-more-wrap {
  position: relative;
}

.more-btn {
  width: 30px;
  height: 30px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  color: var(--text-secondary);
}

.more-btn:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.edit-action-btn {
  border: 0;
  background: transparent;
  color: var(--accent-blue);
  font-family: var(--font-family);
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
}

.detail-menu {
  position: absolute;
  right: 0;
  top: calc(100% + 6px);
  z-index: 10;
  width: 136px;
  padding: 5px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-solid);
  box-shadow: 0 12px 28px rgba(15, 23, 42, 0.16);
}

.detail-menu button {
  width: 100%;
  height: 30px;
  padding: 0 9px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--text-primary);
  font-family: var(--font-family);
  font-size: 12px;
  text-align: left;
  cursor: pointer;
}

.detail-menu button:hover {
  background: var(--bg-secondary);
}

.detail-menu button.danger {
  color: var(--accent-red);
}

.task-status-bar {
  min-height: 34px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 7px 14px;
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-solid);
}

.detail-badge {
  display: inline-flex;
  align-items: center;
  min-height: 17px;
  padding: 2px 7px;
  border-radius: 5px;
  font-size: 9px;
  font-weight: 650;
  line-height: 1;
  white-space: nowrap;
}

.detail-badge--progress {
  color: var(--status-in-progress);
  background: color-mix(in srgb, var(--status-in-progress) 12%, var(--bg-solid));
}

.detail-badge--todo {
  color: var(--text-secondary);
  background: var(--bg-secondary);
}

.detail-badge--done {
  color: var(--status-done);
  background: color-mix(in srgb, var(--status-done) 12%, var(--bg-solid));
}

.detail-badge--overdue,
.detail-badge--urgent {
  color: var(--priority-high);
  background: color-mix(in srgb, var(--priority-high) 12%, var(--bg-solid));
}

.detail-badge--important {
  color: var(--priority-medium);
  background: color-mix(in srgb, var(--priority-medium) 12%, var(--bg-solid));
}

.detail-badge--normal {
  color: var(--text-secondary);
  background: var(--bg-secondary);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-dot.todo { background: var(--accent-blue); }
.status-dot.doing { background: var(--accent-amber); }
.status-dot.done { background: var(--accent-green); }

.status-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.status-priority {
  margin-left: auto;
  padding: 3px 8px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 650;
}

.status-priority.urgent {
  color: var(--accent-red);
  background: var(--accent-red-soft);
}

.status-priority.important {
  color: var(--accent-blue);
  background: var(--accent-blue-soft);
}

.status-priority.normal {
  color: var(--accent-slate);
  background: var(--accent-slate-soft);
}

.detail-body {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  padding: 14px 16px;
}

.task-input-area {
  margin-bottom: 12px;
}

.detail-name-input {
  width: 100%;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--text-primary);
  font-family: var(--font-family);
  font-size: 15px;
  font-weight: 500;
  line-height: 1.35;
}

.input-divider {
  height: 2px;
  margin-top: 6px;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--accent-blue), var(--accent-blue-light));
}

.detail-option-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 9px 0;
  border-bottom: 1px solid var(--border-light);
}

.detail-option-row--no-border {
  border-bottom: 0;
}

.detail-option-icon {
  width: 22px;
  height: 22px;
  margin-top: 1px;
  flex: 0 0 auto;
  display: grid;
  place-items: center;
  border-radius: 11px;
}

.detail-option-icon--blue { color: var(--accent-blue); background: var(--accent-blue-soft); }
.detail-option-icon--green { color: var(--accent-green); background: var(--accent-green-soft); }
.detail-option-icon--purple { color: var(--accent-purple); background: var(--accent-purple-soft); }
.detail-option-icon--amber { color: var(--accent-amber); background: var(--accent-amber-soft); }
.detail-option-icon--slate { color: var(--accent-slate); background: var(--accent-slate-soft); }
.detail-option-icon--cyan { color: var(--accent-cyan); background: var(--accent-cyan-soft); }

.detail-option-content {
  min-width: 0;
  flex: 1;
  display: grid;
  gap: 6px;
}

.detail-option-label {
  font-size: 10px;
  font-weight: 500;
  color: var(--text-secondary);
}

.task-detail-panel :deep(.chip-selector) {
  flex-wrap: wrap;
  gap: 5px;
}

.task-detail-panel :deep(.chip-selector__item) {
  min-height: 22px;
  padding: 4px 9px;
  white-space: nowrap;
  font-size: 11px;
}

.task-detail-panel :deep(.chip-selector--compact .chip-selector__item) {
  min-height: 22px;
  padding: 4px 9px;
  font-size: 11px;
}

.detail-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.detail-tag-chip {
  min-height: 24px;
  padding: 4px 8px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-family: var(--font-family);
  font-size: 11px;
  cursor: pointer;
}

.detail-tag-chip.selected {
  color: var(--accent-cyan);
  border-color: color-mix(in srgb, var(--accent-cyan) 45%, var(--border));
  background: color-mix(in srgb, var(--accent-cyan) 10%, var(--bg-solid));
}

.tag-empty-hint {
  margin: 0;
  color: var(--text-tertiary);
  font-size: 11px;
}

.detail-tag-input-row {
  display: flex;
  gap: 6px;
}

.detail-tag-input-row input {
  min-width: 0;
  flex: 1;
  height: 28px;
  padding: 0 9px;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-family: var(--font-family);
  font-size: 12px;
  outline: none;
}

.detail-tag-input-row input:focus {
  border-color: var(--accent-cyan);
}

.detail-tag-input-row button {
  height: 28px;
  padding: 0 10px;
  border: 1px solid color-mix(in srgb, var(--accent-cyan) 35%, var(--border));
  border-radius: 9px;
  background: var(--bg-solid);
  color: var(--accent-cyan);
  font-family: var(--font-family);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.task-detail-panel :deep(.recurrence-panel),
.task-detail-panel :deep(.reminder-section) {
  gap: 6px;
  color: var(--text-secondary);
  font-size: 11px;
}

.task-detail-panel :deep(.recurrence-header) {
  min-height: 22px;
  gap: 8px;
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 500;
}

.task-detail-panel :deep(.recurrence-summary) {
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 500;
}

.task-detail-panel :deep(.switch) {
  width: 30px;
  height: 18px;
  padding: 2px;
}

.task-detail-panel :deep(.switch i) {
  width: 14px;
  height: 14px;
}

.task-detail-panel :deep(.switch.on i) {
  transform: translateX(12px);
}

.task-detail-panel :deep(.recurrence-body) {
  gap: 7px;
  padding: 8px;
  border-radius: 10px;
}

.task-detail-panel :deep(.recurrence-section-title),
.task-detail-panel :deep(.end-block > span) {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0;
}

.task-detail-panel :deep(.weekday-row) {
  gap: 5px;
}

.task-detail-panel :deep(.weekday-row button) {
  width: 24px;
  height: 24px;
  font-size: 10px;
}

.date-time-display {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.date-badge,
.time-badge {
  padding: 3px 8px;
  border-radius: 5px;
  font-size: 11px;
  font-weight: 600;
}

.date-badge {
  color: var(--accent-green);
  border: 1px solid color-mix(in srgb, var(--accent-green) 32%, var(--border));
  background: var(--accent-green-soft);
}

.time-badge {
  color: var(--accent-blue);
  border: 1px solid color-mix(in srgb, var(--accent-blue) 26%, var(--border));
  background: var(--accent-blue-soft);
}

.date-edit-block {
  display: grid;
  gap: 8px;
}

.date-edit-inputs {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  color: var(--text-secondary);
  font-size: 10px;
}

.date-edit-inputs span,
.date-edit-inputs small {
  line-height: 1;
  white-space: nowrap;
}

.date-edit-inputs small {
  color: var(--text-tertiary);
}

.date-edit-inputs input {
  height: 24px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-family: var(--font-family);
  font-size: 11px;
  font-weight: 500;
}

.date-edit-inputs input {
  flex: 0 0 auto;
  min-width: 0;
  padding: 0 8px;
}

.date-edit-inputs input[type='date'] {
  width: 118px;
}

.date-edit-inputs input[type='time'] {
  width: 86px;
}

.detail-subtask-section {
  margin-top: 2px;
}

.detail-progress {
  margin-bottom: 7px;
}

.detail-progress-head {
  margin-bottom: 3px;
  display: flex;
  justify-content: space-between;
  color: var(--text-tertiary);
  font-size: 9px;
}

.detail-progress-bar {
  height: 3px;
  overflow: hidden;
  border-radius: 2px;
  background: var(--bg-tertiary);
}

.detail-progress-bar span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--status-done);
}

.detail-subtask-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.detail-subtask-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 0;
  border-bottom: 1px solid var(--border-light);
}

.detail-subtask-item:last-child {
  border-bottom: 0;
}

.detail-subtask-checkbox {
  width: 14px;
  height: 14px;
  position: relative;
  flex-shrink: 0;
  border-radius: 50%;
  border: 1.5px solid var(--text-placeholder);
  background: transparent;
  cursor: pointer;
}

.detail-subtask-checkbox.checked {
  border-color: var(--accent-blue);
  background: var(--accent-blue);
}

.detail-subtask-checkbox.checked::after {
  content: '';
  position: absolute;
  left: 4px;
  top: 1px;
  width: 4px;
  height: 8px;
  border: solid white;
  border-width: 0 2px 2px 0;
  transform: rotate(45deg);
}

.detail-subtask-input {
  min-width: 0;
  flex: 1;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--text-primary);
  font-family: var(--font-family);
  font-size: 11px;
}

.detail-subtask-input.done {
  color: var(--text-tertiary);
  text-decoration: line-through;
}

.detail-subtask-delete {
  width: 24px;
  height: 24px;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 14px;
  opacity: 0;
  cursor: pointer;
  transition: opacity 0.15s ease, color 0.15s ease, background 0.15s ease;
}

.detail-subtask-item:hover .detail-subtask-delete {
  opacity: 1;
}

.detail-subtask-delete:hover {
  color: var(--accent-red);
  background: var(--accent-red-soft);
}

.detail-subtask-add {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 0;
  color: var(--text-tertiary);
}

.detail-subtask-add-icon {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  border: 1.5px dashed var(--text-placeholder);
  box-sizing: border-box;
  font-size: 13px;
  line-height: 1;
}

.detail-subtask-add input {
  min-width: 0;
  flex: 1;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--text-primary);
  font-family: var(--font-family);
  font-size: 11px;
}

.detail-subtask-add:focus-within,
.detail-subtask-add:hover {
  color: var(--accent-blue);
}

.detail-subtask-add:hover .detail-subtask-add-icon,
.detail-subtask-add:focus-within .detail-subtask-add-icon {
  border-color: var(--accent-blue);
}

.detail-note-textarea {
  width: 100%;
  min-height: 50px;
  padding: 7px 9px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-family: var(--font-family);
  font-size: 11px;
  line-height: 1.5;
  resize: vertical;
  outline: none;
  transition: border-color 0.15s ease;
}

.detail-note-textarea:focus {
  border-color: var(--accent-blue);
}

.note-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  font-size: 9px;
  color: var(--text-tertiary);
}

.note-footer .saved { color: var(--accent-green); }
.note-footer .error { color: var(--accent-red); }

.detail-meta-info {
  margin-top: 10px;
  padding-top: 8px;
  display: grid;
  gap: 4px;
  border-top: 1px solid var(--border-light);
  font-size: 10px;
  color: var(--text-tertiary);
}

</style>
