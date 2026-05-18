<template>
  <section class="habit-view">
    <div class="today-progress">
      <div class="progress-info"><span>今日 {{ habitStore.todayStats.checked }}/{{ habitStore.todayStats.total }} 已打卡</span><strong>{{ habitStore.todayStats.percentage }}%</strong></div>
      <div class="progress-bar"><div :style="{ width: `${habitStore.todayStats.percentage}%` }"></div></div>
      <p v-if="longestStreak > 0">🔥 最长连续：{{ longestStreak }}天</p>
    </div>
    <div class="habit-tabs">
      <button :class="{ active: tab === 'pending' }" @click="tab = 'pending'">待打卡 {{ habitStore.habitsGrouped.pending.length }}</button>
      <button :class="{ active: tab === 'checked' }" @click="tab = 'checked'">已打卡 {{ habitStore.habitsGrouped.checked.length }}</button>
      <button :class="{ active: tab === 'all' }" @click="tab = 'all'">全部 {{ allHabits.length }}</button>
      <button class="add-btn" @click="createOpen = true">+</button>
    </div>
    <div v-if="displayedHabits.length === 0" class="empty">{{ habitStore.activeHabits.length === 0 ? '还没有习惯，创建一个开始打卡' : '这一组暂无习惯' }}</div>
    <div v-else class="habit-list">
      <HabitCard v-for="habit in displayedHabits" :key="habit.id" :habit="habit" @check-in="checkIn(habit.id)" @open="selectedHabit = habit" />
    </div>
    <CreateHabitModal v-if="createOpen" @close="createOpen = false" @save="createHabit" />
    <CreateHabitModal v-if="editingHabit" :habit="editingHabit" @close="editingHabit = null" @save="updateHabit" />
    <HabitDetailModal v-if="selectedHabit" :habit="selectedHabit" :logs="habitStore.logs" @close="selectedHabit = null" @edit="editSelected" @archive="archiveSelected" @delete="deleteSelected" />
    <CheckInSuccess :visible="successVisible" :streak="successStreak" :next-milestone="nextMilestone" />
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import HabitCard from '../components/HabitCard.vue';
import CreateHabitModal from '../components/CreateHabitModal.vue';
import HabitDetailModal from '../components/HabitDetailModal.vue';
import CheckInSuccess from '../components/CheckInSuccess.vue';
import { useHabitStore } from '../stores/habitStore';
import type { Habit, HabitWithStats } from '../types/habit';

const habitStore = useHabitStore();
const tab = ref<'pending' | 'checked' | 'all'>('pending');
const createOpen = ref(false);
const selectedHabit = ref<HabitWithStats | null>(null);
const editingHabit = ref<Habit | null>(null);
const successVisible = ref(false);
const successStreak = ref(0);
let successTimer: ReturnType<typeof setTimeout> | null = null;

const allHabits = computed(() => habitStore.habitsWithStats);
const displayedHabits = computed(() => {
  if (tab.value === 'pending') return habitStore.habitsGrouped.pending;
  if (tab.value === 'checked') return habitStore.habitsGrouped.checked;
  return allHabits.value;
});
const longestStreak = computed(() => Math.max(0, ...allHabits.value.map((h) => h.longestStreak)));
const nextMilestone = computed(() => [7, 21, 30, 66, 100].find((m) => m > successStreak.value) || null);

onMounted(() => void habitStore.fetchHabits());
async function createHabit(input: Partial<Habit>) { await habitStore.createHabit(input); }
async function updateHabit(input: Partial<Habit>) { if (editingHabit.value) await habitStore.updateHabit(editingHabit.value.id, input); editingHabit.value = null; selectedHabit.value = null; }
async function checkIn(id: string) { await habitStore.checkIn(id); const habit = habitStore.habitsWithStats.find((item) => item.id === id); successStreak.value = habit?.currentStreak || 1; successVisible.value = true; if (successTimer) clearTimeout(successTimer); successTimer = setTimeout(() => { successVisible.value = false; }, 1500); }
function editSelected() {
  const habit = selectedHabit.value;
  if (!habit) return;
  selectedHabit.value = null;
  editingHabit.value = { ...habit };
}
async function archiveSelected() {
  const habit = selectedHabit.value;
  if (!habit) return;
  editingHabit.value = null;
  selectedHabit.value = null;
  await habitStore.archiveHabit(habit.id);
}
async function deleteSelected() {
  const habit = selectedHabit.value;
  if (!habit) return;
  editingHabit.value = null;
  selectedHabit.value = null;
  await habitStore.deleteHabit(habit.id);
}
</script>

<style scoped>
.habit-view { position: relative; min-height: 0; flex: 1; display: flex; flex-direction: column; background: transparent; }
.today-progress { margin: 10px 12px 8px; padding: 12px; border-radius: var(--radius-card); background: var(--bg-solid); border: 1px solid var(--border); box-shadow: var(--shadow-sm); }
.progress-info { display: flex; justify-content: space-between; color: var(--text-primary); font-size: 13px; }.progress-info strong { color: #059669; }
.progress-bar { height: 8px; margin-top: 8px; border-radius: 999px; background: var(--bg-secondary); overflow: hidden; }.progress-bar div { height: 100%; border-radius: inherit; background: linear-gradient(90deg, #10b981, #34d399); }
.today-progress p { margin: 7px 0 0; color: var(--text-tertiary); font-size: 11px; }
.habit-tabs { margin: 0 12px 4px; display: flex; gap: 5px; align-items: center; }.habit-tabs button { height: 28px; padding: 0 10px; border: 1px solid var(--border); border-radius: 9px; background: var(--bg-solid); color: var(--text-secondary); cursor: pointer; }.habit-tabs button.active { color: var(--text-primary); background: var(--seg-active-bg); }.habit-tabs .add-btn { margin-left: auto; color: var(--primary); font-size: 18px; }
.habit-list { min-height: 0; flex: 1; overflow-y: auto; padding-bottom: 12px; }.empty { flex: 1; display: grid; place-items: center; color: var(--text-secondary); font-size: 13px; }
</style>
