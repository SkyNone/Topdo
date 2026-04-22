<script setup lang="ts">
defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const confettiIcons = ['✨', '🎊', '🎉', '⭐', '💛'];
</script>

<template>
  <Teleport to="body">
    <Transition name="celebration">
      <div v-if="visible" class="celebration-overlay" @click.self="emit('close')">
        <div class="celebration-card">
          <div class="celebration-cat">
            <span class="celebration-crown">👑</span>
            <span class="celebration-face">🐱</span>
          </div>

          <div class="confetti-container">
            <span
              v-for="i in 10"
              :key="i"
              class="confetti"
              :style="{
                '--delay': `${i * 0.12}s`,
                '--x': `${(i % 5) * 20 - 40}px`,
                '--rot': `${i * 36}deg`,
              }"
            >{{ confettiIcons[i % confettiIcons.length] }}</span>
          </div>

          <p class="celebration-title">全部完成！今天的你很棒 ✨</p>

          <button class="celebration-btn" @click="emit('close')">
            继续加油 💪
          </button>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.celebration-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  backdrop-filter: blur(6px);
}

.celebration-card {
  background: var(--bg-primary, #fff);
  border-radius: 20px;
  padding: 32px 28px 24px;
  max-width: 280px;
  width: 80%;
  text-align: center;
  position: relative;
  overflow: hidden;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.25);
}

.celebration-cat {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-bottom: 12px;
}

.celebration-crown {
  font-size: 28px;
  animation: crown-drop 0.5s ease-out;
}

.celebration-face {
  font-size: 44px;
  margin-top: -2px;
}

@keyframes crown-drop {
  0% { transform: translateY(-16px) scale(0); opacity: 0; }
  70% { transform: translateY(2px) scale(1.1); opacity: 1; }
  100% { transform: translateY(0) scale(1); }
}

.confetti-container {
  position: absolute;
  top: 0;
  left: 50%;
  pointer-events: none;
}

.confetti {
  position: absolute;
  font-size: 13px;
  animation: confetti-fall 1.8s ease-out var(--delay) both;
}

@keyframes confetti-fall {
  0% { transform: translate(0, -16px) rotate(0deg) scale(0); opacity: 0; }
  15% { opacity: 1; transform: translate(var(--x), 8px) rotate(var(--rot)) scale(1); }
  100% { transform: translate(var(--x), 100px) rotate(calc(var(--rot) + 180deg)) scale(0.4); opacity: 0; }
}

.celebration-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 20px;
  line-height: 1.4;
}

.celebration-btn {
  padding: 10px 28px;
  border: none;
  background: var(--bg-secondary, #f0f0f0);
  border-radius: 10px;
  font-size: 14px;
  color: var(--text-primary, #333);
  cursor: pointer;
  transition: background 0.15s;
}

.celebration-btn:hover {
  background: var(--bg-hover, #e4e4e4);
}

.celebration-enter-active { transition: opacity 0.25s; }
.celebration-leave-active { transition: opacity 0.2s; }
.celebration-enter-from, .celebration-leave-to { opacity: 0; }
</style>
