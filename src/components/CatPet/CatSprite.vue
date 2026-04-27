<template>
  <div
    class="cat-sprite"
    :class="[
      `cat-${sprite}`,
      { 'cat-animate': animations },
      { 'cat-celebrate': transientAnim === CatTransientAnim.Celebrate },
      { 'cat-bounce': transientAnim === CatTransientAnim.NewTask },
    ]"
    :style="{ width: `${size}px`, height: `${size}px` }"
  >
    <div v-if="spriteMarkup" class="cat-svg" :aria-label="`cat-${sprite}`" v-html="spriteMarkup"></div>
    <div v-else class="cat-fallback" aria-hidden="true">🐱</div>
    <div v-if="sprite === 'sleeping'" class="zzz-layer">💤</div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { CatTransientAnim } from '../../types/pet';

const spriteAssets = import.meta.glob('./cat-sprites/*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

const props = withDefaults(
  defineProps<{
    sprite: string;
    size?: number;
    animations?: boolean;
    transientAnim?: CatTransientAnim;
  }>(),
  {
    size: 64,
    animations: true,
    transientAnim: CatTransientAnim.None,
  },
);

const spriteMarkup = computed(() => {
  const key = `./cat-sprites/${props.sprite}.svg`;
  const raw = spriteAssets[key];
  if (!raw) return '';
  return raw
    .replace('<svg', '<svg class="cat-svg-asset" preserveAspectRatio="xMidYMid meet"')
    .replace(/<text\b/g, '<text xml:space="preserve"');
});
</script>

<style scoped>
.cat-sprite {
  position: relative;
  image-rendering: pixelated;
}

.cat-svg {
  width: 100%;
  height: 100%;
  display: block;
  background: transparent;
}

.cat-svg :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
  overflow: visible;
}

.cat-svg :deep(*) {
  vector-effect: non-scaling-stroke;
}

.cat-fallback {
  width: 100%;
  height: 100%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 34px;
  line-height: 1;
  background: transparent;
}

.cat-animate .cat-svg {
  animation: breathe 3s ease-in-out infinite;
}

.cat-crowned.cat-animate .cat-svg {
  animation: glow 2s ease-in-out infinite;
}

.cat-celebrate .cat-svg {
  animation: celebrate 0.3s ease-in-out 6;
}

.cat-bounce .cat-svg {
  animation: bounce-once 0.5s ease-in-out;
}

.zzz-layer {
  position: absolute;
  top: -7px;
  right: -2px;
  font-size: 11px;
  pointer-events: none;
  animation: float-up 2s ease-in-out infinite;
}

@keyframes breathe {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-2px); }
}

@keyframes glow {
  0%, 100% { filter: brightness(1); }
  50% { filter: brightness(1.2) drop-shadow(0 0 4px color-mix(in srgb, gold 50%, transparent)); }
}

@keyframes celebrate {
  0%, 100% { transform: translateY(0) rotate(0deg); }
  25% { transform: translateY(-6px) rotate(-5deg); }
  75% { transform: translateY(-6px) rotate(5deg); }
}

@keyframes bounce-once {
  0%, 100% { transform: translateY(0); }
  40% { transform: translateY(-8px); }
  60% { transform: translateY(-3px); }
}

@keyframes float-up {
  0%, 100% { opacity: 0.6; transform: translateY(0); }
  50% { opacity: 1; transform: translateY(-4px); }
}
</style>
