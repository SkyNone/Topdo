<template>
  <div v-if="modelValue" class="absolute inset-0 z-50 flex items-center justify-center bg-black/25 p-4">
    <div class="w-full max-w-[320px] rounded-[14px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-4 shadow-lg">
      <h3 class="text-sm font-semibold text-[color:var(--text-primary)]">{{ title }}</h3>
      <p class="mt-2 text-xs text-[color:var(--text-secondary)]">{{ message }}</p>

      <div v-if="secondaryConfirmText" class="dialog-actions dialog-actions-stacked">
        <button type="button" class="btn btn-danger" @click="onConfirm">
          {{ confirmText }}
        </button>
        <button type="button" class="btn btn-secondary" @click="onSecondaryConfirm">
          {{ secondaryConfirmText }}
        </button>
        <button type="button" class="btn" @click="onCancel">{{ cancelText }}</button>
      </div>

      <div v-else class="dialog-actions dialog-actions-inline">
        <button type="button" class="btn" @click="onCancel">{{ cancelText }}</button>
        <button type="button" class="btn" :class="danger ? 'btn-danger' : 'btn-primary'" @click="onConfirm">
          {{ confirmText }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    title: string;
    message: string;
    confirmText?: string;
    secondaryConfirmText?: string;
    cancelText?: string;
    danger?: boolean;
  }>(),
  {
    confirmText: '确认',
    cancelText: '取消',
    danger: false
  }
);

const emit = defineEmits<{
  (event: 'update:modelValue', value: boolean): void;
  (event: 'confirm'): void;
  (event: 'secondary-confirm'): void;
  (event: 'cancel'): void;
}>();

function onCancel() {
  emit('update:modelValue', false);
  emit('cancel');
}

function onConfirm() {
  emit('confirm');
}

function onSecondaryConfirm() {
  emit('secondary-confirm');
}
</script>

<style scoped>
.btn {
  min-height: 34px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-solid);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  padding: 7px 12px;
  white-space: nowrap;
}

.dialog-actions {
  margin-top: 16px;
}

.dialog-actions-inline {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.dialog-actions-stacked {
  display: grid;
  gap: 8px;
}

.dialog-actions-stacked .btn {
  width: 100%;
}

.btn-primary {
  background: var(--primary);
  border-color: var(--primary);
  color: #fff;
}

.btn-secondary {
  background: rgba(0, 122, 255, 0.08);
  border-color: rgba(0, 122, 255, 0.22);
  color: var(--primary);
}

.btn-danger {
  background: #ff453a;
  border-color: #ff453a;
  color: #fff;
}
</style>
