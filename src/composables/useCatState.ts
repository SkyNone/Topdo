import { computed, ref, watch } from 'vue';
import { useTaskStore } from '../stores/taskStore';
import { CatLevel, CatTransientAnim, type CatState } from '../types/pet';

function isDoneStatus(status: string): boolean {
  const value = (status || '').trim();
  return value.includes('已完成');
}

function mapLevel(rate: number): CatLevel {
  if (rate >= 100) return CatLevel.Crowned;
  if (rate >= 70) return CatLevel.Happy;
  if (rate >= 30) return CatLevel.Awake;
  return CatLevel.Sleeping;
}

function mapSprite(level: CatLevel): string {
  if (level === CatLevel.Crowned) return 'crowned';
  if (level === CatLevel.Happy) return 'happy';
  if (level === CatLevel.Awake) return 'awake';
  return 'sleeping';
}

export function useCatState() {
  const taskStore = useTaskStore();
  const transientAnim = ref<CatTransientAnim>(CatTransientAnim.None);

  const totalTasks = computed(() => taskStore.tasks.length);
  const doneTasks = computed(() => taskStore.tasks.filter((task) => isDoneStatus(task.status)).length);
  const remainingTasks = computed(() => Math.max(0, totalTasks.value - doneTasks.value));
  const completionRate = computed(() => {
    if (totalTasks.value === 0) return 0;
    return Math.round((doneTasks.value / totalTasks.value) * 100);
  });

  const catLevel = computed(() => mapLevel(completionRate.value));
  const sprite = computed(() => mapSprite(catLevel.value));

  watch(completionRate, (next, prev) => {
    if (next >= 100 && prev < 100) {
      transientAnim.value = CatTransientAnim.Celebrate;
      setTimeout(() => {
        transientAnim.value = CatTransientAnim.None;
      }, 2000);
    }
  });

  watch(totalTasks, (next, prev) => {
    if (next > prev) {
      transientAnim.value = CatTransientAnim.NewTask;
      setTimeout(() => {
        transientAnim.value = CatTransientAnim.None;
      }, 500);
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
