<template>
  <section class="flex min-h-0 flex-1 flex-col px-3 pb-2 pt-0">
    <Transition name="create-panel">
      <QuickInput
        v-if="creating"
        @close="emit('cancel-create')"
        @created="emit('created')"
        @error="emit('error', $event)"
      />
    </Transition>

    <div v-if="displayedTasks.length === 0 && !creating" class="empty-state">
      <div class="empty-icon">{{ emptyIcon }}</div>
      <div class="empty-text">{{ emptyText }}</div>
    </div>

    <div v-else class="task-list">
      <div v-for="task in displayedTasks" :key="task.record_id">
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
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { useTaskStore } from '../stores/taskStore';
import type { SyncState } from '../stores/taskStore';
import type { Task } from '../types';
import QuickInput from './QuickInput.vue';
import TaskItem from './TaskItem.vue';

const props = defineProps<{
  mode: 'local' | 'feishu';
  creating: boolean;
  statusSyncState: Record<string, SyncState>;
  notesSyncState: Record<string, SyncState>;
}>();

const emit = defineEmits<{
  (event: 'cancel-create'): void;
  (event: 'created'): void;
  (event: 'error', message: string): void;
  (event: 'request-delete', task: Task): void;
}>();

const store = useTaskStore();
const displayedTasks = computed(() => store.filteredTasks);
const focusedTaskId = ref<string>('');
const itemRefs = new Map<string, any>();

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
  max-height: 260px;
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

.task-list {
  flex: 1;
  padding: 0 0 10px;
}

.task-list :deep(.list-none:first-child .task-card) {
  margin-top: 8px;
}
</style>
