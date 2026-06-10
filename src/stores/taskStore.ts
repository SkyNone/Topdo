import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { SubTask, SyncTasksResult, Task } from '../types';
import { log } from '../utils/logger';
import { generateRecurringInstances, parseRecurrenceRule } from '../utils/recurrence';

export type AppMode = 'local' | 'feishu';
export type SyncState = 'idle' | 'loading' | 'success' | 'error' | 'pending';
type ReorderPlacement = 'before' | 'after' | 'end';
export type TaskFilter = 'pending' | 'in_progress' | 'done' | 'all';
export type TaskStatusGroup = 'in_progress' | 'pending' | 'completed';

interface UpdateTaskResult {
  success: boolean;
  message: string;
}

interface CreateTaskResult {
  record_id: string;
  synced: boolean;
}

interface TaskState {
  mode: AppMode;
  modeReady: boolean;
  firstLaunch: boolean;
  filter: TaskFilter;
  searchQuery: string;
  searchOpen: boolean;
  recentTags: string[];
  tasks: Task[];
  loading: boolean;
  isSyncing: boolean;
  lastSyncTime: number | null;
  error: string | null;
  offlineMode: boolean;
  pendingCount: number;
  failedCount: number;
  lastSyncErrorSummary: string;
  statusSyncState: Record<string, SyncState>;
  notesSyncState: Record<string, SyncState>;
}

interface CreateTaskInput {
  name: string;
  priority?: string;
  task_type?: string;
  status?: string;
  notes?: string;
  sub_tasks?: SubTask[];
  tags?: string[];
  due_date?: string;
  recurrence_rule?: Task['recurrence_rule'];
  recurrence_parent_id?: string;
  recurrence_index?: number;
  reminder_before?: number | null;
}

const FEISHU_RECORDS_URL = '/open-apis/bitable/v1/apps/{app_token}/tables/{table_id}/records';
const FEISHU_UPDATE_URL = '/open-apis/bitable/v1/apps/{app_token}/tables/{table_id}/records/{record_id}';
const RECENT_TAGS_STORAGE_KEY = 'topdo_recent_task_tags_v1';
const MAX_TASK_TAGS = 5;
const MAX_RECENT_TAGS = 5;
const COMPLETED_HISTORY_WINDOW_DAYS = 30;
const COMPLETED_HISTORY_WINDOW_MS = COMPLETED_HISTORY_WINDOW_DAYS * 24 * 60 * 60 * 1000;
let syncTimer: ReturnType<typeof setTimeout> | null = null;
const localStatusSaveQueue = new Map<string, Promise<void>>();

function nowUnixSecondsString(): string {
  return Math.floor(Date.now() / 1000).toString();
}

function normalizeStatus(status: string): 'todo' | 'in_progress' | 'completed' | 'unknown' {
  const value = status.trim();
  if (value.includes('进行中')) return 'in_progress';
  if (value.includes('已完成')) return 'completed';
  if (value.includes('待处理') || value.includes('待办') || value.includes('待启动')) return 'todo';
  return 'unknown';
}

function normalizeStatusLabel(status: string): string {
  const key = normalizeStatus(status);
  if (key === 'in_progress') return '进行中';
  if (key === 'completed') return '已完成';
  return '待处理';
}

function completedAtForStatus(status: string, previousCompletedAt = ''): string {
  const key = normalizeStatus(status);
  if (key === 'completed') {
    return previousCompletedAt || nowUnixSecondsString();
  }
  return '';
}

function mergeTask(base: Task, patch: Partial<Task>): Task {
  return {
    ...base,
    ...patch,
    updated_at: patch.updated_at ?? nowUnixSecondsString()
  };
}

function fromFeishuPriority(priority: string): string {
  const map: Record<string, string> = {
    '🔴今日必做': '紧急',
    '🔴 今日必做': '紧急',
    '今日必做': '紧急',
    '🟡尽快完成': '重要',
    '🟡重要不紧急': '重要',
    '本周完成': '重要',
    '🟡本周完成': '重要',
    '🟠本周完成': '重要',
    '🔵本周完成': '重要',
    '🔵有空再说': '普通',
    '🔵常规任务': '普通',
    '自由安排': '普通',
    '⚪️自由安排': '普通',
    '⚪自由安排': '普通'
  };
  return map[priority] || priority || '普通';
}

function toFeishuPriority(priority: string): string {
  const map: Record<string, string> = {
    '紧急': '今日必做',
    '重要': '本周完成',
    '普通': '自由安排'
  };
  return map[priority] || priority;
}

function normalizePriority(priority: string): string {
  const value = (priority || '').trim();
  return fromFeishuPriority(value);
}

function failedSyncPatch(message: string): Partial<Task> {
  const retryable = message.includes('待重试') || message.includes('离线缓存');
  return {
    sync_status: retryable ? 'pending' : 'failed',
    last_error: message
  };
}

function findTaskByRecordId(tasks: Task[], recordId: string): Task | undefined {
  return tasks.find((task) => task.record_id === recordId || task.id === recordId);
}

function findRefreshedTaskBySnapshot(tasks: Task[], snapshot: Task): Task | undefined {
  const byDirectId = tasks.find(
    (task) =>
      task.record_id === snapshot.record_id ||
      task.id === snapshot.id ||
      (!!snapshot.feishu_record_id && task.record_id === snapshot.feishu_record_id)
  );
  if (byDirectId) return byDirectId;

  const sourceCreated = timestampMs(snapshot.created_at);
  return tasks.find((task) => {
    if (task.name !== snapshot.name) return false;
    const delta = Math.abs(timestampMs(task.created_at) - sourceCreated);
    return delta <= 60_000;
  });
}

function normalizeSubTasks(value: unknown): SubTask[] {
  const parsed = typeof value === 'string'
    ? (() => {
        try {
          return JSON.parse(value);
        } catch {
          return [];
        }
      })()
    : value;

  if (!Array.isArray(parsed)) return [];
  return parsed
    .filter((item) => item && typeof item === 'object')
    .map((item: any) => ({
      id: String(item.id || ''),
      text: String(item.text || ''),
      done: Boolean(item.done),
      created_at: String(item.created_at || nowUnixSecondsString())
    }))
    .filter((item) => item.id && item.text.trim());
}

function normalizeTagList(value: unknown, limit: number): string[] {
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
    .map((item) => String(item || '').trim())
    .filter(Boolean)
    .filter((item) => {
      const key = item.toLowerCase();
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .slice(0, limit);
}

function normalizeTags(value: unknown): string[] {
  return normalizeTagList(value, MAX_TASK_TAGS);
}

function normalizeRecentTags(value: unknown): string[] {
  return normalizeTagList(value, MAX_RECENT_TAGS);
}

function loadRecentTags(): string[] {
  if (typeof window === 'undefined') return [];
  try {
    return normalizeRecentTags(window.localStorage.getItem(RECENT_TAGS_STORAGE_KEY) || '[]');
  } catch {
    return [];
  }
}

function persistRecentTags(tags: string[]) {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(RECENT_TAGS_STORAGE_KEY, JSON.stringify(normalizeRecentTags(tags)));
  } catch {
    // Ignore storage failures; tag editing should not be blocked by localStorage.
  }
}

function normalizeTask(task: Task): Task {
  return {
    ...task,
    priority: normalizePriority(task.priority || '普通'),
    sort_order: Number(task.sort_order || 0),
    sub_tasks: normalizeSubTasks((task as any).sub_tasks),
    tags: normalizeTags((task as any).tags),
    due_date: String((task as any).due_date || ''),
    recurrence_rule: parseRecurrenceRule((task as any).recurrence_rule),
    recurrence_parent_id: String((task as any).recurrence_parent_id || ''),
    recurrence_index: (task as any).recurrence_index === null || (task as any).recurrence_index === undefined ? null : Number((task as any).recurrence_index),
    reminder_before: (task as any).reminder_before === null || (task as any).reminder_before === undefined ? null : Number((task as any).reminder_before),
    reminder_notified: Boolean((task as any).reminder_notified)
  };
}

function hydrateRecurringInstanceRules(tasks: Task[]): Task[] {
  const byId = new Map<string, Task>();
  for (const task of tasks) {
    const keys = [task.id, task.record_id, task.feishu_record_id].filter(Boolean) as string[];
    for (const key of keys) byId.set(key, task);
  }

  return tasks.map((task) => {
    if (task.recurrence_rule || !task.recurrence_parent_id) return task;
    const parent = byId.get(task.recurrence_parent_id);
    return parent?.recurrence_rule ? { ...task, recurrence_rule: parent.recurrence_rule } : task;
  });
}

function sameRecurrenceRule(a: Task['recurrence_rule'] | null | undefined, b: Task['recurrence_rule'] | null | undefined): boolean {
  return JSON.stringify(a || null) === JSON.stringify(b || null);
}

function isRecordIdNotFound(message: string): boolean {
  return message.includes('RecordIdNotFound') || message.includes('code=1254043');
}

function timestampMs(raw: string): number {
  const text = (raw || '').trim();
  if (!text) return 0;
  const num = Number(text);
  if (Number.isFinite(num)) return num > 1e12 ? num : num * 1000;
  const parsed = new Date(text).getTime();
  return Number.isFinite(parsed) ? parsed : 0;
}

function dayKey(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

function dateKeyFromTimestamp(raw: string): string {
  const ms = timestampMs(raw);
  if (!ms) return '';
  return dayKey(new Date(ms));
}

function completedReferenceMs(task: Task): number {
  return timestampMs(task.completed_at || '') || timestampMs(task.updated_at || '') || timestampMs(task.created_at || '');
}

function isRecentCompletedTask(task: Task): boolean {
  if (normalizeStatus(task.status) !== 'completed') return true;
  const completed = completedReferenceMs(task);
  if (!completed) return true;
  return completed >= Date.now() - COMPLETED_HISTORY_WINDOW_MS;
}

function startOfWeek(date: Date): Date {
  const day = date.getDay() || 7;
  const start = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  start.setDate(start.getDate() - day + 1);
  return start;
}

function statusRank(status: string): number {
  const key = normalizeStatus(status);
  if (key === 'in_progress') return 0;
  if (key === 'todo') return 1;
  if (key === 'completed') return 2;
  return 3;
}

function priorityRank(priority: string): number {
  const normalized = normalizePriority(priority || '普通').trim();
  if (normalized === '紧急') return 0;
  if (normalized === '重要') return 1;
  return 2;
}

function sortTasksByPolicy(tasks: Task[]): Task[] {
  return [...tasks].sort((a, b) => {
    const statusDiff = statusRank(a.status) - statusRank(b.status);
    if (statusDiff !== 0) return statusDiff;

    const priorityDiff = priorityRank(a.priority) - priorityRank(b.priority);
    if (priorityDiff !== 0) return priorityDiff;

    const orderDiff = Number(b.sort_order || 0) - Number(a.sort_order || 0);
    if (orderDiff !== 0) return orderDiff;

    return timestampMs(b.created_at) - timestampMs(a.created_at);
  });
}

function applySearchFilter(tasks: Task[], searchQuery: string): Task[] {
  const query = searchQuery.trim().toLowerCase();
  if (!query) return tasks;

  return tasks.filter((task) => {
    const name = (task.name || '').toLowerCase();
    const notes = (task.notes || '').toLowerCase();
    const tags = normalizeTags((task as any).tags).join(' ').toLowerCase();
    return name.includes(query) || notes.includes(query) || tags.includes(query);
  });
}

export const useTaskStore = defineStore('task', {
  state: (): TaskState => ({
    mode: 'local',
    modeReady: false,
    firstLaunch: false,
    filter: 'pending',
    searchQuery: '',
    searchOpen: false,
    recentTags: loadRecentTags(),
    tasks: [],
    loading: false,
    isSyncing: false,
    lastSyncTime: null,
    error: null,
    offlineMode: false,
    pendingCount: 0,
    failedCount: 0,
    lastSyncErrorSummary: '',
    statusSyncState: {},
    notesSyncState: {}
  }),

  getters: {
    todoCount: (state) => state.tasks.filter((task) => normalizeStatus(task.status) !== 'completed').length,
    pendingTaskCount: (state) => state.tasks.filter((task) => normalizeStatus(task.status) === 'todo').length,
    inProgressTaskCount: (state) => state.tasks.filter((task) => normalizeStatus(task.status) === 'in_progress').length,
    completedTaskCount: (state) => state.tasks.filter((task) => normalizeStatus(task.status) === 'completed').length,
    totalTaskCount: (state) => state.tasks.length,
    inProgressCount: (state) => state.tasks.filter((task) => normalizeStatus(task.status) === 'in_progress').length,
    completedCount: (state) => state.tasks.filter((task) => normalizeStatus(task.status) === 'completed').length,
    inProgressTasks: (state) => state.tasks.filter((task) => normalizeStatus(task.status) === 'in_progress'),
    hasActiveSearch: (state) => state.searchQuery.trim().length > 0,
    hiddenOldCompletedCount: (state) => {
      if (state.searchQuery.trim().length > 0) return 0;
      return state.tasks.filter((task) => normalizeStatus(task.status) === 'completed' && !isRecentCompletedTask(task)).length;
    },
    todayCompletedCount: (state) => {
      const today = dayKey(new Date());
      return state.tasks.filter((task) => normalizeStatus(task.status) === 'completed' && dateKeyFromTimestamp(task.completed_at || '') === today).length;
    },
    weekCompletedCount: (state) => {
      const start = startOfWeek(new Date()).getTime();
      return state.tasks.filter((task) => {
        if (normalizeStatus(task.status) !== 'completed') return false;
        const completed = timestampMs(task.completed_at || '');
        return completed >= start;
      }).length;
    },
    weekCreatedTaskCount: (state) => {
      const start = startOfWeek(new Date()).getTime();
      return state.tasks.filter((task) => timestampMs(task.created_at || '') >= start).length;
    },
    weekCreatedCompletedCount: (state) => {
      const start = startOfWeek(new Date()).getTime();
      return state.tasks.filter((task) => {
        const created = timestampMs(task.created_at || '');
        return created >= start && normalizeStatus(task.status) === 'completed';
      }).length;
    },
    completionStreak: (state) => {
      const completedDays = new Set(
        state.tasks
          .filter((task) => normalizeStatus(task.status) === 'completed')
          .map((task) => dateKeyFromTimestamp(task.completed_at || ''))
          .filter(Boolean)
      );
      let streak = 0;
      const cursor = new Date();
      while (completedDays.has(dayKey(cursor))) {
        streak += 1;
        cursor.setDate(cursor.getDate() - 1);
      }
      return streak;
    },
    recentCompletionStats: (state) => {
      const counts = new Map<string, number>();
      for (const task of state.tasks) {
        if (normalizeStatus(task.status) !== 'completed') continue;
        const key = dateKeyFromTimestamp(task.completed_at || '');
        if (!key) continue;
        counts.set(key, (counts.get(key) || 0) + 1);
      }
      return Array.from({ length: 7 }, (_, index) => {
        const date = new Date();
        date.setDate(date.getDate() - (6 - index));
        const key = dayKey(date);
        return {
          date: key,
          label: index === 6 ? '今天' : `${date.getMonth() + 1}/${date.getDate()}`,
          count: counts.get(key) || 0
        };
      });
    },
    filteredTasks: (state) => {
      const sorted = sortTasksByPolicy(state.tasks);
      const searched = applySearchFilter(sorted, state.searchQuery);
      const hasSearch = state.searchQuery.trim().length > 0;
      const visibleTasks = hasSearch
        ? searched
        : searched.filter((task) => normalizeStatus(task.status) !== 'completed' || isRecentCompletedTask(task));
      switch (state.filter) {
        case 'pending':
          return visibleTasks.filter((task) => normalizeStatus(task.status) === 'todo');
        case 'in_progress':
          return visibleTasks.filter((task) => normalizeStatus(task.status) === 'in_progress');
        case 'done':
          return visibleTasks.filter((task) => normalizeStatus(task.status) === 'completed');
        case 'all':
        default:
          return visibleTasks;
      }
    },
    groupedFilteredTasks: (state): Array<{ status: TaskStatusGroup; tasks: Task[] }> => {
      const inProgress: Task[] = [];
      const pending: Task[] = [];
      const completed: Task[] = [];

      const includePending = state.filter === 'all' || state.filter === 'pending';
      const includeInProgress = state.filter === 'all' || state.filter === 'in_progress';
      const includeDone = state.filter === 'all' || state.filter === 'done';

      const hasSearch = state.searchQuery.trim().length > 0;
      const sorted = applySearchFilter(sortTasksByPolicy(state.tasks), state.searchQuery)
        .filter((task) => hasSearch || normalizeStatus(task.status) !== 'completed' || isRecentCompletedTask(task));
      for (const task of sorted) {
        const key = normalizeStatus(task.status);
        if (key === 'in_progress') {
          if (includeInProgress) inProgress.push(task);
          continue;
        }
        if (key === 'completed') {
          if (includeDone) completed.push(task);
          continue;
        }
        if (includePending) pending.push(task);
      }

      const groups: Array<{ status: TaskStatusGroup; tasks: Task[] }> = [];
      if (includeInProgress) {
        groups.push({ status: 'in_progress', tasks: inProgress });
      }
      if (includePending) {
        groups.push({ status: 'pending', tasks: pending });
      }
      if (includeDone) {
        groups.push({ status: 'completed', tasks: completed });
      }
      return groups;
    }
  },

  actions: {
    setFilter(filter: TaskFilter) {
      this.filter = filter;
    },

    setSearchQuery(query: string) {
      this.searchQuery = query;
    },

    openSearch() {
      this.searchOpen = true;
    },

    closeSearch() {
      this.searchOpen = false;
    },

    clearSearch() {
      this.searchQuery = '';
      this.closeSearch();
    },

    rememberTags(tags: unknown) {
      const normalized = normalizeTags(tags);
      if (!normalized.length) return;
      this.recentTags = normalizeRecentTags([...normalized, ...this.recentTags]);
      persistRecentTags(this.recentTags);
    },

    setModeState(mode: AppMode) {
      this.mode = mode;
      console.log('[Store] mode 变更为:', mode);
    },

    setTasks(tasks: Task[]) {
      this.tasks = hydrateRecurringInstanceRules(tasks.map(normalizeTask));
    },

    setSyncMeta(payload?: SyncTasksResult['sync_meta']) {
      this.pendingCount = payload?.pending_count || 0;
      this.failedCount = payload?.failed_count || 0;
      this.lastSyncErrorSummary = payload?.last_error_summary || '';
    },

    setTaskPatch(recordId: string, patch: Partial<Task>) {
      const index = this.tasks.findIndex((task) => task.record_id === recordId || task.id === recordId);
      if (index < 0) return;
      const merged = mergeTask(this.tasks[index], patch);
      if (patch.priority !== undefined && this.mode !== 'feishu') {
        merged.priority = normalizePriority(patch.priority);
      }
      this.tasks[index] = merged;
    },

    async updateTaskLocalFields(recordId: string, patch: Partial<Task>, fields: Record<string, string>) {
      const target = findTaskByRecordId(this.tasks, recordId);
      if (!target) throw new Error('任务不存在');

      const previous = { ...target };
      this.setTaskPatch(recordId, patch);

      try {
        const updated = await invoke<Task>('update_local_task', {
          id: target.id || target.record_id,
          fields
        });
        const normalized = normalizeTask(updated);
        this.setTaskPatch(recordId, normalized);
        return normalized;
      } catch (error) {
        this.setTaskPatch(recordId, previous);
        throw error;
      }
    },

    async updateTaskSubTasks(recordId: string, subTasks: SubTask[]) {
      const normalized = normalizeSubTasks(subTasks);
      await this.updateTaskLocalFields(
        recordId,
        { sub_tasks: normalized },
        { sub_tasks: JSON.stringify(normalized) }
      );
    },

    async updateTaskTags(recordId: string, tags: string[]) {
      const normalized = normalizeTags(tags);
      const updated = await this.updateTaskLocalFields(
        recordId,
        { tags: normalized },
        { tags: JSON.stringify(normalized) }
      );
      this.rememberTags(normalized);
      if (updated.sync_status === 'pending') {
        this.scheduleSyncAfterWrite();
      }
    },

    async updateTaskDueDate(recordId: string, dueDate: string) {
      const next = dueDate.trim();
      await this.updateTaskLocalFields(recordId, { due_date: next }, { due_date: next });
    },

    async updateTaskRecurrence(recordId: string, rule: Task['recurrence_rule']) {
      const serialized = rule ? JSON.stringify(rule) : '';
      await this.updateTaskLocalFields(recordId, { recurrence_rule: rule || null }, { recurrence_rule: serialized });
    },

    async updateTaskReminder(recordId: string, reminderBefore: number | null) {
      await this.updateTaskLocalFields(
        recordId,
        { reminder_before: reminderBefore, reminder_notified: false },
        {
          reminder_before: reminderBefore === null ? '' : String(reminderBefore),
          reminder_notified: '0'
        }
      );
    },

    async markTaskReminderNotified(recordId: string) {
      await this.updateTaskLocalFields(recordId, { reminder_notified: true }, { reminder_notified: '1' });
    },

    async updateTaskDetails(recordId: string, input: Partial<CreateTaskInput>) {
      const target = findTaskByRecordId(this.tasks, recordId);
      if (!target) throw new Error('任务不存在');

      const fields: Record<string, string> = {};
      const patch: Partial<Task> = {};
      let parentRecurrenceUpdate: {
        recordId: string;
        fields: Record<string, string>;
        patch: Partial<Task>;
        previous: Task;
      } | null = null;
      let remoteAffectingChanged = false;
      let tagsForRecent: string[] | null = null;

      if (input.name !== undefined) {
        const name = input.name.trim();
        if (!name) throw new Error('任务名称不能为空');
        if (name !== (target.name || '').trim()) {
          fields.name = name;
          patch.name = name;
          remoteAffectingChanged = true;
        }
      }
      if (input.priority !== undefined) {
        const priority = normalizePriority(input.priority || '普通');
        if (priority !== normalizePriority(target.priority || '普通')) {
          fields.priority = priority;
          patch.priority = priority;
          remoteAffectingChanged = true;
        }
      }
      if (input.due_date !== undefined) {
        const dueDate = input.due_date.trim();
        if (dueDate !== (target.due_date || '').trim()) {
          fields.due_date = dueDate;
          patch.due_date = dueDate;
        }
      }
      if (input.notes !== undefined) {
        if (input.notes !== (target.notes || '')) {
          fields.notes = input.notes;
          patch.notes = input.notes;
          remoteAffectingChanged = true;
        }
      }
      if (input.sub_tasks !== undefined) {
        const subTasks = normalizeSubTasks(input.sub_tasks);
        if (JSON.stringify(subTasks) !== JSON.stringify(normalizeSubTasks(target.sub_tasks || []))) {
          fields.sub_tasks = JSON.stringify(subTasks);
          patch.sub_tasks = subTasks;
        }
      }
      if (input.tags !== undefined) {
        const tags = normalizeTags(input.tags);
        if (JSON.stringify(tags) !== JSON.stringify(normalizeTags(target.tags || []))) {
          fields.tags = JSON.stringify(tags);
          patch.tags = tags;
          tagsForRecent = tags;
          remoteAffectingChanged = true;
        }
      }
      if (input.recurrence_rule !== undefined) {
        const recurrence = input.recurrence_rule || null;
        const serialized = recurrence ? JSON.stringify(recurrence) : '';
        if (!sameRecurrenceRule(recurrence, target.recurrence_rule)) {
          fields.recurrence_rule = serialized;
          patch.recurrence_rule = recurrence;
        }

        const parentId = (target.recurrence_parent_id || '').trim();
        const parent = parentId ? findTaskByRecordId(this.tasks, parentId) : undefined;
        if (parent && !sameRecurrenceRule(recurrence, parent.recurrence_rule)) {
          parentRecurrenceUpdate = {
            recordId: parent.record_id || parent.id || parentId,
            fields: { recurrence_rule: serialized },
            patch: { recurrence_rule: recurrence },
            previous: { ...parent }
          };
        }
      }
      if (input.reminder_before !== undefined) {
        const reminderBefore = input.reminder_before ?? null;
        if (reminderBefore !== (target.reminder_before ?? null)) {
          fields.reminder_before = reminderBefore === null ? '' : String(reminderBefore);
          fields.reminder_notified = '0';
          patch.reminder_before = reminderBefore;
          patch.reminder_notified = false;
        }
      }

      const hasCurrentChanges = Object.keys(fields).length > 0;
      if (!hasCurrentChanges && !parentRecurrenceUpdate) return;

      const previous = { ...target };
      if (hasCurrentChanges) {
        this.setTaskPatch(recordId, {
          ...patch,
          sync_status: this.mode === 'feishu' && remoteAffectingChanged ? 'pending' : target.sync_status
        });
      }
      if (parentRecurrenceUpdate) {
        this.setTaskPatch(parentRecurrenceUpdate.recordId, parentRecurrenceUpdate.patch);
      }

      try {
        if (hasCurrentChanges) {
          const updated = await invoke<Task>('update_local_task', {
            id: target.id || target.record_id,
            fields
          });
          this.setTaskPatch(recordId, {
            ...normalizeTask(updated),
            record_id: updated.record_id || recordId,
            id: updated.id || updated.record_id || recordId,
            sync_status: this.mode === 'feishu' && remoteAffectingChanged ? 'pending' : updated.sync_status
          });
        }
        if (parentRecurrenceUpdate) {
          const updatedParent = await invoke<Task>('update_local_task', {
            id: parentRecurrenceUpdate.previous.id || parentRecurrenceUpdate.previous.record_id,
            fields: parentRecurrenceUpdate.fields
          });
          this.setTaskPatch(parentRecurrenceUpdate.recordId, normalizeTask(updatedParent));
        }
        this.tasks = sortTasksByPolicy(this.tasks);
        if (remoteAffectingChanged) {
          this.scheduleSyncAfterWrite();
        }
        if (tagsForRecent) {
          this.rememberTags(tagsForRecent);
        }
      } catch (error) {
        if (hasCurrentChanges) this.setTaskPatch(recordId, previous);
        if (parentRecurrenceUpdate) {
          this.setTaskPatch(parentRecurrenceUpdate.recordId, parentRecurrenceUpdate.previous);
        }
        throw error;
      }
    },

    async initRecurringTasks() {
      const instances = generateRecurringInstances(this.tasks);
      for (const instance of instances) {
        await this.createTask({
          name: instance.name,
          priority: instance.priority,
          status: instance.status,
          due_date: instance.due_date,
          recurrence_parent_id: instance.recurrence_parent_id,
          recurrence_index: instance.recurrence_index,
          recurrence_rule: instance.recurrence_rule,
          reminder_before: instance.reminder_before
        });
      }
    },

    startDailyRecurrenceCheck() {
      const now = new Date();
      const nextMidnight = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime();
      window.setTimeout(() => {
        void this.initRecurringTasks();
        window.setInterval(() => void this.initRecurringTasks(), 24 * 60 * 60 * 1000);
      }, nextMidnight - now.getTime());
    },

    async reorderTask(recordId: string, targetPriority: string, targetRecordId = '', placement: ReorderPlacement = 'before') {
      const target = findTaskByRecordId(this.tasks, recordId);
      if (!target || normalizeStatus(target.status) === 'completed') return;

      const priority = normalizePriority(targetPriority || target.priority || '普通');
      const status = normalizeStatus(target.status);
      const previousTasks = [...this.tasks];
      const baseOrder = Date.now();
      const group = sortTasksByPolicy(this.tasks)
        .filter((task) => normalizeStatus(task.status) === status)
        .filter((task) => normalizePriority(task.priority || '普通') === priority)
        .filter((task) => task.record_id !== recordId && task.id !== recordId);

      const targetIndex = targetRecordId
        ? group.findIndex((task) => task.record_id === targetRecordId || task.id === targetRecordId)
        : -1;
      const insertIndex = targetRecordId && targetIndex >= 0
        ? targetIndex + (placement === 'after' ? 1 : 0)
        : group.length;
      group.splice(insertIndex, 0, { ...target, priority });

      const updates = group.map((task, index) => ({
        recordId: task.record_id,
        id: task.id || task.record_id,
        sort_order: baseOrder - index,
        priority: task.record_id === target.record_id || task.id === target.id ? priority : normalizePriority(task.priority || '普通')
      }));

      for (const update of updates) {
        const patch: Partial<Task> = {
          priority: update.priority,
          sort_order: update.sort_order
        };
        if (update.recordId === recordId && this.mode === 'feishu' && update.priority !== normalizePriority(target.priority || '普通')) {
          patch.sync_status = 'pending';
        }
        this.setTaskPatch(update.recordId, patch);
      }
      this.tasks = sortTasksByPolicy(this.tasks);

      try {
        if (priority !== normalizePriority(target.priority || '普通')) {
          await this.updateTaskPriority(recordId, priority);
        }
        if (this.mode === 'local') {
          await invoke('reorder_local_tasks', {
            orderedIds: updates.map((update) => update.id)
          });
        } else {
          await Promise.all(
            updates.map((update) =>
              invoke<Task>('update_local_task', {
                id: update.id,
                fields: { sort_order: String(update.sort_order) }
              })
            )
          );
        }
        for (const update of updates) {
          this.setTaskPatch(update.recordId, { sort_order: update.sort_order });
        }
        this.tasks = sortTasksByPolicy(this.tasks);
      } catch (error) {
        this.tasks = previousTasks;
        throw error;
      }
    },

    moveTaskToStatusTail(recordId: string, nextStatus: string) {
      const idx = this.tasks.findIndex((task) => task.record_id === recordId || task.id === recordId);
      if (idx < 0) return;

      const statusLabel = normalizeStatusLabel(nextStatus);
      const task = this.tasks[idx];
      this.tasks.splice(idx, 1);
      task.status = statusLabel;

      const targetGroup = normalizeStatus(statusLabel);
      let insertAt = this.tasks.length;
      for (let i = this.tasks.length - 1; i >= 0; i -= 1) {
        if (normalizeStatus(this.tasks[i].status) === targetGroup) {
          insertAt = i + 1;
          break;
        }
      }
      this.tasks.splice(insertAt, 0, task);
    },

    async initMode() {
      try {
        const mode = await invoke<string>('get_app_mode');
        console.log('[Init] 读取到的模式:', mode);
        if (mode === 'feishu') {
          this.setModeState('feishu');
          this.firstLaunch = false;
        } else if (mode === 'local') {
          this.setModeState('local');
          this.firstLaunch = false;
        } else {
          this.setModeState('local');
          this.firstLaunch = true;
        }
      } catch {
        console.log('[Init] 读取到的模式:', 'local (fallback)');
        this.setModeState('local');
        this.firstLaunch = true;
      } finally {
        this.modeReady = true;
      }
    },

    async setMode(mode: AppMode) {
      await invoke('set_app_mode', { mode });
      this.setModeState(mode);
      this.firstLaunch = false;
      this.error = null;
      this.offlineMode = false;
      await this.fetchTasks();
    },

    async fetchTasks() {
      this.loading = true;
      this.error = null;
      try {
        if (this.mode === 'local') {
          const tasks = await invoke<Task[]>('get_local_tasks');
          this.setTasks(tasks);
          this.setSyncMeta();
          this.offlineMode = false;
          return;
        }
        await this.triggerSync();
      } catch (error) {
        this.error = String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },

    async triggerSync() {
      if (this.mode !== 'feishu') return;
      if (this.isSyncing) return;
      this.isSyncing = true;
      log('SYNC', '开始同步');
      try {
        log('API', '调用飞书', { url: FEISHU_RECORDS_URL, method: 'GET' });
        const syncResult = await invoke<SyncTasksResult | Task[]>('sync_tasks');
        const normalized = Array.isArray(syncResult)
          ? { tasks: syncResult, sync_meta: { pending_count: 0, failed_count: 0, last_sync_at: '', last_error_summary: '' } }
          : syncResult;
        this.setTasks(normalized.tasks);
        this.setSyncMeta(normalized.sync_meta);
        this.lastSyncTime = Date.now();
        this.offlineMode = false;
        this.error = null;
        log('API', '飞书返回', {
          status: 200,
          data: {
            count: normalized.tasks.length,
            pending_count: normalized.sync_meta.pending_count,
            failed_count: normalized.sync_meta.failed_count
          }
        });
        log('SYNC', '同步完成', { count: normalized.tasks.length, pending: normalized.sync_meta.pending_count, failed: normalized.sync_meta.failed_count });
      } catch (error) {
        this.offlineMode = true;
        this.error = String(error);
        log('API', '飞书报错', { error: String(error) });
        throw error;
      } finally {
        this.isSyncing = false;
      }
    },

    scheduleSyncAfterWrite() {
      if (this.mode !== 'feishu') return;
      if (syncTimer) clearTimeout(syncTimer);
      syncTimer = setTimeout(() => {
        void this.triggerSync();
      }, 1000);
    },

    async updateTaskStatus(recordId: string, toStatus: string) {
      const target = this.tasks.find((task) => task.record_id === recordId || task.id === recordId);
      if (!target) throw new Error('任务不存在');

      const fromStatus = target.status;
      const previous = { ...target };
      const previousTasks = this.tasks.map((task) => ({ ...task }));
      log('STATUS', '切换状态', { taskId: recordId, from: fromStatus, to: toStatus });

      this.moveTaskToStatusTail(recordId, toStatus);
      const normalizedStatus = normalizeStatusLabel(toStatus);
      this.setTaskPatch(recordId, {
        status: normalizedStatus,
        completed_at: completedAtForStatus(normalizedStatus, previous.completed_at),
        sync_status: this.mode === 'feishu' ? 'pending' : 'synced'
      });
      const current = this.tasks.find((task) => task.record_id === recordId || task.id === recordId);
      const key = normalizeStatus(normalizedStatus);
      if (current && key === 'in_progress') {
        const currentIndex = this.tasks.findIndex(
          (task) => (task.record_id === recordId || task.id === recordId) && normalizeStatus(task.status) === 'in_progress'
        );
        if (currentIndex > 0) {
          this.tasks.splice(currentIndex, 1);
          this.tasks.unshift(current);
        }
      }
      if (this.mode === 'local') {
        this.statusSyncState[recordId] = 'success';

        const id = target.id || target.record_id;
        const previousQueue = localStatusSaveQueue.get(recordId) || Promise.resolve();
        const saveTask = previousQueue
          .catch(() => undefined)
          .then(async () => {
            await invoke<Task>('update_local_task', {
              id,
              fields: { status: normalizedStatus }
            });
          })
          .catch((error) => {
            const latest = this.tasks.find((task) => task.record_id === recordId || task.id === recordId);
            if (latest && normalizeStatus(latest.status) === normalizeStatus(normalizedStatus)) {
              this.tasks = previousTasks;
              this.statusSyncState[recordId] = 'error';
              this.error = `状态保存失败：${String(error)}`;
            }
            log('API', '本地状态保存失败', { error: String(error) });
          })
          .finally(() => {
            if (localStatusSaveQueue.get(recordId) === saveTask) {
              localStatusSaveQueue.delete(recordId);
            }
          });

        localStatusSaveQueue.set(recordId, saveTask);
        return;
      }

      this.statusSyncState[recordId] = 'loading';

      const tryUpdateRemoteStatus = async (remoteRecordId: string) => {
        log('API', '调用飞书', {
          url: FEISHU_UPDATE_URL,
          method: 'PUT',
          record_id: remoteRecordId,
          field: '状态'
        });

        const result = await invoke<UpdateTaskResult>('update_task', {
          recordId: remoteRecordId,
          record_id: remoteRecordId,
          fieldName: '状态',
          field_name: '状态',
          value: toStatus
        });

        log('API', '飞书返回', { status: result.success ? 200 : 500, data: result });
        return result;
      };

      const findRefreshedTask = (snapshot: Task): Task | undefined => {
        const byDirectId = this.tasks.find(
          (task) =>
            task.record_id === snapshot.record_id ||
            task.id === snapshot.id ||
            (!!snapshot.feishu_record_id && task.record_id === snapshot.feishu_record_id)
        );
        if (byDirectId) return byDirectId;

        // 对离线新建后换 record_id 的场景做兜底：用名称 + 创建时间近似匹配
        const sourceCreated = timestampMs(snapshot.created_at);
        return this.tasks.find((task) => {
          if (task.name !== snapshot.name) return false;
          const delta = Math.abs(timestampMs(task.created_at) - sourceCreated);
          return delta <= 60_000;
        });
      };

      try {
        let result = await tryUpdateRemoteStatus(target.record_id);

        if (!result.success && isRecordIdNotFound(result.message || '')) {
          await this.triggerSync();
          const refreshed = findRefreshedTask(target);
          if (refreshed) {
            result = await tryUpdateRemoteStatus(refreshed.record_id);
            if (result.success && refreshed.record_id !== recordId) {
              this.statusSyncState[refreshed.record_id] = 'success';
            }
          } else {
            // 已完成同步但旧记录已不存在，避免把用户卡在错误提示里
            this.statusSyncState[recordId] = 'pending';
            return;
          }
        }

        if (!result.success) {
          const message = result.message || '飞书更新失败';
          const patch = failedSyncPatch(message);
          this.setTaskPatch(recordId, patch);
          this.statusSyncState[recordId] = patch.sync_status === 'failed' ? 'error' : 'pending';
          this.offlineMode = true;
          this.error = message;
          if (patch.sync_status === 'pending') this.scheduleSyncAfterWrite();
          return;
        }

        this.setTaskPatch(recordId, { sync_status: 'synced' });
        this.statusSyncState[recordId] = 'success';
        this.offlineMode = false;
        this.lastSyncTime = Date.now();
        this.scheduleSyncAfterWrite();
      } catch (error) {
        this.setTaskPatch(recordId, previous);
        this.tasks = previousTasks;
        this.statusSyncState[recordId] = 'error';
        log('API', '飞书报错', { error: String(error) });
        throw error;
      }
    },

    async updateTaskNotes(recordId: string, notes: string) {
      const target = this.tasks.find((task) => task.record_id === recordId || task.id === recordId);
      if (!target) throw new Error('任务不存在');

      const previous = { ...target };
      this.setTaskPatch(recordId, {
        notes,
        sync_status: this.mode === 'feishu' ? 'pending' : 'synced'
      });
      this.notesSyncState[recordId] = 'loading';

      const tryUpdateRemoteNotes = async (remoteRecordId: string) => {
        log('API', '调用飞书', {
          url: FEISHU_UPDATE_URL,
          method: 'PUT',
          record_id: remoteRecordId,
          field: '备注/收获'
        });

        const result = await invoke<UpdateTaskResult>('update_task', {
          recordId: remoteRecordId,
          record_id: remoteRecordId,
          fieldName: '备注/收获',
          field_name: '备注/收获',
          value: notes
        });

        log('API', '飞书返回', { status: result.success ? 200 : 500, data: result });
        return result;
      };

      const findRefreshedTask = (snapshot: Task): Task | undefined => {
        const byDirectId = this.tasks.find(
          (task) =>
            task.record_id === snapshot.record_id ||
            task.id === snapshot.id ||
            (!!snapshot.feishu_record_id && task.record_id === snapshot.feishu_record_id)
        );
        if (byDirectId) return byDirectId;

        const sourceCreated = timestampMs(snapshot.created_at);
        return this.tasks.find((task) => {
          if (task.name !== snapshot.name) return false;
          const delta = Math.abs(timestampMs(task.created_at) - sourceCreated);
          return delta <= 60_000;
        });
      };

      try {
        if (this.mode === 'local') {
          const updated = await invoke<Task>('update_local_task', {
            id: target.id || target.record_id,
            fields: { notes }
          });
          this.setTaskPatch(recordId, {
            ...updated,
            record_id: updated.record_id || recordId,
            id: updated.id || updated.record_id || recordId,
            sync_status: 'synced'
          });
          this.notesSyncState[recordId] = 'success';
          return;
        }

        let result = await tryUpdateRemoteNotes(target.record_id);

        if (!result.success && isRecordIdNotFound(result.message || '')) {
          await this.triggerSync();
          const refreshed = findRefreshedTask(target);
          if (refreshed) {
            result = await tryUpdateRemoteNotes(refreshed.record_id);
            if (result.success && refreshed.record_id !== recordId) {
              this.notesSyncState[refreshed.record_id] = 'success';
            }
          } else {
            this.notesSyncState[recordId] = 'pending';
            return;
          }
        }

        if (!result.success) {
          const message = result.message || '飞书保存失败';
          const patch = failedSyncPatch(message);
          this.setTaskPatch(recordId, patch);
          this.notesSyncState[recordId] = patch.sync_status === 'failed' ? 'error' : 'pending';
          this.offlineMode = true;
          this.error = message;
          if (patch.sync_status === 'pending') this.scheduleSyncAfterWrite();
          return;
        }

        this.setTaskPatch(recordId, { sync_status: 'synced' });
        this.notesSyncState[recordId] = 'success';
        this.offlineMode = false;
        this.lastSyncTime = Date.now();
        this.scheduleSyncAfterWrite();
      } catch (error) {
        this.setTaskPatch(recordId, previous);
        this.notesSyncState[recordId] = 'error';
        log('API', '飞书报错', { error: String(error) });
        throw error;
      }
    },

    async updateTaskName(recordId: string, name: string) {
      const trimmed = name.trim();
      if (!trimmed) throw new Error('任务名称不能为空');

      const target = findTaskByRecordId(this.tasks, recordId);
      if (!target) throw new Error('任务不存在');

      const previous = { ...target };
      this.setTaskPatch(recordId, {
        name: trimmed,
        sync_status: this.mode === 'feishu' ? 'pending' : 'synced'
      });

      const tryUpdateRemoteName = async (remoteRecordId: string) => {
        log('API', '调用飞书', {
          url: FEISHU_UPDATE_URL,
          method: 'PUT',
          record_id: remoteRecordId,
          field: '任务名称'
        });

        const result = await invoke<UpdateTaskResult>('update_task', {
          recordId: remoteRecordId,
          record_id: remoteRecordId,
          fieldName: '任务名称',
          field_name: '任务名称',
          value: trimmed
        });

        log('API', '飞书返回', { status: result.success ? 200 : 500, data: result });
        return result;
      };

      try {
        if (this.mode === 'local') {
          const updated = await invoke<Task>('update_local_task', {
            id: target.id || target.record_id,
            fields: { name: trimmed }
          });
          this.setTaskPatch(recordId, {
            ...updated,
            record_id: updated.record_id || recordId,
            id: updated.id || updated.record_id || recordId,
            sync_status: 'synced'
          });
          return;
        }

        let result = await tryUpdateRemoteName(target.record_id);

        if (!result.success && isRecordIdNotFound(result.message || '')) {
          await this.triggerSync();
          const refreshed = findRefreshedTaskBySnapshot(this.tasks, target);
          if (refreshed) {
            result = await tryUpdateRemoteName(refreshed.record_id);
          } else {
            return;
          }
        }

        if (!result.success) {
          const message = result.message || '飞书保存失败';
          const patch = failedSyncPatch(message);
          this.setTaskPatch(recordId, patch);
          this.offlineMode = true;
          this.error = message;
          if (patch.sync_status === 'pending') this.scheduleSyncAfterWrite();
          return;
        }

        this.setTaskPatch(recordId, { sync_status: 'synced' });
        this.offlineMode = false;
        this.lastSyncTime = Date.now();
        this.scheduleSyncAfterWrite();
      } catch (error) {
        this.setTaskPatch(recordId, previous);
        log('API', '飞书报错', { error: String(error) });
        throw error;
      }
    },

    async updateTaskPriority(recordId: string, priority: string) {
      const normalizedPriority = normalizePriority(priority || '普通');
      const target = findTaskByRecordId(this.tasks, recordId);
      if (!target) throw new Error('任务不存在');

      const previous = { ...target };
      this.setTaskPatch(recordId, {
        priority: normalizedPriority,
        sync_status: this.mode === 'feishu' ? 'pending' : 'synced'
      });
      this.tasks = sortTasksByPolicy(this.tasks);

      const tryUpdateRemotePriority = async (remoteRecordId: string) => {
        log('API', '调用飞书', {
          url: FEISHU_UPDATE_URL,
          method: 'PUT',
          record_id: remoteRecordId,
          field: '优先级'
        });

        const result = await invoke<UpdateTaskResult>('update_task', {
          recordId: remoteRecordId,
          record_id: remoteRecordId,
          fieldName: '优先级',
          field_name: '优先级',
          value: toFeishuPriority(normalizedPriority)
        });

        log('API', '飞书返回', { status: result.success ? 200 : 500, data: result });
        return result;
      };

      try {
        if (this.mode === 'local') {
          const updated = await invoke<Task>('update_local_task', {
            id: target.id || target.record_id,
            fields: { priority: normalizedPriority }
          });
          this.setTaskPatch(recordId, {
            ...updated,
            record_id: updated.record_id || recordId,
            id: updated.id || updated.record_id || recordId,
            sync_status: 'synced'
          });
          this.tasks = sortTasksByPolicy(this.tasks);
          return;
        }

        let result = await tryUpdateRemotePriority(target.record_id);

        if (!result.success && isRecordIdNotFound(result.message || '')) {
          await this.triggerSync();
          const refreshed = findRefreshedTaskBySnapshot(this.tasks, target);
          if (refreshed) {
            result = await tryUpdateRemotePriority(refreshed.record_id);
          } else {
            return;
          }
        }

        if (!result.success) {
          const message = result.message || '飞书保存失败';
          const patch = failedSyncPatch(message);
          this.setTaskPatch(recordId, patch);
          this.tasks = sortTasksByPolicy(this.tasks);
          this.offlineMode = true;
          this.error = message;
          if (patch.sync_status === 'pending') this.scheduleSyncAfterWrite();
          return;
        }

        this.setTaskPatch(recordId, { sync_status: 'synced' });
        this.tasks = sortTasksByPolicy(this.tasks);
        this.offlineMode = false;
        this.lastSyncTime = Date.now();
        this.scheduleSyncAfterWrite();
      } catch (error) {
        this.setTaskPatch(recordId, previous);
        this.tasks = sortTasksByPolicy(this.tasks);
        log('API', '飞书报错', { error: String(error) });
        throw error;
      }
    },

    async createTask(input: string | CreateTaskInput, priority = '普通', taskType = '日常事务') {
      const taskInput: CreateTaskInput =
        typeof input === 'string'
          ? { name: input, priority, task_type: taskType, status: '待处理' }
          : input;

      const trimmed = taskInput.name.trim();
      if (!trimmed) throw new Error('任务名称不能为空');

      const normalizedPriority = normalizePriority(taskInput.priority || '普通');
      const feishuPriority = toFeishuPriority(normalizedPriority);
      const normalizedType =
        this.mode === 'local'
          ? (taskInput.task_type || '日常事务').trim() || '日常事务'
          : '';
      const normalizedStatus = (taskInput.status || '待处理').trim() || '待处理';
      const dueDate = (taskInput.due_date || '').trim();
      const notes = (taskInput.notes || '').trim();
      const subTasks = normalizeSubTasks(taskInput.sub_tasks || []);
      const tags = normalizeTags(taskInput.tags || []);
      const recurrenceRule = taskInput.recurrence_rule || null;
      const recurrenceParentId = (taskInput.recurrence_parent_id || '').trim();
      const recurrenceIndex = taskInput.recurrence_index ?? null;
      const reminderBefore = taskInput.reminder_before ?? null;

      if (this.mode === 'local') {
        let created: Task;
        try {
          created = await invoke<Task>('create_local_task', {
            name: trimmed,
            priority: normalizedPriority,
            taskType: normalizedType
          });
        } catch (firstError) {
          created = await invoke<Task>('create_local_task', {
            name: trimmed,
            priority: normalizedPriority,
            task_type: normalizedType
          });
          log('API', '本地创建参数兼容重试成功', { error: String(firstError) });
        }
        const normalizedCreated = normalizeTask(created);
        this.tasks.unshift(normalizedCreated);
        const patch: Partial<Task> = {};
        const fields: Record<string, string> = {};

        if (dueDate) {
          patch.due_date = dueDate;
          fields.due_date = dueDate;
        }
        if (notes) {
          patch.notes = notes;
          fields.notes = notes;
        }
        if (subTasks.length) {
          patch.sub_tasks = subTasks;
          fields.sub_tasks = JSON.stringify(subTasks);
        }
        if (tags.length) {
          patch.tags = tags;
          fields.tags = JSON.stringify(tags);
        }
        if (recurrenceRule) {
          patch.recurrence_rule = recurrenceRule;
          fields.recurrence_rule = JSON.stringify(recurrenceRule);
        }
        if (recurrenceParentId || recurrenceIndex !== null || reminderBefore !== null) {
          patch.recurrence_parent_id = recurrenceParentId;
          patch.recurrence_index = recurrenceIndex;
          patch.reminder_before = reminderBefore;
          patch.reminder_notified = false;
          fields.recurrence_parent_id = recurrenceParentId;
          fields.recurrence_index = recurrenceIndex === null ? '' : String(recurrenceIndex);
          fields.reminder_before = reminderBefore === null ? '' : String(reminderBefore);
          fields.reminder_notified = '0';
        }

        if (Object.keys(fields).length > 0) {
          try {
            await this.updateTaskLocalFields(
              normalizedCreated.record_id,
              patch,
              fields
            );
            this.rememberTags(tags);
          } catch (error) {
            this.tasks = this.tasks.filter((task) => task.record_id !== normalizedCreated.record_id);
            try {
              await invoke<boolean>('delete_local_task', { id: normalizedCreated.id || normalizedCreated.record_id });
            } catch (deleteError) {
              log('API', '本地新任务回滚删除失败', { error: String(deleteError) });
            }
            log('API', '本地新任务附加字段保存失败', { error: String(error) });
            throw error;
          }
        }
        if (!Object.keys(fields).length) {
          this.rememberTags(tags);
        }
        return created.record_id;
      }

      const tempId = `temp-${Date.now()}`;
      const optimistic: Task = {
        id: tempId,
        record_id: tempId,
        name: trimmed,
        status: normalizedStatus,
        priority: normalizedPriority,
        task_type: normalizedType,
        time_spent: '',
        created_at: nowUnixSecondsString(),
        updated_at: nowUnixSecondsString(),
        completed_at: '',
        notes: '',
        sort_order: 0,
        sub_tasks: subTasks,
        tags,
        due_date: dueDate,
        recurrence_rule: recurrenceRule,
        recurrence_parent_id: recurrenceParentId,
        recurrence_index: recurrenceIndex,
        reminder_before: reminderBefore,
        reminder_notified: false,
        source: 'feishu',
        feishu_record_id: '',
        sync_status: 'pending',
        last_synced_at: '',
        retry_count: 0,
        last_error: '',
        last_retry_at: ''
      };

      this.tasks.unshift(optimistic);

      try {
        log('API', '调用飞书', { url: FEISHU_RECORDS_URL, method: 'POST' });
        const result = await invoke<CreateTaskResult>('create_task', { name: trimmed });
        log('API', '飞书返回', { status: result.synced ? 200 : 202, data: result });

        const idx = this.tasks.findIndex((task) => task.record_id === tempId);
        if (idx >= 0) {
          this.tasks[idx] = {
            ...this.tasks[idx],
            id: result.record_id,
            record_id: result.record_id,
            sync_status: result.synced ? 'synced' : 'pending'
          };
        }

        const syncField = async (fieldName: string, value: string) => {
          const fieldValue = fieldName === '优先级' ? toFeishuPriority(value) : value;
          const fieldResult = await invoke<UpdateTaskResult>('update_task', {
            recordId: result.record_id,
            record_id: result.record_id,
            fieldName,
            field_name: fieldName,
            value: fieldValue
          });
          if (!fieldResult.success) throw new Error(fieldResult.message || `${fieldName} 同步失败`);
        };

        try {
          await syncField('优先级', feishuPriority);
          if (normalizedStatus && normalizedStatus !== '待处理') await syncField('状态', normalizedStatus);
        } catch (error) {
          const message = `新任务字段同步失败：${String(error)}`;
          const patch = failedSyncPatch(message);
          this.setTaskPatch(result.record_id, patch);
          this.offlineMode = true;
          this.error = message;
          if (patch.sync_status === 'pending') this.scheduleSyncAfterWrite();
          log('API', '飞书报错', { error: message });
        }

        if (result.synced) {
          this.offlineMode = false;
          this.lastSyncTime = Date.now();
        } else {
          this.offlineMode = true;
        }
        const localPatch: Partial<Task> = {};
        const localFields: Record<string, string> = {};
        if (dueDate) {
          localPatch.due_date = dueDate;
          localFields.due_date = dueDate;
        }
        if (notes || subTasks.length || tags.length || recurrenceRule || recurrenceParentId || recurrenceIndex || reminderBefore !== null) {
          localPatch.notes = notes;
          localPatch.sub_tasks = subTasks;
          localPatch.tags = tags;
          localPatch.recurrence_rule = recurrenceRule;
          localPatch.recurrence_parent_id = recurrenceParentId;
          localPatch.recurrence_index = recurrenceIndex;
          localPatch.reminder_before = reminderBefore;
          localPatch.reminder_notified = false;
          localFields.notes = notes;
          localFields.sub_tasks = JSON.stringify(subTasks);
          localFields.tags = JSON.stringify(tags);
          localFields.recurrence_rule = recurrenceRule ? JSON.stringify(recurrenceRule) : '';
          localFields.recurrence_parent_id = recurrenceParentId;
          localFields.recurrence_index = recurrenceIndex === null ? '' : String(recurrenceIndex);
          localFields.reminder_before = reminderBefore === null ? '' : String(reminderBefore);
          localFields.reminder_notified = '0';
        }
        if (Object.keys(localFields).length > 0) {
          try {
            const updated = await this.updateTaskLocalFields(result.record_id, localPatch, localFields);
            this.rememberTags(tags);
            if (updated.sync_status === 'pending') {
              this.scheduleSyncAfterWrite();
            }
          } catch (error) {
            log('API', '飞书新任务本地附加字段保存失败', { error: String(error) });
          }
        }
        if (!Object.keys(localFields).length) {
          this.rememberTags(tags);
        }
        if (!result.synced) {
          this.scheduleSyncAfterWrite();
        }

        return result.record_id;
      } catch (error) {
        this.tasks = this.tasks.filter((task) => task.record_id !== tempId);
        log('API', '飞书报错', { error: String(error) });
        throw error;
      }
    },

    async deleteTask(recordId: string, options: { stopFutureRepeats?: boolean } = {}) {
      const target = this.tasks.find((task) => task.record_id === recordId || task.id === recordId);
      if (!target) return;
      const snapshot = [...this.tasks];

      const parentId = (target.recurrence_parent_id || '').trim();
      const parent = parentId ? findTaskByRecordId(this.tasks, parentId) : undefined;
      const shouldStopParent = Boolean(
        options.stopFutureRepeats && parent && parent.record_id !== recordId && parent.id !== recordId
      );
      if (options.stopFutureRepeats && parentId && !parent) {
        throw new Error('找不到原始重复任务，无法停止后续重复');
      }
      if (shouldStopParent && parent) {
        this.setTaskPatch(parent.record_id || parent.id || parentId, { recurrence_rule: null });
      }
      this.tasks = this.tasks.filter((task) => task.record_id !== recordId);

      try {
        if (shouldStopParent && parent) {
          await invoke<Task>('update_local_task', {
            id: parent.id || parent.record_id,
            fields: { recurrence_rule: '' }
          });
        }

        if (this.mode === 'local') {
          log('API', '删除本地任务', { recordId: target.record_id });
          await invoke<boolean>('delete_local_task', { id: target.id || target.record_id });
          return;
        }

        log('API', '调用飞书', { method: 'DELETE', record_id: target.record_id });
        const result = await invoke<UpdateTaskResult>('delete_task', {
          recordId: target.record_id,
          record_id: target.record_id
        });
        log('API', '飞书返回', { status: result.success ? 200 : 500, data: result });
        if (!result.success) {
          throw new Error(result.message || '删除失败');
        }
        this.scheduleSyncAfterWrite();
      } catch (error) {
        if (shouldStopParent && parent) {
          try {
            await invoke<Task>('update_local_task', {
              id: parent.id || parent.record_id,
              fields: {
                recurrence_rule: parent.recurrence_rule ? JSON.stringify(parent.recurrence_rule) : ''
              }
            });
          } catch (rollbackError) {
            log('API', '停止重复回滚失败', { error: String(rollbackError) });
          }
        }
        this.tasks = snapshot;
        throw error;
      }
    }
  }
});
