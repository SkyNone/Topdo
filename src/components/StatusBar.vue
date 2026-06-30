<template>
  <footer class="status-bar">
    <template v-if="mode === 'local'">
      <div class="status-left">
        <span class="sync-indicator"></span>
        <span>本地模式 | 共 {{ taskCount }} 条任务</span>
        <button type="button" class="shortcut-btn" title="快捷键" @click="$emit('open-shortcuts')">⌘K 快捷键</button>
      </div>
      <span class="status-right">Topdo v2.1.2</span>
    </template>

    <template v-else>
      <div class="status-left">
        <button type="button" class="sync-btn" :disabled="isSyncing" title="立即同步" @click="$emit('sync')">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" :class="isSyncing ? 'spin' : ''" aria-hidden="true">
            <path d="M21 12a9 9 0 1 1-2.64-6.36" />
            <path d="M21 3v6h-6" />
          </svg>
        </button>
        <span class="sync-indicator" :class="offlineMode ? 'warn' : ''"></span>
        <span v-if="offlineMode">离线模式</span>
        <span v-else>上次同步：{{ lastSyncText }}</span>
        <span v-if="failedCount > 0" class="meta-pill danger">失败 {{ failedCount }}</span>
        <button type="button" class="shortcut-btn" title="快捷键" @click="$emit('open-shortcuts')">⌘K 快捷键</button>
      </div>
      <span class="status-right" :title="lastSyncErrorSummary || ''">
        {{ failedCount > 0 && lastSyncErrorSummary ? '同步异常' : 'Topdo v2.1.2' }}
      </span>
    </template>
  </footer>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  mode: 'local' | 'feishu';
  taskCount: number;
  isSyncing: boolean;
  offlineMode: boolean;
  lastSyncTime: number | null;
  pendingCount: number;
  failedCount: number;
  lastSyncErrorSummary: string;
}>();

defineEmits<{
  (event: 'sync'): void;
  (event: 'open-shortcuts'): void;
}>();

const lastSyncText = computed(() => {
  if (!props.lastSyncTime) return '未同步';
  const seconds = Math.max(0, Math.floor((Date.now() - props.lastSyncTime) / 1000));
  if (seconds < 60) return '刚刚同步';
  return `${Math.floor(seconds / 60)}分钟前`;
});
</script>

<style scoped>
.status-bar {
  height: 26px;
  padding: 0 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
  border-top: 0.5px solid var(--border-light);
  background: var(--bg-secondary);
  flex-shrink: 0;
}

.status-left {
  display: flex;
  align-items: center;
  gap: 4px;
}

.status-right {
  font-size: var(--font-size-xs);
  color: var(--text-tertiary);
}

.sync-indicator {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--status-done);
  margin-right: 4px;
}

.sync-indicator.warn {
  background: var(--status-pending);
}

.sync-btn {
  width: 16px;
  height: 16px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  margin-right: 2px;
}

.sync-btn:hover {
  color: var(--primary);
}

.sync-btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.shortcut-btn {
  margin-left: 6px;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  border-radius: 6px;
  height: 18px;
  padding: 0 6px;
  font-size: 10px;
  cursor: pointer;
}

.shortcut-btn:hover {
  background: var(--bg-hover);
  color: var(--text-secondary);
}

.spin {
  animation: spin 0.9s linear infinite;
}

.meta-pill {
  margin-left: 4px;
  padding: 0 5px;
  height: 14px;
  border-radius: 7px;
  font-size: 10px;
  line-height: 14px;
  color: var(--text-secondary);
  background: var(--bg-secondary);
}

.meta-pill.danger {
  color: #ff8a80;
  background: color-mix(in srgb, #ff453a 24%, transparent);
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
}
</style>
