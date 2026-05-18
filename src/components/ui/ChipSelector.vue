<template>
  <div class="chip-selector" :class="[`chip-selector--${tone}`, { 'chip-selector--compact': compact }]">
    <button
      v-for="option in options"
      :key="String(option.value)"
      type="button"
      class="chip-selector__item"
      :class="{ active: option.value === modelValue }"
      :disabled="option.disabled"
      @click="emit('update:modelValue', option.value)"
    >
      <span v-if="option.dot" class="chip-selector__dot" :style="{ background: option.dot }"></span>
      {{ option.label }}
    </button>
  </div>
</template>

<script setup lang="ts">
type ChipValue = string | number | null;

export interface ChipOption {
  label: string;
  value: ChipValue;
  disabled?: boolean;
  dot?: string;
}

withDefaults(defineProps<{
  options: ChipOption[];
  modelValue: ChipValue;
  tone?: 'blue' | 'purple' | 'amber' | 'red' | 'neutral';
  compact?: boolean;
}>(), {
  tone: 'blue',
  compact: false
});

const emit = defineEmits<{ (event: 'update:modelValue', value: ChipValue): void }>();
</script>

<style scoped>
.chip-selector {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.chip-selector--blue {
  --chip-active-bg: var(--accent-blue-soft);
  --chip-active-border: var(--accent-blue);
  --chip-active-text: var(--accent-blue);
}

.chip-selector--purple {
  --chip-active-bg: var(--accent-purple-soft);
  --chip-active-border: var(--accent-purple);
  --chip-active-text: var(--accent-purple);
}

.chip-selector--amber {
  --chip-active-bg: var(--accent-amber-soft);
  --chip-active-border: var(--accent-amber);
  --chip-active-text: var(--accent-amber);
}

.chip-selector--red {
  --chip-active-bg: var(--accent-red-soft);
  --chip-active-border: var(--accent-red);
  --chip-active-text: var(--accent-red);
}

.chip-selector--neutral {
  --chip-active-bg: var(--bg-solid);
  --chip-active-border: var(--border);
  --chip-active-text: var(--text-primary);
}

.chip-selector__item {
  min-height: 30px;
  padding: 6px 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-family: var(--font-family);
  font-size: 13px;
  font-weight: 500;
  line-height: 1;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease, color 0.15s ease, transform 0.15s ease;
}

.chip-selector--compact .chip-selector__item {
  min-height: 26px;
  padding: 5px 11px;
  font-size: 12px;
}

.chip-selector__item:hover:not(:disabled) {
  background: var(--bg-hover);
  transform: translateY(-1px);
}

.chip-selector__item.active {
  color: var(--chip-active-text);
  border-color: var(--chip-active-border);
  background: var(--chip-active-bg);
}

.chip-selector__item:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.chip-selector__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 0.5px rgba(0, 0, 0, 0.08);
}
</style>
