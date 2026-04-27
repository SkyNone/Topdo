<template>
  <header
    data-tauri-drag-region
    class="title-bar"
    :class="{ inactive: !windowFocused }"
    @dblclick="onDoubleClick"
  >
    <div
      class="traffic-lights"
      data-no-drag
      @mouseenter="trafficHover = true"
      @mouseleave="trafficHover = false"
      @dblclick.stop
    >
      <button class="traffic-btn close" type="button" title="关闭到托盘" @click.stop="$emit('close-to-tray')" @dblclick.stop>
        <svg v-if="trafficHover" viewBox="0 0 12 12" width="10" height="10" aria-hidden="true">
          <line x1="3" y1="3" x2="9" y2="9" stroke="rgba(0,0,0,0.5)" stroke-width="1.2" stroke-linecap="round" />
          <line x1="9" y1="3" x2="3" y2="9" stroke="rgba(0,0,0,0.5)" stroke-width="1.2" stroke-linecap="round" />
        </svg>
      </button>

      <button class="traffic-btn minimize" type="button" title="迷你模式" @click.stop="$emit('mini')" @dblclick.stop>
        <svg v-if="trafficHover" viewBox="0 0 12 12" width="10" height="10" aria-hidden="true">
          <line x1="2.5" y1="6" x2="9.5" y2="6" stroke="rgba(0,0,0,0.5)" stroke-width="1.2" stroke-linecap="round" />
        </svg>
      </button>

      <button class="traffic-btn maximize" type="button" title="窗口控制" @click.stop="handleGreen" @dblclick.stop>
        <svg v-if="trafficHover" viewBox="0 0 12 12" width="10" height="10" aria-hidden="true">
          <circle cx="6" cy="6" r="3.5" stroke="rgba(0,0,0,0.5)" stroke-width="1.1" fill="none" />
        </svg>
      </button>
    </div>

    <div data-tauri-drag-region class="title-text">Topdo</div>

    <div class="titlebar-actions" data-no-drag @dblclick.stop>
      <button
        class="titlebar-action"
        type="button"
        :title="isDarkMode ? '切换到浅色模式' : '切换到深色模式'"
        @click.stop="toggleTheme"
        @dblclick.stop
      >
        <svg v-if="!isDarkMode" width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M13.5 9.2A5.5 5.5 0 0 1 6.8 2.5a6 6 0 1 0 6.7 6.7Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <circle cx="8" cy="8" r="3" stroke="currentColor" stroke-width="1.5"/>
          <line x1="8" y1="1.5" x2="8" y2="3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="8" y1="13" x2="8" y2="14.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="1.5" y1="8" x2="3" y2="8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="13" y1="8" x2="14.5" y2="8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="3.4" y1="3.4" x2="4.46" y2="4.46" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="11.54" y1="11.54" x2="12.6" y2="12.6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="3.4" y1="12.6" x2="4.46" y2="11.54" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="11.54" y1="4.46" x2="12.6" y2="3.4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>

      <button
        class="titlebar-action"
        :class="{ active: isPinned }"
        type="button"
        :title="isPinned ? '取消置顶' : '窗口置顶'"
        @click.stop="togglePin"
        @dblclick.stop
      >
        <svg v-if="!isPinned" width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M9.85 1.85L14.15 6.15" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <path d="M5.75 6.6L9.4 2.95L13.05 6.6L9.4 10.25L5.75 6.6Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
          <path d="M6.6 9.4L2 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <path d="M3.5 9.25L6.75 12.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M6 2.5L10 2.5L10.75 7H5.25L6 2.5Z" fill="currentColor" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
          <line x1="8" y1="7" x2="8" y2="14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <line x1="4.5" y1="7" x2="11.5" y2="7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>

      <button class="titlebar-action" type="button" title="设置" @click.stop="openSettings" @dblclick.stop>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M6.86 2h2.28l.32 1.91a4.5 4.5 0 0 1 1.32.77L12.6 4l1.14 1.97-1.5 1.22c.06.27.1.54.1.81s-.04.54-.1.81l1.5 1.22L12.6 12l-1.82-.68a4.5 4.5 0 0 1-1.32.77L9.14 14H6.86l-.32-1.91a4.5 4.5 0 0 1-1.32-.77L3.4 12 2.26 10.03l1.5-1.22A4.5 4.5 0 0 1 3.66 8c0-.27.04-.54.1-.81l-1.5-1.22L3.4 4l1.82.68a4.5 4.5 0 0 1 1.32-.77L6.86 2Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
          <circle cx="8" cy="8" r="1.75" stroke="currentColor" stroke-width="1.5"/>
        </svg>
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

const props = defineProps<{
  alwaysOnTop: boolean;
  resolvedTheme: 'light' | 'dark';
}>();

const emit = defineEmits<{
  (event: 'settings'): void;
  (event: 'toggle-pin'): void;
  (event: 'toggle-theme'): void;
  (event: 'mini'): void;
  (event: 'close-to-tray'): void;
}>();

const trafficHover = ref(false);
const windowFocused = ref(true);

function onFocus() {
  windowFocused.value = true;
}

function onBlur() {
  windowFocused.value = false;
}

function onDoubleClick() {
  emit('mini');
}

function handleGreen() {
  // 悬浮窗不启用全屏行为，保留标准红绿灯布局语义
}

const isDarkMode = computed(() => props.resolvedTheme === 'dark');
const isPinned = computed(() => props.alwaysOnTop);

function toggleTheme() {
  emit('toggle-theme');
}

function togglePin() {
  emit('toggle-pin');
}

function openSettings() {
  emit('settings');
}

onMounted(() => {
  window.addEventListener('focus', onFocus);
  window.addEventListener('blur', onBlur);
});

onBeforeUnmount(() => {
  window.removeEventListener('focus', onFocus);
  window.removeEventListener('blur', onBlur);
});
</script>

<style scoped>
.title-bar {
  height: 38px;
  padding: 0 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: transparent;
  border-bottom: 0.5px solid var(--border-light);
  position: relative;
}

.traffic-lights {
  display: flex;
  align-items: center;
  gap: 8px;
  -webkit-app-region: no-drag;
  padding: 0 4px;
}

.traffic-btn {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  transition: opacity 0.15s ease;
}

.traffic-btn.close {
  background: var(--traffic-close);
}

.traffic-btn.minimize {
  background: var(--traffic-minimize);
}

.traffic-btn.maximize {
  background: var(--traffic-maximize);
}

.title-bar.inactive .traffic-btn {
  background: var(--traffic-inactive);
}

.traffic-btn svg {
  opacity: 0;
  transition: opacity 0.1s ease;
}

.traffic-lights:hover .traffic-btn svg {
  opacity: 1;
}

.title-text {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
  font-size: var(--font-size-base);
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: -0.01em;
  pointer-events: none;
  user-select: none;
}

.titlebar-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  -webkit-app-region: no-drag;
  margin-left: auto;
}

.titlebar-action {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--text-secondary, #86868b);
  cursor: pointer;
  transition: all 0.15s ease;
  padding: 0;
  -webkit-app-region: no-drag;
}

.titlebar-action:hover {
  background: var(--bg-hover, rgba(0, 0, 0, 0.04));
  color: var(--text-primary, #1d1d1f);
}

.titlebar-action.active {
  color: var(--primary, #007aff);
}

.titlebar-action.active:hover {
  background: rgba(0, 122, 255, 0.08);
}

.titlebar-action svg {
  flex-shrink: 0;
}
</style>
