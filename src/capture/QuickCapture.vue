<template>
  <main class="quick-capture">
    <div class="capture-icon"><Icon name="add" :size="16" /></div>
    <input
      ref="inputRef"
      v-model="taskName"
      class="capture-input"
      placeholder="输入任务名，回车创建..."
      @keydown.enter.prevent="handleCreate"
      @keydown.escape.prevent="handleClose"
    />
    <kbd>{{ shortcutLabel }}</kbd>
  </main>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { emit as emitEvent, listen } from '@tauri-apps/api/event';
import { nextTick, onMounted, onUnmounted, ref } from 'vue';
import Icon from '../components/Icon.vue';
import { useTaskStore } from '../stores/taskStore';
import { buildDueDateValue } from '../utils/dueDate';

const taskStore = useTaskStore();
const taskName = ref('');
const inputRef = ref<HTMLInputElement | null>(null);
const shortcutLabel = '⌥Space';
let unlisten: (() => void) | null = null;

function todayKey(): string {
  const date = new Date();
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

async function handleCreate() {
  const name = taskName.value.trim();
  if (!name) return;
  try {
    await taskStore.createTask({ name, priority: '普通', status: '待处理', due_date: buildDueDateValue(todayKey(), '') });
    await emitEvent('tasks-updated');
    taskName.value = '';
    await handleClose();
  } catch (error) {
    console.error('快速新建任务失败:', error);
  }
}

async function handleClose() {
  taskName.value = '';
  await getCurrentWindow().hide();
}

async function focusInput() {
  await nextTick();
  inputRef.value?.focus();
  inputRef.value?.select();
}

onMounted(async () => {
  await taskStore.initMode();
  await taskStore.fetchTasks().catch(() => undefined);
  unlisten = await listen('quick-capture-focus', focusInput);
  await focusInput();
});

onUnmounted(() => unlisten?.());
</script>

<style scoped>
.quick-capture {
  width: 560px;
  height: 56px;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 20px;
  border: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
  border-radius: 12px;
  background: color-mix(in srgb, var(--bg-solid) 95%, transparent);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12);
  backdrop-filter: blur(20px);
}
.capture-icon { width: 24px; height: 24px; display: grid; place-items: center; border-radius: 7px; color: white; background: var(--accent-blue); }
.capture-input { flex: 1; border: 0; outline: none; background: transparent; color: var(--text-primary); font-family: var(--font-family); font-size: 16px; }
kbd { padding: 3px 7px; border: 1px solid var(--border); border-radius: 6px; background: var(--bg-secondary); color: var(--text-tertiary); font-size: 11px; }
</style>
