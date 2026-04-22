<script setup lang="ts">
import { open } from '@tauri-apps/plugin-shell';

async function openGitHub() {
  await open('https://github.com/SkyNone/Topdo');
}

async function openFeedback() {
  await open('https://github.com/SkyNone/Topdo/issues');
}
</script>

<template>
  <div class="about-section">
    <div class="about-divider"></div>

    <p class="about-title">关于</p>
    <p class="about-version">Topdo v2.0</p>
    <p class="about-slogan">一个运营同学的效率自救产物</p>
    <p class="about-desc">
      这个小工具诞生于无数个被消息轰炸的下午。<br />
      如果它也帮到了你，我就很开心了。
    </p>

    <div class="about-links">
      <button class="about-link" @click="openGitHub">
        ⭐ GitHub
      </button>
      <button class="about-link" @click="openFeedback">
        💬 反馈建议
      </button>
    </div>
  </div>
</template>

<style scoped>
.about-section {
  padding: 0 0 24px;
}

.about-divider {
  height: 1px;
  background: var(--border-color, rgba(0, 0, 0, 0.08));
  margin: 20px 0 16px;
}

.about-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 8px;
}

.about-version {
  font-size: 12px;
  color: var(--text-secondary, #999);
  margin: 0 0 2px;
}

.about-slogan {
  font-size: 12px;
  color: var(--text-secondary, #999);
  font-style: italic;
  margin: 0 0 12px;
}

.about-desc {
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary, #888);
  margin: 0 0 16px;
}

.about-links {
  display: flex;
  gap: 12px;
}

.about-link {
  padding: 6px 14px;
  border: 1px solid var(--border-color, rgba(0, 0, 0, 0.1));
  background: transparent;
  border-radius: 8px;
  font-size: 12px;
  color: var(--text-secondary, #666);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.about-link:hover {
  background: var(--bg-hover, rgba(0, 0, 0, 0.04));
  color: var(--text-primary);
}
</style>
