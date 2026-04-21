<template>
  <button
    type="button"
    class="panel-cat-corner"
    title="切换到猫咪态（⌥T）"
    @click="$emit('collapse')"
  >
    <CatPet :show-badge="showBadge" :animations="animations" :size="28" />
  </button>
</template>

<script setup lang="ts">
import CatPet from './CatPet/CatPet.vue';

defineProps<{
  showBadge: boolean;
  animations: boolean;
}>();

defineEmits<{
  (event: 'collapse'): void;
}>();
</script>

<style scoped>
.panel-cat-corner {
  width: 44px;
  height: 44px;
  padding: 8px;
  box-sizing: border-box;
  border-radius: 12px;
  border: none;
  background: transparent;
  box-shadow: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s ease;
  -webkit-app-region: no-drag;
}

.panel-cat-corner:hover {
  transform: translateY(-1px);
  filter: brightness(1.04);
}
</style>
