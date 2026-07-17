<template>
  <div ref="triggerRoot" class="topdo-select" :class="{ 'is-open': open, 'is-disabled': disabled }">
    <button
      ref="triggerButton"
      type="button"
      class="topdo-select__trigger"
      :disabled="disabled"
      role="combobox"
      aria-haspopup="listbox"
      :aria-expanded="open"
      @click="toggle"
      @keydown="onTriggerKeydown"
    >
      <span class="topdo-select__value" :class="{ placeholder: !selectedOption }">{{ displayLabel }}</span>
      <Icon class="topdo-select__chevron" :class="{ open }" name="chevron-down" :size="15" />
    </button>

    <Teleport to="body">
      <Transition name="topdo-select-menu">
        <div
          v-if="open"
          ref="menu"
          class="topdo-select__menu"
          :style="menuStyle"
          role="listbox"
          :aria-label="ariaLabel"
          @keydown="onMenuKeydown"
        >
          <button
            v-for="(option, index) in options"
            :key="`${option.value}-${index}`"
            :ref="(element) => setOptionRef(element, index)"
            type="button"
            class="topdo-select__option"
            :class="{ selected: option.value === modelValue, active: index === activeIndex }"
            :disabled="option.disabled"
            role="option"
            :aria-selected="option.value === modelValue"
            tabindex="-1"
            @mouseenter="activeIndex = index"
            @click="selectOption(option)"
          >
            <span class="topdo-select__check">
              <Icon v-if="option.value === modelValue" name="check-circle" :size="15" />
            </span>
            <span class="topdo-select__option-label">{{ option.label }}</span>
          </button>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from 'vue';
import type { ComponentPublicInstance, CSSProperties } from 'vue';
import Icon from './Icon.vue';

export interface TopdoSelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

const props = withDefaults(defineProps<{
  modelValue: string;
  options: TopdoSelectOption[];
  placeholder?: string;
  ariaLabel?: string;
  disabled?: boolean;
}>(), {
  placeholder: '请选择',
  ariaLabel: '选择选项',
  disabled: false
});

const emit = defineEmits<{
  (event: 'update:modelValue', value: string): void;
  (event: 'change', value: string): void;
}>();

const triggerRoot = ref<HTMLElement | null>(null);
const triggerButton = ref<HTMLButtonElement | null>(null);
const menu = ref<HTMLElement | null>(null);
const optionRefs = ref<Array<HTMLButtonElement | null>>([]);
const open = ref(false);
const activeIndex = ref(-1);
const menuStyle = ref<CSSProperties>({});

const selectedOption = computed(() => props.options.find((option) => option.value === props.modelValue));
const displayLabel = computed(() => selectedOption.value?.label || props.placeholder);

function setOptionRef(element: Element | ComponentPublicInstance | null, index: number) {
  optionRefs.value[index] = element instanceof HTMLButtonElement ? element : null;
}

function enabledIndexes(): number[] {
  return props.options
    .map((option, index) => (option.disabled ? -1 : index))
    .filter((index) => index >= 0);
}

function initialActiveIndex(): number {
  const selectedIndex = props.options.findIndex((option) => option.value === props.modelValue && !option.disabled);
  return selectedIndex >= 0 ? selectedIndex : (enabledIndexes()[0] ?? -1);
}

function updateMenuPosition() {
  const rect = triggerButton.value?.getBoundingClientRect();
  if (!rect) return;

  const viewportPadding = 8;
  const gap = 6;
  const width = Math.max(rect.width, 180);
  const left = Math.min(Math.max(rect.left, viewportPadding), window.innerWidth - width - viewportPadding);
  const spaceBelow = window.innerHeight - rect.bottom - gap - viewportPadding;
  const spaceAbove = rect.top - gap - viewportPadding;
  const placeAbove = spaceBelow < 120 && spaceAbove > spaceBelow;
  const maxHeight = Math.max(80, Math.min(220, placeAbove ? spaceAbove : spaceBelow));

  menuStyle.value = {
    left: `${left}px`,
    top: placeAbove ? 'auto' : `${rect.bottom + gap}px`,
    bottom: placeAbove ? `${window.innerHeight - rect.top + gap}px` : 'auto',
    width: `${width}px`,
    maxHeight: `${maxHeight}px`
  };
}

function focusActiveOption() {
  nextTick(() => optionRefs.value[activeIndex.value]?.focus());
}

function addGlobalListeners() {
  document.addEventListener('pointerdown', onOutsidePointerDown);
  document.addEventListener('scroll', closeOnViewportChange, true);
  window.addEventListener('resize', closeOnViewportChange);
}

function removeGlobalListeners() {
  document.removeEventListener('pointerdown', onOutsidePointerDown);
  document.removeEventListener('scroll', closeOnViewportChange, true);
  window.removeEventListener('resize', closeOnViewportChange);
}

function show(focusOption = false) {
  if (props.disabled || !props.options.length) return;
  open.value = true;
  activeIndex.value = initialActiveIndex();
  optionRefs.value = [];
  nextTick(() => {
    updateMenuPosition();
    if (focusOption) focusActiveOption();
  });
  addGlobalListeners();
}

function close(restoreFocus = false) {
  if (!open.value) return;
  open.value = false;
  removeGlobalListeners();
  if (restoreFocus) nextTick(() => triggerButton.value?.focus());
}

function toggle() {
  if (open.value) close();
  else show();
}

function selectOption(option: TopdoSelectOption) {
  if (option.disabled) return;
  emit('update:modelValue', option.value);
  emit('change', option.value);
  close(true);
}

function moveActive(step: 1 | -1) {
  const indexes = enabledIndexes();
  if (!indexes.length) return;
  const currentPosition = indexes.indexOf(activeIndex.value);
  const nextPosition = currentPosition < 0
    ? 0
    : (currentPosition + step + indexes.length) % indexes.length;
  activeIndex.value = indexes[nextPosition];
  focusActiveOption();
}

function onTriggerKeydown(event: KeyboardEvent) {
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault();
    show(true);
  } else if (event.key === 'Escape') {
    event.preventDefault();
    close();
  }
}

function onMenuKeydown(event: KeyboardEvent) {
  if (event.key === 'ArrowDown') {
    event.preventDefault();
    moveActive(1);
  } else if (event.key === 'ArrowUp') {
    event.preventDefault();
    moveActive(-1);
  } else if (event.key === 'Home') {
    event.preventDefault();
    activeIndex.value = enabledIndexes()[0] ?? -1;
    focusActiveOption();
  } else if (event.key === 'End') {
    event.preventDefault();
    const indexes = enabledIndexes();
    activeIndex.value = indexes[indexes.length - 1] ?? -1;
    focusActiveOption();
  } else if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    const option = props.options[activeIndex.value];
    if (option) selectOption(option);
  } else if (event.key === 'Escape') {
    event.preventDefault();
    close(true);
  } else if (event.key === 'Tab') {
    close();
  }
}

function onOutsidePointerDown(event: PointerEvent) {
  const target = event.target as Node | null;
  if (target && !triggerRoot.value?.contains(target) && !menu.value?.contains(target)) close();
}

function closeOnViewportChange(event?: Event) {
  const target = event?.target;
  if (target instanceof Node && menu.value?.contains(target)) return;
  close();
}

onBeforeUnmount(removeGlobalListeners);
</script>

<style scoped>
.topdo-select {
  width: 100%;
  min-width: 0;
}

.topdo-select__trigger {
  width: 100%;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-solid);
  color: var(--text-primary);
  font-family: var(--font-family);
  font-size: 13px;
  line-height: 20px;
  text-align: left;
  padding: 0 10px 0 12px;
  outline: none;
  cursor: pointer;
  transition: border-color 0.15s ease, box-shadow 0.15s ease, background 0.15s ease;
}

.topdo-select__trigger:hover {
  border-color: color-mix(in srgb, var(--primary) 30%, var(--border));
  background: color-mix(in srgb, var(--primary) 2%, var(--bg-solid));
}

.topdo-select__trigger:focus-visible,
.is-open .topdo-select__trigger {
  border-color: var(--primary);
  box-shadow: var(--shadow-focus);
}

.topdo-select__value {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.topdo-select__value.placeholder {
  color: var(--text-placeholder);
}

.topdo-select__chevron {
  flex-shrink: 0;
  color: var(--text-tertiary);
  transition: transform 0.15s ease;
}

.topdo-select__chevron.open {
  transform: rotate(180deg);
}

.topdo-select__trigger:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.topdo-select__menu {
  position: fixed;
  z-index: 4000;
  overflow-y: auto;
  padding: 5px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-solid) 98%, transparent);
  box-shadow: 0 14px 34px rgba(0, 0, 0, 0.16), var(--shadow-sm);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}

.topdo-select__option {
  width: 100%;
  min-height: 34px;
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr);
  align-items: center;
  gap: 7px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--text-primary);
  font-family: var(--font-family);
  font-size: 12px;
  line-height: 17px;
  text-align: left;
  padding: 7px 9px;
  outline: none;
  cursor: pointer;
}

.topdo-select__option:hover,
.topdo-select__option.active,
.topdo-select__option:focus-visible {
  background: var(--bg-hover);
}

.topdo-select__option.selected {
  background: color-mix(in srgb, var(--primary) 9%, var(--bg-solid));
  color: var(--primary);
  font-weight: 600;
}

.topdo-select__option:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.topdo-select__check {
  width: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--primary);
}

.topdo-select__option-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.topdo-select-menu-enter-active,
.topdo-select-menu-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
  transform-origin: top center;
}

.topdo-select-menu-enter-from,
.topdo-select-menu-leave-to {
  opacity: 0;
  transform: translateY(-3px) scale(0.985);
}
</style>
