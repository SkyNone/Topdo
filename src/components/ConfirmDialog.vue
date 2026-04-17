<template>
  <div v-if="modelValue" class="absolute inset-0 z-50 flex items-center justify-center bg-black/25 p-4">
    <div class="w-full max-w-[280px] rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] p-4 shadow-lg">
      <h3 class="text-sm font-semibold text-[color:var(--text-primary)]">{{ title }}</h3>
      <p class="mt-2 text-xs text-[color:var(--text-secondary)]">{{ message }}</p>

      <div class="mt-4 flex justify-end gap-2">
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
  (event: 'cancel'): void;
}>();

function onCancel() {
  emit('update:modelValue', false);
  emit('cancel');
}

function onConfirm() {
  emit('confirm');
}
</script>

<style scoped>
.btn {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-solid);
  color: var(--text-secondary);
  font-size: 12px;
  padding: 6px 10px;
}

.btn-primary {
  background: var(--primary);
  border-color: var(--primary);
  color: #fff;
}

.btn-danger {
  background: #ff453a;
  border-color: #ff453a;
  color: #fff;
}
</style>
