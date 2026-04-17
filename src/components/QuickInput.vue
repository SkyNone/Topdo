<template>
  <div ref="panelRef" class="quick-input">
    <div class="input-row">
      <input
        ref="inputRef"
        v-model="taskName"
        class="task-input"
        placeholder="输入任务名称，按回车创建..."
        @keydown.enter.prevent="handleCreate"
        @keydown.esc.prevent="handleCancel"
      />
    </div>

    <div class="options-panel">
      <div class="option-row">
        <span class="option-label">优先级</span>
        <div class="priority-options">
          <button
            v-for="p in priorities"
            :key="p.value"
            type="button"
            class="priority-btn"
            :class="[
              `priority-btn--${p.tone}`,
              { active: selectedPriority === p.value }
            ]"
            @click="selectedPriority = p.value"
          >
            <span class="priority-dot" :style="{ background: p.color }"></span>
            {{ p.label }}
          </button>
        </div>
      </div>
    </div>

    <div class="input-actions">
      <button type="button" class="btn-cancel" @click="handleCancel">取消</button>
      <button type="button" class="btn-create" :disabled="!taskName.trim() || submitting" @click="handleCreate">
        创建
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import { useTaskStore } from '../stores/taskStore';

const emit = defineEmits<{
  (event: 'close'): void;
  (event: 'created'): void;
  (event: 'error', message: string): void;
}>();

const taskStore = useTaskStore();

const panelRef = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLInputElement | null>(null);
const taskName = ref('');
const selectedPriority = ref('普通');
const submitting = ref(false);

const priorities = [
  { value: '普通', label: '普通', color: '#C7C7CC', tone: 'normal' },
  { value: '重要', label: '重要', color: '#007AFF', tone: 'important' },
  { value: '紧急', label: '紧急', color: '#FF3B30', tone: 'urgent' }
];

function reset() {
  taskName.value = '';
  selectedPriority.value = '普通';
}

async function handleCreate() {
  const name = taskName.value.trim();
  if (!name || submitting.value) return;

  const payload = {
    name,
    priority: selectedPriority.value,
    status: '待处理'
  };

  submitting.value = true;
  reset();
  emit('created');
  emit('close');

  try {
    await taskStore.createTask(payload);
  } catch (error) {
    emit('error', `创建任务失败：${String(error)}`);
  } finally {
    submitting.value = false;
  }
}

function handleCancel() {
  reset();
  emit('close');
}

function onGlobalMouseDown(event: MouseEvent) {
  const target = event.target as Node | null;
  if (!target || !panelRef.value) return;
  if (!panelRef.value.contains(target)) {
    handleCancel();
  }
}

onMounted(() => {
  nextTick(() => inputRef.value?.focus());
  document.addEventListener('mousedown', onGlobalMouseDown);
});

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onGlobalMouseDown);
});
</script>

<style scoped>
.quick-input {
  margin: 6px 10px;
  padding: 12px;
  background: var(--bg-solid);
  border-radius: var(--radius-card);
  border: 1px solid var(--primary);
  box-shadow: 0 2px 12px rgba(0, 122, 255, 0.08);
}

.input-row {
  position: relative;
}

.task-input {
  width: 100%;
  height: 32px;
  padding: 0 10px;
  font-size: var(--font-size-base);
  font-family: var(--font-family);
  color: var(--text-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-btn);
  background: var(--bg-secondary);
  outline: none;
  transition: all 0.15s ease;
}

.task-input:focus {
  border-color: var(--primary);
  background: var(--bg-solid);
  box-shadow: var(--shadow-focus);
}

.task-input::placeholder {
  color: var(--text-placeholder);
  font-size: var(--font-size-sm);
}

.options-panel {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 0.5px solid var(--border-light);
}

.option-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.option-label {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  min-width: 42px;
  flex-shrink: 0;
}

.priority-options {
  display: flex;
  gap: 4px;
}

.priority-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 10px;
  font-size: var(--font-size-sm);
  font-family: var(--font-family);
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

.priority-btn.active {
  color: var(--text-secondary);
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
  color: var(--primary);
  background: color-mix(in srgb, var(--primary) 12%, var(--bg-solid));
  border-color: color-mix(in srgb, var(--primary) 45%, var(--border));
}

.priority-btn--urgent.active {
  color: #d93025;
  background: color-mix(in srgb, #ff3b30 12%, var(--bg-solid));
  border-color: color-mix(in srgb, #ff3b30 45%, var(--border));
}

.input-actions {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}

.btn-cancel {
  padding: 4px 12px;
  font-size: var(--font-size-sm);
  font-family: var(--font-family);
  color: var(--text-secondary);
  background: none;
  border: none;
  border-radius: var(--radius-btn);
  cursor: pointer;
}

.btn-cancel:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.btn-create {
  padding: 4px 16px;
  height: 28px;
  font-size: var(--font-size-sm);
  font-family: var(--font-family);
  font-weight: 500;
  color: white;
  background: var(--primary);
  border: none;
  border-radius: var(--radius-btn);
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-create:hover {
  background: var(--primary-hover);
}

.btn-create:disabled {
  background: var(--bg-tertiary);
  color: var(--text-tertiary);
  cursor: not-allowed;
}
</style>
