<template>
  <section class="welcome-page">
    <div class="welcome-header">
      <div class="welcome-icon">✅</div>
      <h1 class="welcome-title">Topdo</h1>
      <p class="welcome-subtitle">你的桌面任务悬浮窗</p>
    </div>

    <div class="mode-list">
      <div class="mode-option" :class="{ selected: selectedMode === 'local' }" @click="selectedMode = 'local'">
        <div class="mode-indicator" />
        <div class="mode-icon">💻</div>
        <div class="mode-info">
          <div class="mode-name">本地模式</div>
          <div class="mode-desc">开箱即用，数据存本地</div>
        </div>
      </div>

      <div class="mode-option" :class="{ selected: selectedMode === 'feishu' }" @click="selectedMode = 'feishu'">
        <div class="mode-indicator" />
        <div class="mode-icon">☁️</div>
        <div class="mode-info">
          <div class="mode-name">飞书同步模式</div>
          <div class="mode-desc">连接飞书多维表格，多端同步</div>
        </div>
      </div>
    </div>

    <div class="welcome-actions">
      <button type="button" class="btn-start" @click="handleStart">
        {{ selectedMode === 'local' ? '开始使用' : '配置连接' }}
      </button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { ref } from 'vue';

const selectedMode = ref<'local' | 'feishu'>('local');

const emit = defineEmits<{
  (event: 'select-local'): void;
  (event: 'select-feishu'): void;
}>();

function handleStart() {
  if (selectedMode.value === 'local') {
    emit('select-local');
  } else {
    emit('select-feishu');
  }
}
</script>

<style scoped>
.welcome-page {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 32px 20px 24px;
  height: 100%;
  box-sizing: border-box;
}

.welcome-header {
  text-align: center;
  margin-bottom: 24px;
}

.welcome-icon {
  font-size: 32px;
  margin-bottom: 8px;
}

.welcome-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary, #1d1d1f);
  margin: 0 0 4px 0;
  letter-spacing: -0.015em;
}

.welcome-subtitle {
  font-size: 13px;
  color: var(--text-secondary, #86868b);
  margin: 0;
  font-weight: 400;
}

.mode-list {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 24px;
}

.mode-option {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 10px;
  background: transparent;
  cursor: pointer;
  transition: all 0.15s ease;
  position: relative;
  border: 1px solid transparent;
}

.mode-option:hover {
  background: var(--bg-hover, rgba(0, 0, 0, 0.03));
}

.mode-option.selected {
  background: rgba(0, 122, 255, 0.06);
  border-color: rgba(0, 122, 255, 0.15);
}

.mode-indicator {
  width: 3px;
  height: 28px;
  border-radius: 1.5px;
  background: transparent;
  flex-shrink: 0;
  transition: background 0.15s ease;
}

.mode-option.selected .mode-indicator {
  background: var(--primary, #007aff);
}

.mode-icon {
  font-size: 24px;
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.mode-info {
  flex: 1;
  min-width: 0;
}

.mode-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary, #1d1d1f);
  margin-bottom: 2px;
}

.mode-desc {
  font-size: 11px;
  color: var(--text-secondary, #86868b);
}

.welcome-actions {
  width: 100%;
  display: flex;
  justify-content: center;
}

.btn-start {
  padding: 8px 32px;
  font-size: 13px;
  font-weight: 600;
  color: white;
  background: var(--primary, #007aff);
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  font-family: var(--font-family, -apple-system, BlinkMacSystemFont, sans-serif);
}

.btn-start:hover {
  background: var(--primary-hover, #0066d6);
}

.btn-start:active {
  transform: scale(0.98);
}

@media (prefers-color-scheme: dark) {
  .mode-option.selected {
    background: rgba(10, 132, 255, 0.1);
    border-color: rgba(10, 132, 255, 0.2);
  }
}

:root.dark .mode-option.selected {
  background: rgba(10, 132, 255, 0.1);
  border-color: rgba(10, 132, 255, 0.2);
}

:root.dark .mode-option:hover {
  background: rgba(255, 255, 255, 0.04);
}
</style>
