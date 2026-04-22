<template>
  <div class="cat-badge">{{ displayCount }}</div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{ count: number }>();
const displayCount = computed(() => (props.count > 99 ? '99+' : String(props.count)));
</script>

<style scoped>
.cat-badge {
  position: absolute;
  top: -4px;
  right: -4px;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 9px;
  background: #ff3b30;
  color: #fff;
  font-size: 11px;
  font-weight: 600;
  font-family: -apple-system, BlinkMacSystemFont, 'SF Pro', sans-serif;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}
</style>
