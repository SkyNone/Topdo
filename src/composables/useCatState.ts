import { computed, ref, watch } from 'vue';
import { useTaskStore } from '../stores/taskStore';
import { usePetStore } from '../stores/petStore';
import { CatLevel, CatTransientAnim, type CatState } from '../types/pet';

const AWAKE_DONE_THRESHOLD = 1;
const HAPPY_DONE_THRESHOLD = 3;
const NEW_TASK_ANIMATION_MS = 500;
const CELEBRATE_ANIMATION_MS = 2000;

function isDoneStatus(status: string): boolean {
  const value = (status || '').trim();
  return value.includes('已完成');
}

function parseTaskDate(raw: string | undefined): Date | null {
  const value = (raw || '').trim();
  if (!value) return null;

  const asNum = Number(value);
  const date = Number.isFinite(asNum)
    ? new Date(asNum > 1e12 ? asNum : asNum * 1000)
    : new Date(value);

  if (Number.isNaN(date.getTime())) return null;
  return date;
}

function isCompletedToday(completedAt: string | undefined): boolean {
  const completedDate = parseTaskDate(completedAt);
  if (!completedDate) return false;

  const now = new Date();
  const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const tomorrowStart = new Date(todayStart.getTime() + 86400000);
  return completedDate >= todayStart && completedDate < tomorrowStart;
}

function formatLocalDateKey(date = new Date()): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

function mapLevel(todayDoneTasks: number, allCompleted: boolean): CatLevel {
  if (allCompleted) return CatLevel.Crowned;
  if (todayDoneTasks >= HAPPY_DONE_THRESHOLD) return CatLevel.Happy;
  if (todayDoneTasks >= AWAKE_DONE_THRESHOLD) return CatLevel.Awake;
  return CatLevel.Sleeping;
}

function maxLevel(left: CatLevel, right: CatLevel): CatLevel {
  return left >= right ? left : right;
}

function mapSprite(level: CatLevel): string {
  if (level === CatLevel.Crowned) return 'crowned';
  if (level === CatLevel.Happy) return 'happy';
  if (level === CatLevel.Awake) return 'awake';
  return 'sleeping';
}

export function useCatState() {
  const taskStore = useTaskStore();
  const petStore = usePetStore();
  const transientAnim = ref<CatTransientAnim>(CatTransientAnim.None);
  let transientTimer: ReturnType<typeof setTimeout> | null = null;

  function playTransient(anim: CatTransientAnim, durationMs: number) {
    transientAnim.value = anim;
    if (transientTimer) {
      clearTimeout(transientTimer);
    }
    transientTimer = setTimeout(() => {
      transientAnim.value = CatTransientAnim.None;
      transientTimer = null;
    }, durationMs);
  }

  const totalTasks = computed(() => taskStore.tasks.length);
  const currentCompletedTasks = computed(() => taskStore.tasks.filter((task) => isDoneStatus(task.status)).length);
  const doneTasks = computed(() =>
    taskStore.tasks.filter((task) => isDoneStatus(task.status) && isCompletedToday(task.completed_at)).length
  );
  const remainingTasks = computed(() => Math.max(0, totalTasks.value - currentCompletedTasks.value));
  const completionRate = computed(() => {
    if (totalTasks.value === 0) return 0;
    return Math.round((currentCompletedTasks.value / totalTasks.value) * 100);
  });
  const allCompleted = computed(() => totalTasks.value > 0 && currentCompletedTasks.value === totalTasks.value);
  const todayKey = computed(() => formatLocalDateKey());
  const milestoneLevel = computed(() => mapLevel(doneTasks.value, allCompleted.value));
  const persistedLevel = computed(() =>
    petStore.dailyProgressDate === todayKey.value ? petStore.dailyProgressLevel : CatLevel.Sleeping
  );

  const catLevel = computed(() => maxLevel(milestoneLevel.value, persistedLevel.value));
  const sprite = computed(() => mapSprite(catLevel.value));

  watch([todayKey, milestoneLevel], ([nextDate, nextLevel]) => {
    const storedLevel = petStore.dailyProgressDate === nextDate ? petStore.dailyProgressLevel : CatLevel.Sleeping;
    const targetLevel = maxLevel(nextLevel, storedLevel);

    if (petStore.dailyProgressDate !== nextDate || petStore.dailyProgressLevel !== targetLevel) {
      void petStore.syncDailyProgress(nextDate, targetLevel);
    }
  });

  watch([doneTasks, totalTasks, allCompleted, catLevel], ([nextDone, nextTotal, nextAllCompleted, nextLevel], [prevDone, prevTotal, prevAllCompleted, prevLevel]) => {
    const justCompletedAll = nextAllCompleted && !prevAllCompleted;

    if (justCompletedAll) {
      playTransient(CatTransientAnim.Celebrate, CELEBRATE_ANIMATION_MS);
      return;
    }

    if (nextTotal > prevTotal) {
      playTransient(CatTransientAnim.NewTask, NEW_TASK_ANIMATION_MS);
      return;
    }

    if (nextLevel < prevLevel && transientAnim.value !== CatTransientAnim.None) {
      if (transientTimer) {
        clearTimeout(transientTimer);
        transientTimer = null;
      }
      transientAnim.value = CatTransientAnim.None;
    }
  });

  const catState = computed<CatState>(() => ({
    level: catLevel.value,
    transientAnim: transientAnim.value,
    completionRate: completionRate.value,
    totalTasks: totalTasks.value,
    doneTasks: doneTasks.value,
    remainingTasks: remainingTasks.value,
    sprite: sprite.value,
  }));

  return {
    catState,
    catLevel,
    completionRate,
    totalTasks,
    doneTasks,
    remainingTasks,
    sprite,
  };
}
