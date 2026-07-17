<template>
  <div ref="triggerRoot" class="topdo-multi-select" :class="{ 'is-open': open, 'is-disabled': disabled }">
    <button
      ref="triggerButton"
      type="button"
      class="topdo-multi-select__trigger"
      :disabled="disabled"
      role="combobox"
      aria-haspopup="listbox"
      :aria-expanded="open"
      @click="toggle"
      @keydown="onTriggerKeydown"
    >
      <span class="topdo-multi-select__value" :class="{ placeholder: !selectedOptions.length }">{{ displayLabel }}</span>
      <span v-if="selectedOptions.length > 1" class="topdo-multi-select__count">{{ selectedOptions.length }}</span>
      <Icon class="topdo-multi-select__chevron" :class="{ open }" name="chevron-down" :size="15" />
    </button>

    <Teleport to="body">
      <Transition name="topdo-multi-menu">
        <div
          v-if="open"
          ref="menu"
          class="topdo-multi-select__menu"
          :style="menuStyle"
          role="listbox"
          aria-multiselectable="true"
          :aria-label="ariaLabel"
          @keydown="onMenuKeydown"
        >
          <div class="topdo-multi-select__options">
            <button
              v-for="(option, index) in options"
              :key="`${option.value}-${index}`"
              :ref="(element) => setOptionRef(element, index)"
              type="button"
              class="topdo-multi-select__option"
              :class="{ selected: isSelected(option.value), active: index === activeIndex }"
              :disabled="option.disabled"
              role="option"
              :aria-selected="isSelected(option.value)"
              tabindex="-1"
              @mouseenter="activeIndex = index"
              @click="toggleOption(option)"
            >
              <span class="topdo-multi-select__check">
                <Icon v-if="isSelected(option.value)" name="check-circle" :size="15" />
              </span>
              <span class="topdo-multi-select__option-label">{{ option.label }}</span>
              <span v-if="modelValue[0] === option.value" class="topdo-multi-select__default">写回</span>
            </button>
          </div>
          <div class="topdo-multi-select__footer">
            <span>已选 {{ modelValue.length }} 项</span>
            <button type="button" @click="close(true)">完成</button>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from 'vue';
import type { ComponentPublicInstance, CSSProperties } from 'vue';
import Icon from './Icon.vue';

export interface TopdoMultiSelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

const props = withDefaults(defineProps<{
  modelValue: string[];
  options: TopdoMultiSelectOption[];
  placeholder?: string;
  ariaLabel?: string;
  disabled?: boolean;
}>(), {
  placeholder: '请选择',
  ariaLabel: '选择多个选项',
  disabled: false
});

const emit = defineEmits<{
  (event: 'update:modelValue', value: string[]): void;
}>();

const triggerRoot = ref<HTMLElement | null>(null);
const triggerButton = ref<HTMLButtonElement | null>(null);
const menu = ref<HTMLElement | null>(null);
const optionRefs = ref<Array<HTMLButtonElement | null>>([]);
const open = ref(false);
const activeIndex = ref(-1);
const menuStyle = ref<CSSProperties>({});

const selectedOptions = computed(() => props.modelValue
  .map((value) => props.options.find((option) => option.value === value))
  .filter((option): option is TopdoMultiSelectOption => Boolean(option)));
const displayLabel = computed(() => selectedOptions.value.length
  ? selectedOptions.value.map((option) => option.label).join('、')
  : props.placeholder);

function isSelected(value: string): boolean {
  return props.modelValue.includes(value);
}

function setOptionRef(element: Element | ComponentPublicInstance | null, index: number) {
  optionRefs.value[index] = element instanceof HTMLButtonElement ? element : null;
}

function enabledIndexes(): number[] {
  return props.options
    .map((option, index) => (option.disabled ? -1 : index))
    .filter((index) => index >= 0);
}

function initialActiveIndex(): number {
  const selectedIndex = props.options.findIndex((option) => isSelected(option.value) && !option.disabled);
  return selectedIndex >= 0 ? selectedIndex : (enabledIndexes()[0] ?? -1);
}

function updateMenuPosition() {
  const rect = triggerButton.value?.getBoundingClientRect();
  if (!rect) return;

  const viewportPadding = 8;
  const gap = 6;
  const width = Math.max(rect.width, 210);
  const left = Math.min(Math.max(rect.left, viewportPadding), window.innerWidth - width - viewportPadding);
  const spaceBelow = window.innerHeight - rect.bottom - gap - viewportPadding;
  const spaceAbove = rect.top - gap - viewportPadding;
  const placeAbove = spaceBelow < 150 && spaceAbove > spaceBelow;
  const maxHeight = Math.max(120, Math.min(260, placeAbove ? spaceAbove : spaceBelow));

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

function toggleOption(option: TopdoMultiSelectOption) {
  if (option.disabled) return;
  const next = isSelected(option.value)
    ? props.modelValue.filter((value) => value !== option.value)
    : [...props.modelValue, option.value];
  emit('update:modelValue', next);
}

function moveActive(step: 1 | -1) {
  const indexes = enabledIndexes();
  if (!indexes.length) return;
  const currentPosition = indexes.indexOf(activeIndex.value);
  const nextPosition = currentPosition < 0 ? 0 : (currentPosition + step + indexes.length) % indexes.length;
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
  } else if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    const option = props.options[activeIndex.value];
    if (option) toggleOption(option);
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
.topdo-multi-select { width: 100%; min-width: 0; }

.topdo-multi-select__trigger {
  width: 100%;
  height: 36px;
  display: flex;
  align-items: center;
  gap: 7px;
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

.topdo-multi-select__trigger:hover {
  border-color: color-mix(in srgb, var(--primary) 30%, var(--border));
  background: color-mix(in srgb, var(--primary) 2%, var(--bg-solid));
}

.topdo-multi-select__trigger:focus-visible,
.is-open .topdo-multi-select__trigger {
  border-color: var(--primary);
  box-shadow: var(--shadow-focus);
}

.topdo-multi-select__value {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.topdo-multi-select__value.placeholder { color: var(--text-placeholder); }

.topdo-multi-select__count {
  min-width: 19px;
  height: 19px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border-radius: 999px;
  background: color-mix(in srgb, var(--primary) 11%, var(--bg-solid));
  color: var(--primary);
  font-size: 10px;
  font-weight: 700;
}

.topdo-multi-select__chevron {
  flex-shrink: 0;
  color: var(--text-tertiary);
  transition: transform 0.15s ease;
}

.topdo-multi-select__chevron.open { transform: rotate(180deg); }
.topdo-multi-select__trigger:disabled { cursor: not-allowed; opacity: 0.55; }

.topdo-multi-select__menu {
  position: fixed;
  z-index: 4000;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 5px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-solid) 98%, transparent);
  box-shadow: 0 14px 34px rgba(0, 0, 0, 0.16), var(--shadow-sm);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}

.topdo-multi-select__options { min-height: 0; overflow-y: auto; }

.topdo-multi-select__option {
  width: 100%;
  min-height: 34px;
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr) auto;
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

.topdo-multi-select__option:hover,
.topdo-multi-select__option.active,
.topdo-multi-select__option:focus-visible { background: var(--bg-hover); }

.topdo-multi-select__option.selected {
  background: color-mix(in srgb, var(--primary) 9%, var(--bg-solid));
  color: var(--primary);
  font-weight: 600;
}

.topdo-multi-select__check {
  width: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--primary);
}

.topdo-multi-select__option-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.topdo-multi-select__default {
  border-radius: 999px;
  background: color-mix(in srgb, var(--primary) 11%, var(--bg-solid));
  color: var(--primary);
  font-size: 9px;
  line-height: 18px;
  font-weight: 600;
  padding: 0 6px;
}

.topdo-multi-select__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-top: 4px;
  padding: 7px 7px 2px;
  border-top: 1px solid var(--border-light);
  color: var(--text-tertiary);
  font-size: 10px;
}

.topdo-multi-select__footer button {
  border: 0;
  background: transparent;
  color: var(--primary);
  font: inherit;
  font-weight: 650;
  cursor: pointer;
}

.topdo-multi-menu-enter-active,
.topdo-multi-menu-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
  transform-origin: top center;
}

.topdo-multi-menu-enter-from,
.topdo-multi-menu-leave-to {
  opacity: 0;
  transform: translateY(-3px) scale(0.985);
}
</style>
