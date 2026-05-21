<template>
  <section class="flex min-h-0 flex-1 flex-col px-3 pb-2 pt-0">
    <Transition name="create-panel">
      <QuickInput
        v-if="creating"
        :template="createTemplate"
        @close="emit('cancel-create')"
        @created="emit('created', $event)"
        @error="emit('error', $event)"
      />
    </Transition>

    <div v-if="displayedTasks.length === 0 && !creating" class="empty-state">
      <div class="empty-icon">{{ emptyIcon }}</div>
      <div class="empty-text">{{ emptyText }}</div>
      <div v-if="showStarterTemplates" class="starter-templates">
        <p>可以从一个模板开始：</p>
        <button
          v-for="template in starterTemplates"
          :key="template.key"
          type="button"
          class="starter-template"
          @click="onStarterTemplate(template)"
        >
          <span class="starter-template__icon">{{ template.icon }}</span>
          <span>
            <strong>{{ template.title }}</strong>
            <small>{{ template.description }}</small>
          </span>
          <span class="starter-template__arrow">›</span>
        </button>
      </div>
    </div>

    <div v-else class="task-list" :class="{ dragging: Boolean(draggingTaskId) }">
      <div v-if="draggingTaskId" class="drag-guidance">按住手柄排序，仅调整当前优先级分组内顺序</div>
      <div
        v-for="group in groupedTasks"
        :key="group.key"
        class="task-group"
        :data-group-key="group.key"
        :data-group-priority="group.priority"
        :data-droppable="group.droppable ? 'true' : 'false'"
        :class="{
          'drag-over': canDropInGroup(group) && dragOverGroup === group.key,
          'drag-disabled': draggingTaskId && group.droppable && !canDropInGroup(group)
        }"
        @dragover.prevent="canDropInGroup(group) && onGroupDragOver(group, $event)"
        @drop.prevent="canDropInGroup(group) && onDropOnGroup(group.priority)"
      >
        <div class="task-group__header">
          <span>{{ group.label }}</span>
          <span>{{ group.tasks.length }}</span>
        </div>
        <div
          v-for="task in group.tasks"
          :key="task.record_id"
          :data-task-id="task.record_id"
          class="task-draggable"
          :class="{
            dragging: draggingTaskId === task.record_id,
            'pointer-dragging': pointerDragActive && draggingTaskId === task.record_id,
            'drop-before': dragOverTaskId === task.record_id && dragOverPlacement === 'before',
            'drop-after': dragOverTaskId === task.record_id && dragOverPlacement === 'after'
          }"
          @click.capture="onTaskPointerClickCapture(task.record_id, $event)"
          @dragover.prevent.stop="canDropInGroup(group) && onTaskDragOver(group, task.record_id, $event)"
          @drop.prevent.stop="canDropInGroup(group) && onDropOnTask(group, task.record_id)"
        >
          <div
            v-if="group.droppable"
            role="button"
            tabindex="0"
            class="task-drag-handle"
            title="拖动调整当前分组内顺序"
            aria-label="拖动调整当前分组内顺序"
            @click.stop
            @pointerdown.stop="onHandlePointerDown(task, group, $event)"
            @mousedown.stop="onHandleMouseDown(task, group, $event)"
          >
            <span></span>
            <span></span>
            <span></span>
          </div>
          <TaskItem
            :ref="(el) => setTaskItemRef(task.record_id, el)"
            :task="task"
            :mode="mode"
            :focused="focusedTaskId === task.record_id"
            :status-sync="statusSyncState[task.record_id] || 'idle'"
            :notes-sync="notesSyncState[task.record_id] || 'idle'"
            @error="emit('error', $event)"
            @focus="setFocusedTask"
            @request-delete="emit('request-delete', $event)"
          />
        </div>
        <div
          v-if="canDropInGroup(group)"
          class="task-drop-tail"
          :class="{ active: isGroupTailActive(group) }"
          @dragover.prevent="onGroupTailDragOver(group, $event)"
          @drop.prevent="onDropOnGroup(group.priority)"
        >
          <span>{{ group.tasks.length ? `放到${group.label}末尾` : `放到${group.label}` }}</span>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { useTaskStore } from '../stores/taskStore';
import type { SyncState } from '../stores/taskStore';
import type { RecurrenceRule, Task } from '../types';
import QuickInput from './QuickInput.vue';
import TaskItem from './TaskItem.vue';

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

const props = defineProps<{
  mode: 'local' | 'feishu';
  creating: boolean;
  createTemplate?: QuickTaskTemplate | null;
  statusSyncState: Record<string, SyncState>;
  notesSyncState: Record<string, SyncState>;
}>();

const emit = defineEmits<{
  (event: 'cancel-create'): void;
  (event: 'created', payload: CreatedTaskPayload): void;
  (event: 'error', message: string): void;
  (event: 'request-delete', task: Task): void;
  (event: 'create-template', template: QuickTaskTemplate): void;
  (event: 'create-habit-template'): void;
}>();

const store = useTaskStore();
const displayedTasks = computed(() => store.filteredTasks);
const focusedTaskId = ref<string>('');
const draggingTaskId = ref('');
const dragSourceGroupKey = ref('');
const dragSourcePriority = ref('');
const dragOverGroup = ref('');
const dragOverTaskId = ref('');
const dragOverPlacement = ref<'before' | 'after'>('before');
const pointerDragActive = ref(false);
const itemRefs = new Map<string, any>();
const DRAG_ACTIVATION_DISTANCE = 6;
let pointerDragMoved = false;
let pointerStartX = 0;
let pointerStartY = 0;
let pointerDropPriority = '';
let suppressClickTaskId = '';
let suppressClickUntil = 0;

const starterTemplates = computed(() => [
  {
    key: 'follow-up',
    icon: '📌',
    title: '明天跟进重要事项',
    description: '重要 · 明天 10:00',
    template: {
      name: '明天跟进重要事项',
      priority: '重要',
      dueDate: offsetDateKey(1),
      dueTime: '10:00',
      reminderBefore: null,
      expand: null
    } satisfies QuickTaskTemplate
  },
  {
    key: 'weekly-report',
    icon: '↻',
    title: '每周五写周报',
    description: '重复 · 周五 18:00',
    template: {
      name: '每周五写周报',
      priority: '普通',
      dueDate: thisFridayKey(),
      dueTime: '18:00',
      recurrenceRule: { type: 'weekly', interval: 1, daysOfWeek: [5] },
      reminderBefore: 0,
      expand: null
    } satisfies QuickTaskTemplate
  },
  {
    key: 'habit-water',
    icon: '🎯',
    title: '每天喝水',
    description: '习惯模板 · 去习惯页创建',
    template: null
  }
]);

const priorityGroups = [
  { key: 'urgent', priority: '紧急', label: '紧急' },
  { key: 'important', priority: '重要', label: '重要' },
  { key: 'normal', priority: '普通', label: '普通' }
];

type TaskGroup = {
  key: string;
  priority: string;
  label: string;
  droppable: boolean;
  tasks: Task[];
};

const groupedTasks = computed(() => {
  const groups = priorityGroups
    .map((group) => ({
      ...group,
      droppable: true,
      tasks: displayedTasks.value.filter(
        (task) => normalizeTaskStatus(task.status) !== 'completed' && normalizePriority(task.priority) === group.priority
      )
    }))
    .filter((group) => group.tasks.length > 0 || store.filter !== 'done');

  const completed = displayedTasks.value.filter((task) => normalizeTaskStatus(task.status) === 'completed');
  if (completed.length > 0 || store.filter === 'done') {
    groups.push({ key: 'completed', priority: '普通', label: '已完成', droppable: false, tasks: completed });
  }
  return groups;
});

function normalizePriority(priority: string): string {
  const value = (priority || '').trim();
  if (['紧急', '今日必做', '🔴今日必做', '🔴 今日必做'].includes(value)) return '紧急';
  if (['重要', '本周完成', '🟡本周完成', '🟠本周完成', '🔵本周完成', '🟡尽快完成', '🟡重要不紧急'].includes(value)) return '重要';
  return '普通';
}

function normalizeTaskStatus(status: string): 'todo' | 'in_progress' | 'completed' | 'unknown' {
  const value = status.trim();
  if (value.includes('进行中')) return 'in_progress';
  if (value.includes('已完成')) return 'completed';
  if (value.includes('待处理') || value.includes('待办') || value.includes('待启动')) return 'todo';
  return 'unknown';
}

const emptyIcon = computed(() => {
  if (store.filter === 'done') return '🎉';
  if (store.filter === 'in_progress') return '🚀';
  return '☕';
});

const emptyText = computed(() => {
  if (store.hasActiveSearch) return '没有匹配的任务';
  if (store.filter === 'done') return '还没有已完成的任务';
  if (store.filter === 'pending') return '待启动任务已清空';
  if (store.filter === 'in_progress') return '还没有进行中的任务';
  return '今日无事，享受片刻';
});

const showStarterTemplates = computed(() => !store.hasActiveSearch && store.totalTaskCount === 0 && store.filter !== 'done');

function dateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
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

function onStarterTemplate(item: { key: string; template: QuickTaskTemplate | null }) {
  if (item.key === 'habit-water') {
    emit('create-habit-template');
    return;
  }
  if (item.template) emit('create-template', item.template);
}

function setTaskItemRef(recordId: string, el: any | null) {
  if (!el) {
    itemRefs.delete(recordId);
    return;
  }
  itemRefs.set(recordId, el);
}

function setFocusedTask(recordId: string) {
  focusedTaskId.value = recordId;
}

function ensureFocusedTask() {
  if (displayedTasks.value.length === 0) {
    focusedTaskId.value = '';
    return;
  }
  const exists = displayedTasks.value.some((task) => task.record_id === focusedTaskId.value);
  if (!exists) {
    focusedTaskId.value = displayedTasks.value[0].record_id;
  }
}

function moveFocus(step: number) {
  if (displayedTasks.value.length === 0) return;
  const currentIndex = displayedTasks.value.findIndex((task) => task.record_id === focusedTaskId.value);
  const base = currentIndex >= 0 ? currentIndex : 0;
  const next = (base + step + displayedTasks.value.length) % displayedTasks.value.length;
  focusedTaskId.value = displayedTasks.value[next].record_id;
}

async function toggleFocusedExpand() {
  if (!focusedTaskId.value) return;
  itemRefs.get(focusedTaskId.value)?.toggleExpandFromKeyboard?.();
}

async function toggleFocusedStatus() {
  if (!focusedTaskId.value) return;
  await itemRefs.get(focusedTaskId.value)?.toggleStatusFromKeyboard?.();
}

function requestDeleteFocused() {
  if (!focusedTaskId.value) return;
  itemRefs.get(focusedTaskId.value)?.requestDeleteFromKeyboard?.();
}

async function openTask(recordId: string) {
  focusedTaskId.value = recordId;
  await nextTick();
  itemRefs.get(recordId)?.openFromReminder?.();
}

function canDropInGroup(group: TaskGroup): boolean {
  return Boolean(draggingTaskId.value && group.droppable && dragSourceGroupKey.value === group.key);
}

function canDropOnGroupElement(group: HTMLElement): boolean {
  return Boolean(
    draggingTaskId.value
      && group.dataset.droppable === 'true'
      && group.dataset.groupKey === dragSourceGroupKey.value
  );
}

function startHandleDrag(task: Task, group: TaskGroup) {
  draggingTaskId.value = task.record_id;
  dragSourceGroupKey.value = group.key;
  dragSourcePriority.value = group.priority;
  dragOverGroup.value = group.key;
  pointerDropPriority = group.priority;
}

function onDragEnd() {
  draggingTaskId.value = '';
  dragSourceGroupKey.value = '';
  dragSourcePriority.value = '';
  dragOverGroup.value = '';
  dragOverTaskId.value = '';
  dragOverPlacement.value = 'before';
  pointerDragActive.value = false;
  pointerDragMoved = false;
  pointerDropPriority = '';
}

function resolveGroupDropTarget(groupElement: HTMLElement): { recordId: string; placement: 'before' | 'after' } | null {
  const items = Array.from(groupElement.querySelectorAll<HTMLElement>('.task-draggable[data-task-id]'))
    .filter((item) => item.dataset.taskId && item.dataset.taskId !== draggingTaskId.value);
  if (items.length === 0) return null;

  for (const item of items) {
    const rect = item.getBoundingClientRect();
    if (lastDragClientY.value < rect.top + rect.height / 2) {
      return { recordId: item.dataset.taskId || '', placement: 'before' };
    }
  }

  const last = items[items.length - 1];
  return { recordId: last.dataset.taskId || '', placement: 'after' };
}

const lastDragClientY = ref(0);

function onGroupDragOver(group: TaskGroup, event: DragEvent) {
  if (!canDropInGroup(group)) return;
  dragOverGroup.value = group.key;
  pointerDropPriority = group.priority;
  lastDragClientY.value = event.clientY;
  const target = event.currentTarget;
  if (!(target instanceof HTMLElement)) {
    dragOverTaskId.value = '';
    return;
  }

  const dropTarget = resolveGroupDropTarget(target);
  if (!dropTarget) {
    dragOverTaskId.value = '';
    return;
  }

  dragOverTaskId.value = dropTarget.recordId;
  dragOverPlacement.value = dropTarget.placement;
}

function onGroupTailDragOver(group: TaskGroup, event: DragEvent) {
  if (!canDropInGroup(group)) return;
  dragOverGroup.value = group.key;
  pointerDropPriority = group.priority;
  dragOverTaskId.value = '';
  dragOverPlacement.value = 'after';
  lastDragClientY.value = event.clientY;
}

function onTaskDragOver(group: TaskGroup, recordId: string, event: DragEvent) {
  if (!canDropInGroup(group)) return;
  dragOverGroup.value = group.key;
  pointerDropPriority = group.priority;
  dragOverTaskId.value = recordId;
  lastDragClientY.value = event.clientY;
  const target = event.currentTarget;
  if (!(target instanceof HTMLElement)) {
    dragOverPlacement.value = 'before';
    return;
  }
  const rect = target.getBoundingClientRect();
  dragOverPlacement.value = event.clientY > rect.top + rect.height / 2 ? 'after' : 'before';
}

function findDroppableGroupFromPoint(x: number, y: number): HTMLElement | null {
  const element = document.elementFromPoint(x, y);
  const group = element?.closest<HTMLElement>('.task-group[data-droppable="true"]') || null;
  if (!group || !canDropOnGroupElement(group)) return null;
  return group;
}

function updatePointerDropTarget(event: PointerEvent | MouseEvent) {
  lastDragClientY.value = event.clientY;
  const group = findDroppableGroupFromPoint(event.clientX, event.clientY);
  if (!group) {
    dragOverGroup.value = '';
    dragOverTaskId.value = '';
    pointerDropPriority = '';
    return;
  }

  dragOverGroup.value = group.dataset.groupKey || '';
  pointerDropPriority = group.dataset.groupPriority || '';

  const taskElement = document
    .elementFromPoint(event.clientX, event.clientY)
    ?.closest<HTMLElement>('.task-draggable[data-task-id]');
  if (taskElement?.dataset.taskId && taskElement.dataset.taskId !== draggingTaskId.value) {
    const rect = taskElement.getBoundingClientRect();
    dragOverTaskId.value = taskElement.dataset.taskId;
    dragOverPlacement.value = event.clientY > rect.top + rect.height / 2 ? 'after' : 'before';
    return;
  }

  const dropTarget = resolveGroupDropTarget(group);
  if (!dropTarget) {
    dragOverTaskId.value = '';
    return;
  }
  dragOverTaskId.value = dropTarget.recordId;
  dragOverPlacement.value = dropTarget.placement;
}

function beginPointerDrag(task: Task, group: TaskGroup, event: PointerEvent | MouseEvent) {
  if (event.button !== 0 || normalizeTaskStatus(task.status) === 'completed') return;
  event.preventDefault();
  removePointerListeners();
  startHandleDrag(task, group);
  pointerDragActive.value = true;
  pointerDragMoved = false;
  pointerStartX = event.clientX;
  pointerStartY = event.clientY;
  lastDragClientY.value = event.clientY;
  updatePointerDropTarget(event);

  const target = event.currentTarget;
  if (target instanceof HTMLElement && 'pointerId' in event) {
    target.setPointerCapture?.(event.pointerId);
  }
  window.addEventListener('pointermove', onHandlePointerMove, { passive: false });
  window.addEventListener('pointerup', onHandlePointerUp);
  window.addEventListener('pointercancel', onHandlePointerCancel);
  window.addEventListener('mousemove', onHandleMouseMove, { passive: false });
  window.addEventListener('mouseup', onHandleMouseUp);
  document.addEventListener('pointermove', onHandlePointerMove, { passive: false });
  document.addEventListener('pointerup', onHandlePointerUp);
  document.addEventListener('pointercancel', onHandlePointerCancel);
  document.addEventListener('mousemove', onHandleMouseMove, { passive: false });
  document.addEventListener('mouseup', onHandleMouseUp);
  window.addEventListener('blur', onHandlePointerCancel);
  document.addEventListener('visibilitychange', onDocumentVisibilityChange);
}

function onHandlePointerDown(task: Task, group: TaskGroup, event: PointerEvent) {
  beginPointerDrag(task, group, event);
}

function onHandleMouseDown(task: Task, group: TaskGroup, event: MouseEvent) {
  if (pointerDragActive.value) return;
  beginPointerDrag(task, group, event);
}

function onTaskPointerClickCapture(recordId: string, event: MouseEvent) {
  if (recordId !== suppressClickTaskId || Date.now() > suppressClickUntil) return;
  event.preventDefault();
  event.stopPropagation();
  suppressClickTaskId = '';
}

function onHandlePointerMove(event: PointerEvent) {
  if (!pointerDragActive.value || !draggingTaskId.value) return;
  event.preventDefault();
  const distance = Math.hypot(event.clientX - pointerStartX, event.clientY - pointerStartY);
  if (distance >= DRAG_ACTIVATION_DISTANCE) pointerDragMoved = true;
  updatePointerDropTarget(event);
}

function onHandleMouseMove(event: MouseEvent) {
  if (!pointerDragActive.value || !draggingTaskId.value) return;
  event.preventDefault();
  const distance = Math.hypot(event.clientX - pointerStartX, event.clientY - pointerStartY);
  if (distance >= DRAG_ACTIVATION_DISTANCE) pointerDragMoved = true;
  updatePointerDropTarget(event);
}

function removePointerListeners() {
  window.removeEventListener('pointermove', onHandlePointerMove);
  window.removeEventListener('pointerup', onHandlePointerUp);
  window.removeEventListener('pointercancel', onHandlePointerCancel);
  window.removeEventListener('mousemove', onHandleMouseMove);
  window.removeEventListener('mouseup', onHandleMouseUp);
  document.removeEventListener('pointermove', onHandlePointerMove);
  document.removeEventListener('pointerup', onHandlePointerUp);
  document.removeEventListener('pointercancel', onHandlePointerCancel);
  document.removeEventListener('mousemove', onHandleMouseMove);
  document.removeEventListener('mouseup', onHandleMouseUp);
  window.removeEventListener('blur', onHandlePointerCancel);
  document.removeEventListener('visibilitychange', onDocumentVisibilityChange);
}

function onHandlePointerCancel() {
  removePointerListeners();
  onDragEnd();
}

function onDocumentVisibilityChange() {
  if (document.hidden) {
    onHandlePointerCancel();
  }
}

async function onHandlePointerUp(event: PointerEvent) {
  removePointerListeners();
  if (!pointerDragActive.value || !draggingTaskId.value || !pointerDragMoved) {
    onDragEnd();
    return;
  }

  updatePointerDropTarget(event);
  const sourceId = draggingTaskId.value;
  const targetPriority = pointerDropPriority;
  const targetRecordId = dragOverTaskId.value;
  const placement = dragOverPlacement.value;
  suppressClickTaskId = sourceId;
  suppressClickUntil = Date.now() + 300;

  try {
    if (!targetPriority || targetPriority !== dragSourcePriority.value) return;
    if (targetRecordId && targetRecordId !== sourceId) {
      await store.reorderTask(sourceId, targetPriority, targetRecordId, placement);
    } else {
      await store.reorderTask(sourceId, targetPriority, '', 'end');
    }
  } catch (error) {
    emit('error', `排序失败：${String(error)}`);
  } finally {
    onDragEnd();
  }
}

async function onHandleMouseUp(event: MouseEvent) {
  removePointerListeners();
  if (!pointerDragActive.value || !draggingTaskId.value || !pointerDragMoved) {
    onDragEnd();
    return;
  }

  updatePointerDropTarget(event);
  const sourceId = draggingTaskId.value;
  const targetPriority = pointerDropPriority;
  const targetRecordId = dragOverTaskId.value;
  const placement = dragOverPlacement.value;
  suppressClickTaskId = sourceId;
  suppressClickUntil = Date.now() + 300;

  try {
    if (!targetPriority || targetPriority !== dragSourcePriority.value) return;
    if (targetRecordId && targetRecordId !== sourceId) {
      await store.reorderTask(sourceId, targetPriority, targetRecordId, placement);
    } else {
      await store.reorderTask(sourceId, targetPriority, '', 'end');
    }
  } catch (error) {
    emit('error', `排序失败：${String(error)}`);
  } finally {
    onDragEnd();
  }
}

async function onDropOnGroup(priority: string) {
  if (!draggingTaskId.value) return;
  if (priority !== dragSourcePriority.value) {
    onDragEnd();
    return;
  }
  try {
    if (dragOverTaskId.value) {
      await store.reorderTask(draggingTaskId.value, priority, dragOverTaskId.value, dragOverPlacement.value);
    } else {
      await store.reorderTask(draggingTaskId.value, priority, '', 'end');
    }
  } catch (error) {
    emit('error', `排序失败：${String(error)}`);
  } finally {
    onDragEnd();
  }
}

function isGroupTailActive(group: TaskGroup): boolean {
  if (!draggingTaskId.value || dragOverGroup.value !== group.key) return false;
  if (!dragOverTaskId.value) return true;
  const lastTask = group.tasks[group.tasks.length - 1];
  return Boolean(lastTask && dragOverTaskId.value === lastTask.record_id && dragOverPlacement.value === 'after');
}

async function onDropOnTask(group: TaskGroup, beforeRecordId: string) {
  if (!draggingTaskId.value) {
    onDragEnd();
    return;
  }
  const priority = group.priority;
  if (priority !== dragSourcePriority.value) {
    onDragEnd();
    return;
  }
  const placement = dragOverTaskId.value === beforeRecordId ? dragOverPlacement.value : 'before';
  if (draggingTaskId.value === beforeRecordId) {
    onDragEnd();
    return;
  }
  try {
    await store.reorderTask(draggingTaskId.value, priority, beforeRecordId, placement);
  } catch (error) {
    emit('error', `排序失败：${String(error)}`);
  } finally {
    onDragEnd();
  }
}

function clearFocus() {
  focusedTaskId.value = '';
}

function hasTasks() {
  return displayedTasks.value.length > 0;
}

defineExpose({
  moveFocus,
  toggleFocusedExpand,
  toggleFocusedStatus,
  requestDeleteFocused,
  openTask,
  clearFocus,
  hasTasks
});

watch(
  () => displayedTasks.value.map((task) => task.record_id).join(','),
  () => {
    ensureFocusedTask();
    nextTick(() => {
      // keep focus ring in sync
    });
  },
  { immediate: true }
);

onBeforeUnmount(() => {
  removePointerListeners();
});
</script>

<style scoped>
.create-panel-enter-active {
  transition: max-height 0.2s ease-out, opacity 0.2s ease-out, transform 0.2s ease-out;
}

.create-panel-leave-active {
  transition: max-height 0.15s ease-in, opacity 0.15s ease-in, transform 0.15s ease-in;
}

.create-panel-enter-from,
.create-panel-leave-to {
  max-height: 0;
  opacity: 0;
  transform: translateY(-6px);
}

.create-panel-enter-to,
.create-panel-leave-from {
  max-height: 520px;
  opacity: 1;
  transform: translateY(0);
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  color: var(--text-tertiary);
}

.empty-icon {
  font-size: 36px;
  margin-bottom: 12px;
  opacity: 0.7;
}

.empty-text {
  font-size: var(--font-size-md);
  font-weight: 400;
  color: var(--text-secondary);
}

.starter-templates {
  width: min(100%, 320px);
  margin-top: 18px;
  display: grid;
  gap: 8px;
}

.starter-templates p {
  margin: 0 0 2px;
  font-size: 11px;
  color: var(--text-tertiary);
}

.starter-template {
  min-height: 54px;
  padding: 9px 12px;
  display: grid;
  grid-template-columns: 34px 1fr auto;
  gap: 10px;
  align-items: center;
  border: 1px solid color-mix(in srgb, var(--border) 78%, transparent);
  border-radius: 14px;
  background: var(--bg-solid);
  color: var(--text-primary);
  text-align: left;
  cursor: pointer;
  box-shadow: var(--shadow-sm);
  transition: border-color 0.15s ease, background 0.15s ease, transform 0.15s ease;
}

.starter-template:hover {
  border-color: color-mix(in srgb, var(--primary) 42%, var(--border));
  background: color-mix(in srgb, var(--primary) 5%, var(--bg-solid));
  transform: translateY(-1px);
}

.starter-template__icon {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  border-radius: 12px;
  background: var(--bg-secondary);
  font-size: 16px;
}

.starter-template strong {
  display: block;
  font-size: 13px;
  font-weight: 650;
  line-height: 1.2;
}

.starter-template small {
  display: block;
  margin-top: 3px;
  color: var(--text-tertiary);
  font-size: 11px;
  line-height: 1.2;
}

.starter-template__arrow {
  color: var(--text-tertiary);
  font-size: 18px;
}

.task-list {
  flex: 1;
  padding: 0 0 10px;
}

.task-list.dragging {
  cursor: grabbing;
}

.drag-guidance {
  margin: 6px 8px 8px;
  padding: 7px 10px;
  border: 1px solid color-mix(in srgb, var(--primary) 28%, transparent);
  border-radius: 12px;
  background: color-mix(in srgb, var(--primary) 9%, var(--bg-solid));
  color: var(--primary);
  font-size: 11px;
  font-weight: 650;
  text-align: center;
}

.task-group {
  margin-top: 8px;
  border-radius: var(--radius-card);
  padding: 1px 0 4px;
  transition: background 0.15s ease, outline-color 0.15s ease;
}

.task-group.drag-over {
  outline: 1px solid color-mix(in srgb, var(--primary) 28%, transparent);
  background: color-mix(in srgb, var(--primary) 5%, transparent);
}

.task-group__header {
  margin: 10px 12px 2px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
}

.task-draggable.dragging,
.task-draggable.pointer-dragging {
  opacity: 0.78;
}

.task-draggable {
  position: relative;
}

.task-draggable.dragging :deep(.task-card),
.task-draggable.pointer-dragging :deep(.task-card) {
  border-color: color-mix(in srgb, var(--primary) 60%, var(--border));
  box-shadow: 0 12px 28px color-mix(in srgb, var(--primary) 22%, transparent);
  transform: scale(0.985);
}

.task-draggable :deep(.task-card) {
  padding-right: 32px;
}

.task-drag-handle {
  position: absolute;
  top: 50%;
  right: 10px;
  z-index: 4;
  width: 28px;
  height: 32px;
  display: inline-flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 3px;
  border: none;
  border-radius: 9px;
  background: transparent;
  color: var(--text-tertiary);
  cursor: grab;
  opacity: 0.62;
  transform: translateY(-50%);
  user-select: none;
  -webkit-user-drag: none;
  touch-action: none;
  transition: opacity 0.15s ease, background 0.15s ease, color 0.15s ease;
}

.task-drag-handle span {
  width: 3px;
  height: 3px;
  border-radius: 50%;
  background: currentColor;
}

.task-draggable:hover .task-drag-handle,
.task-drag-handle:focus-visible,
.task-list.dragging .task-drag-handle,
.task-draggable.dragging .task-drag-handle {
  opacity: 1;
}

.task-drag-handle:hover,
.task-drag-handle:focus-visible {
  background: color-mix(in srgb, var(--primary) 10%, transparent);
  color: var(--primary);
  outline: none;
}

.task-drag-handle:active {
  cursor: grabbing;
}

.task-draggable.drop-before::before,
.task-draggable.drop-after::after {
  content: '';
  position: absolute;
  left: 16px;
  right: 16px;
  z-index: 8;
  height: 4px;
  border-radius: 999px;
  background: var(--primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--primary) 16%, transparent),
    0 4px 12px color-mix(in srgb, var(--primary) 30%, transparent);
}

.task-draggable.drop-before::before {
  top: -3px;
}

.task-draggable.drop-after::after {
  bottom: -3px;
}

.task-drop-tail {
  min-height: 30px;
  margin: 4px 12px 2px;
  display: grid;
  place-items: center;
  border: 1px dashed color-mix(in srgb, var(--text-tertiary) 28%, transparent);
  border-radius: 14px;
  background: color-mix(in srgb, var(--bg-solid) 54%, transparent);
  color: var(--text-tertiary);
  font-size: 11px;
  font-weight: 650;
  opacity: 0.72;
  transition: border-color 0.15s ease, background 0.15s ease, color 0.15s ease, opacity 0.15s ease;
}

.task-drop-tail.active {
  border-color: color-mix(in srgb, var(--primary) 70%, transparent);
  background: color-mix(in srgb, var(--primary) 12%, var(--bg-solid));
  color: var(--primary);
  opacity: 1;
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--primary) 14%, transparent);
}
</style>
