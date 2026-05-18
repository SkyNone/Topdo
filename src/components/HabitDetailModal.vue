<template>
  <div class="modal-mask" @click.self="emit('close')">
    <div class="detail-card">
      <button class="close" @click="emit('close')">×</button>
      <div class="hero"><div class="emoji" :style="{ background: `${habit.color}22` }">{{ habit.emoji }}</div><h3>{{ habit.name }}</h3><p>{{ frequencyLabel }}</p></div>
      <div class="stat-row"><div><strong>{{ habit.currentStreak }}</strong><span>当前连续</span></div><div><strong>{{ habit.longestStreak }}</strong><span>最长连续</span></div><div><strong>{{ habit.monthCompletionRate }}%</strong><span>本月完成</span></div></div>
      <div class="calendar"><span v-for="day in monthDays" :key="day.key" :class="{ done: day.done, today: day.today }">{{ day.day }}</span></div>
      <div class="actions">
        <button type="button" @click.stop="emit('edit')">编辑</button>
        <button type="button" @click.stop="emit('archive')">归档</button>
        <button type="button" class="danger" @click.stop="emit('delete')">删除</button>
      </div>
    </div>
  </div>
</template>
<script setup lang="ts">
import { computed } from 'vue';
import type { HabitLog, HabitWithStats } from '../types/habit';
const props = defineProps<{ habit: HabitWithStats; logs: HabitLog[] }>();
const emit = defineEmits<{ (event: 'close'): void; (event: 'edit'): void; (event: 'archive'): void; (event: 'delete'): void }>();
const frequencyLabel = computed(() => props.habit.frequency_type === 'daily' ? '每天' : props.habit.frequency_type === 'weekdays' ? '工作日' : '自定义');
function key(date: Date) { return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`; }
const monthDays = computed(() => {
  const now = new Date();
  const total = new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate();
  const done = new Set(props.logs.filter((l) => l.habit_id === props.habit.id).map((l) => l.checked_at));
  return Array.from({ length: total }, (_, i) => { const date = new Date(now.getFullYear(), now.getMonth(), i + 1); return { key: key(date), day: i + 1, done: done.has(key(date)), today: key(date) === key(now) }; });
});
</script>
<style scoped>
.modal-mask { position: absolute; inset: 0; z-index: 90; display: grid; place-items: center; background: rgba(0,0,0,.22); }
.detail-card { position: relative; width: min(304px, calc(100% - 24px)); max-height: calc(100% - 36px); overflow: auto; padding: 16px; border-radius: 18px; background: var(--bg-solid); border: 1px solid var(--border); box-shadow: 0 18px 50px rgba(0,0,0,.2); }
.close { position: absolute; right: 10px; top: 8px; border: 0; background: transparent; color: var(--text-tertiary); font-size: 22px; cursor: pointer; }
.hero { text-align: center; }.emoji { width: 58px; height: 58px; margin: 0 auto 8px; display: grid; place-items: center; border-radius: 18px; font-size: 30px; } h3 { margin: 0; color: var(--text-primary); } p { margin: 4px 0 0; color: var(--text-tertiary); font-size: 12px; }
.stat-row { margin-top: 14px; display: grid; grid-template-columns: repeat(3,1fr); gap: 8px; }.stat-row div { padding: 8px; border-radius: 12px; background: var(--bg-secondary); text-align: center; }.stat-row strong { display: block; color: var(--text-primary); font-size: 18px; }.stat-row span { font-size: 10px; color: var(--text-tertiary); }
.calendar { margin-top: 14px; display: grid; grid-template-columns: repeat(7,1fr); gap: 5px; }.calendar span { height: 24px; display: grid; place-items: center; border-radius: 7px; background: var(--bg-secondary); color: var(--text-tertiary); font-size: 10px; }.calendar span.done { background: #10b981; color: white; }.calendar span.today { outline: 1px solid var(--primary); }
.actions { margin-top: 14px; display: flex; justify-content: flex-end; gap: 8px; }.actions button { border: 1px solid var(--border); border-radius: 8px; background: var(--bg-secondary); color: var(--text-secondary); padding: 5px 9px; cursor: pointer; }.actions .danger { color: #dc2626; }
</style>
