<template>
  <div class="mode-title" :class="{ open }" data-no-drag @click.stop="toggleDropdown" @mousedown.stop @dblclick.stop>
    <span class="mode-title__label">
      <Icon v-if="appStore.currentMode === 'habits'" name="habit-mode" :size="16" />
      <span>{{ appStore.currentMode === 'habits' ? '习惯' : 'Topdo' }}</span>
    </span>
    <Icon
      v-if="appStore.habitModuleEnabled"
      class="dropdown-arrow"
      :class="{ open }"
      name="chevron-down"
      :size="14"
    />
    <div v-if="open && appStore.habitModuleEnabled" class="dropdown-menu" data-no-drag @click.stop @mousedown.stop @dblclick.stop>
      <button type="button" class="dropdown-item" :class="{ active: appStore.currentMode === 'tasks' }" @click="switchMode('tasks')">
        <span class="check">{{ appStore.currentMode === 'tasks' ? '✓' : '' }}</span>
        <span class="item-icon"><Icon name="list" :size="18" /></span>
        <span class="item-copy"><strong>任务</strong><small>待办、循环任务、截止提醒</small></span>
      </button>
      <button type="button" class="dropdown-item" :class="{ active: appStore.currentMode === 'habits' }" @click="switchMode('habits')">
        <span class="check">{{ appStore.currentMode === 'habits' ? '✓' : '' }}</span>
        <span class="item-icon"><Icon name="habit-mode" :size="18" /></span>
        <span class="item-copy"><strong>习惯</strong><small>每日打卡、连续天数</small></span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { useAppStore } from '../stores/appStore';
import Icon from './Icon.vue';

const appStore = useAppStore();
const open = ref(false);

function toggleDropdown() {
  if (appStore.habitModuleEnabled) open.value = !open.value;
}

function switchMode(mode: 'tasks' | 'habits') {
  appStore.switchMode(mode);
  open.value = false;
}

function close() {
  open.value = false;
}

onMounted(() => document.addEventListener('click', close));
onBeforeUnmount(() => document.removeEventListener('click', close));
</script>

<style scoped>
.mode-title {
  position: relative;
  z-index: 120;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  max-width: 100%;
  min-width: 0;
  height: 30px;
  justify-content: center;
  padding: 0 10px;
  border-radius: 10px;
  cursor: default;
  -webkit-app-region: no-drag;
}
.mode-title:hover,
.mode-title.open {
  background: var(--bg-hover);
}
.mode-title__label {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
}

.mode-title__label span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.dropdown-arrow {
  color: var(--text-tertiary);
  transition: transform .15s ease;
}
.dropdown-arrow.open {
  transform: rotate(180deg);
}
.dropdown-menu {
  position: absolute;
  top: 36px;
  left: 50%;
  z-index: 1000;
  width: 238px;
  transform: translateX(-50%);
  padding: 6px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: color-mix(in srgb, var(--bg-solid) 98%, transparent);
  box-shadow: 0 14px 34px rgba(0,0,0,.18);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}
.dropdown-item {
  width: 100%;
  min-height: 54px;
  display: grid;
  grid-template-columns: 18px 24px minmax(0, 1fr);
  gap: 8px;
  align-items: center;
  padding: 8px 10px;
  border: 0;
  border-radius: 9px;
  background: transparent;
  color: var(--text-primary);
  text-align: left;
  cursor: pointer;
}
.dropdown-item:hover {
  background: var(--bg-hover);
}
.dropdown-item.active {
  background: color-mix(in srgb, var(--primary) 10%, var(--bg-solid));
}
.check {
  color: var(--primary);
  font-weight: 700;
  text-align: center;
}
.item-icon {
  display: inline-flex;
  color: var(--primary);
}
.item-copy {
  min-width: 0;
  display: grid;
  gap: 2px;
}
.item-copy strong {
  color: var(--text-primary);
  font-size: 13px;
  line-height: 18px;
  font-weight: 600;
}
.item-copy small {
  display: block;
  color: var(--text-tertiary);
  font-size: 11px;
  line-height: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
