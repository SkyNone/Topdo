<template>
  <div class="cat-pet-container">
    <CatSprite :sprite="catState.sprite" :size="size" :animations="animations" :transient-anim="catState.transientAnim" />
    <CatBadge v-if="showBadge && catState.remainingTasks > 0" :count="catState.remainingTasks" />
  </div>
</template>

<script setup lang="ts">
import CatBadge from './CatBadge.vue';
import CatSprite from './CatSprite.vue';
import { useCatState } from '../../composables/useCatState';

withDefaults(
  defineProps<{
    showBadge?: boolean;
    animations?: boolean;
    size?: number;
  }>(),
  {
    showBadge: true,
    animations: true,
    size: 64,
  },
);

const { catState } = useCatState();
</script>

<style scoped>
.cat-pet-container {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  user-select: none;
  -webkit-user-select: none;
  pointer-events: none;
}
</style>
