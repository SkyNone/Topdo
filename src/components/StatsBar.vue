<template>
  <div class="stats-bar">
    <div class="segmented">
      <button class="tab-btn" :class="{ active: currentFilter === 'pending' }" @click="setFilter('pending')">
        待办 <span class="tab-count">{{ pendingCount }}</span>
      </button>
      <button class="tab-btn" :class="{ active: currentFilter === 'in_progress' }" @click="setFilter('in_progress')">
        进行中 <span class="tab-count">{{ inProgressCount }}</span>
      </button>
      <button class="tab-btn" :class="{ active: currentFilter === 'done' }" @click="setFilter('done')">
        已完成 <span class="tab-count">{{ doneCount }}</span>
      </button>
      <button class="tab-btn" :class="{ active: currentFilter === 'all' }" @click="setFilter('all')">
        全部 <span class="tab-count">{{ totalCount }}</span>
      </button>
    </div>

    <button type="button" class="add-btn" title="新建任务" @click="emit('add')">+</button>
    <button type="button" class="stats-summary" @click="statsOpen = !statsOpen">
      今日完成 {{ taskStore.todayCompletedCount }} 项 · 本周 {{ taskStore.weekCompletedCount }} 项 · 🔥连续 {{ taskStore.completionStreak }} 天
    </button>
    <StatsPanel v-if="statsOpen" @open-receipt="openReceipt" />
    <DailyReceipt v-model="receiptOpen" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useTaskStore } from '../stores/taskStore';
import type { TaskFilter } from '../stores/taskStore';
import StatsPanel from './StatsPanel.vue';
import DailyReceipt from './DailyReceipt.vue';

const taskStore = useTaskStore();

const currentFilter = computed(() => taskStore.filter);
const totalCount = computed(() => taskStore.totalTaskCount);
const pendingCount = computed(() => taskStore.pendingTaskCount);
const inProgressCount = computed(() => taskStore.inProgressTaskCount);
const doneCount = computed(() => taskStore.completedTaskCount);
const statsOpen = ref(false);
const receiptOpen = ref(false);

function openReceipt() {
  statsOpen.value = false;
  receiptOpen.value = true;
}

function setFilter(filter: TaskFilter) {
  taskStore.setFilter(filter);
}

const emit = defineEmits<{
  (event: 'add'): void;
}>();
</script>

<style scoped>
.stats-bar {
  position: relative;
  padding: 1px 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 28px;
  align-items: center;
  gap: 8px;
}

.segmented {
  min-width: 0;
  height: 28px;
  padding: 2px;
  display: flex;
  align-items: center;
  gap: 1px;
  border-radius: 10px;
  background: var(--seg-bg);
  border: 0.5px solid color-mix(in srgb, var(--border) 55%, transparent);
}

.tab-btn {
  flex: 1;
  min-width: 0;
  height: 24px;
  padding: 0 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 3px;
  white-space: nowrap;
  font-size: var(--font-size-sm);
  font-family: var(--font-family);
  font-weight: 500;
  color: var(--text-secondary);
  background: transparent;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: color 0.15s ease, background 0.15s ease, box-shadow 0.15s ease;
}

.tab-btn:hover {
  background: color-mix(in srgb, var(--bg-hover) 45%, transparent);
}

.tab-btn.active {
  color: var(--text-primary);
  font-weight: 500;
  background: var(--seg-active-bg);
  box-shadow: var(--seg-active-shadow);
}

.tab-count {
  font-variant-numeric: tabular-nums;
  color: inherit;
}

.add-btn {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border-radius: 8px;
  border: 0.5px solid color-mix(in srgb, var(--border) 65%, transparent);
  background: var(--bg-solid);
  color: var(--primary);
  font-size: 20px;
  line-height: 1;
  font-weight: 400;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background-color 0.15s ease, color 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
}

.add-btn:hover {
  color: color-mix(in srgb, var(--primary) 88%, #000);
  background: color-mix(in srgb, var(--primary) 10%, var(--bg-solid));
  border-color: color-mix(in srgb, var(--primary) 45%, var(--border));
  box-shadow: 0 1px 4px rgba(0, 122, 255, 0.14);
}

.stats-summary {
  grid-column: 1 / -1;
  width: 100%;
  padding: 4px 8px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 11px;
  text-align: left;
  cursor: pointer;
}

.stats-summary:hover {
  background: color-mix(in srgb, var(--bg-hover) 45%, transparent);
  color: var(--text-secondary);
}
</style>
