import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { SyncTasksResult, Task } from '../types';
import { log } from '../utils/logger';

export type AppMode = 'local' | 'feishu';
export type SyncState = 'idle' | 'loading' | 'success' | 'error' | 'pending';
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
}

const FEISHU_RECORDS_URL = '/open-apis/bitable/v1/apps/{app_token}/tables/{table_id}/records';
const FEISHU_UPDATE_URL = '/open-apis/bitable/v1/apps/{app_token}/tables/{table_id}/records/{record_id}';
let syncTimer: ReturnType<typeof setTimeout> | null = null;

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

function normalizeTask(task: Task): Task {
  return {
    ...task,
    priority: normalizePriority(task.priority || '普通')
  };
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

    return timestampMs(b.created_at) - timestampMs(a.created_at);
  });
}

function applySearchFilter(tasks: Task[], searchQuery: string): Task[] {
  const query = searchQuery.trim().toLowerCase();
  if (!query) return tasks;

  return tasks.filter((task) => {
    const name = (task.name || '').toLowerCase();
    const notes = (task.notes || '').toLowerCase();
    return name.includes(query) || notes.includes(query);
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
    filteredTasks: (state) => {
      const sorted = sortTasksByPolicy(state.tasks);
      const searched = applySearchFilter(sorted, state.searchQuery);
      switch (state.filter) {
        case 'pending':
          return searched.filter((task) => normalizeStatus(task.status) === 'todo');
        case 'in_progress':
          return searched.filter((task) => normalizeStatus(task.status) === 'in_progress');
        case 'done':
          return searched.filter((task) => normalizeStatus(task.status) === 'completed');
        case 'all':
        default:
          return searched;
      }
    },
    groupedFilteredTasks: (state): Array<{ status: TaskStatusGroup; tasks: Task[] }> => {
      const inProgress: Task[] = [];
      const pending: Task[] = [];
      const completed: Task[] = [];

      const includePending = state.filter === 'all' || state.filter === 'pending';
      const includeInProgress = state.filter === 'all' || state.filter === 'in_progress';
      const includeDone = state.filter === 'all' || state.filter === 'done';

      const sorted = applySearchFilter(sortTasksByPolicy(state.tasks), state.searchQuery);
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

    setModeState(mode: AppMode) {
      this.mode = mode;
      console.log('[Store] mode 变更为:', mode);
    },

    setTasks(tasks: Task[]) {
      if (this.mode === 'feishu') {
        this.tasks = tasks.map(normalizeTask);
        return;
      }
      this.tasks = tasks.map(normalizeTask);
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
      const previousTasks = [...this.tasks];
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
        if (this.mode === 'local') {
          const updated = await invoke<Task>('update_local_task', {
            id: target.id || target.record_id,
            fields: { status: toStatus }
          });
          this.setTaskPatch(recordId, {
            ...updated,
            record_id: updated.record_id || recordId,
            id: updated.id || updated.record_id || recordId,
            sync_status: 'synced'
          });
          this.statusSyncState[recordId] = 'success';
          return;
        }

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

        if (!result.success) throw new Error(result.message || '飞书更新失败');

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

        if (!result.success) throw new Error(result.message || '飞书保存失败');

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

        if (!result.success) throw new Error(result.message || '飞书保存失败');

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

        if (!result.success) throw new Error(result.message || '飞书保存失败');

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
        this.tasks.unshift(normalizeTask(created));
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
          await invoke<UpdateTaskResult>('update_task', {
            recordId: result.record_id,
            record_id: result.record_id,
            fieldName,
            field_name: fieldName,
            value: fieldValue
          });
        };

        try {
          await syncField('优先级', feishuPriority);
          if (normalizedStatus && normalizedStatus !== '待处理') await syncField('状态', normalizedStatus);
        } catch (error) {
          log('API', '飞书报错', { error: `新任务字段同步失败: ${String(error)}` });
        }

        if (result.synced) {
          this.offlineMode = false;
          this.lastSyncTime = Date.now();
        } else {
          this.offlineMode = true;
        }
        // 无论首轮是否成功，都在 1s 后触发一次增量同步：
        // - 成功时可拉取到其他端更新
        // - 失败时可自动重试把 pending 任务推到飞书
        this.scheduleSyncAfterWrite();

        return result.record_id;
      } catch (error) {
        this.tasks = this.tasks.filter((task) => task.record_id !== tempId);
        log('API', '飞书报错', { error: String(error) });
        throw error;
      }
    },

    async deleteTask(recordId: string) {
      const target = this.tasks.find((task) => task.record_id === recordId || task.id === recordId);
      if (!target) return;
      const snapshot = [...this.tasks];
      this.tasks = this.tasks.filter((task) => task.record_id !== recordId);

      try {
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
        this.tasks = snapshot;
        throw error;
      }
    }
  }
});
