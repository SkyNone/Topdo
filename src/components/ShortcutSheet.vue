<template>
  <div class="shortcut-sheet-mask" @click.self="$emit('close')">
    <section class="shortcut-sheet" role="dialog" aria-modal="true" aria-label="快捷键面板">
      <div class="sheet-header">
        <h3>快捷键</h3>
        <button type="button" class="close-btn" title="关闭" @click="$emit('close')">Esc</button>
      </div>

      <div class="sheet-group">
        <p class="group-title">全局</p>
        <div class="row"><span>新建任务</span><kbd>⌘N</kbd></div>
        <div class="row"><span>打开/关闭设置</span><kbd>⌘,</kbd></div>
        <div class="row"><span>主题切换</span><kbd>⌘⇧L</kbd></div>
        <div class="row"><span>快捷键面板</span><kbd>⌘K</kbd></div>
        <div class="row"><span>关闭当前层</span><kbd>Esc</kbd></div>
        <div class="row">
          <span>手动同步</span>
          <div class="keys">
            <kbd>⌘⇧R</kbd>
            <em>仅飞书</em>
          </div>
        </div>
      </div>

      <div class="sheet-group">
        <p class="group-title">窗口</p>
        <div class="row"><span>显示/隐藏 Topdo</span><kbd>{{ toggleWindowShortcut }}</kbd></div>
        <div class="row"><span>面板 / 迷你模式切换</span><kbd>{{ toggleModeShortcut }}</kbd></div>
      </div>

      <div class="sheet-group">
        <p class="group-title">筛选与搜索</p>
        <div class="row"><span>搜索任务</span><kbd>⌘F</kbd></div>
        <div class="row"><span>待办</span><kbd>⌘1</kbd></div>
        <div class="row"><span>进行中</span><kbd>⌘2</kbd></div>
        <div class="row"><span>已完成</span><kbd>⌘3</kbd></div>
        <div class="row"><span>全部</span><kbd>⌘4</kbd></div>
      </div>

      <div class="sheet-group">
        <p class="group-title">任务列表</p>
        <div class="row"><span>上下移动焦点</span><kbd>↑ / ↓</kbd></div>
        <div class="row"><span>展开/收起任务</span><kbd>Enter</kbd></div>
        <div class="row"><span>切换任务状态</span><kbd>⌘Enter</kbd></div>
        <div class="row">
          <span>删除任务</span>
          <div class="keys">
            <kbd>Delete / ⌫</kbd>
            <em>仅本地</em>
          </div>
        </div>
      </div>

      <div class="sheet-group">
        <p class="group-title">快速交互</p>
        <div class="row"><span>双击任务标题</span><kbd>编辑名称</kbd></div>
        <div class="row"><span>回车 / 失焦</span><kbd>保存名称</kbd></div>
        <div class="row"><span>Esc</span><kbd>取消编辑</kbd></div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { onMounted, ref } from 'vue';

defineEmits<{
  (event: 'close'): void;
}>();

interface ShortcutConfigPayload {
  toggle_window: string;
}

interface ModeShortcutConfigPayload {
  toggle_mode: string;
}

const toggleWindowShortcut = ref('Cmd+Shift+T');
const toggleModeShortcut = ref('Alt+T');

async function loadShortcutLabels() {
  try {
    const [windowConfig, modeConfig] = await Promise.all([
      invoke<ShortcutConfigPayload>('get_shortcut_config'),
      invoke<ModeShortcutConfigPayload>('get_mode_shortcut_config')
    ]);
    toggleWindowShortcut.value = windowConfig.toggle_window || 'Cmd+Shift+T';
    toggleModeShortcut.value = modeConfig.toggle_mode || 'Alt+T';
  } catch {
    // keep defaults
  }
}

onMounted(() => {
  void loadShortcutLabels();
});
</script>

<style scoped>
.shortcut-sheet-mask {
  position: absolute;
  inset: 0;
  z-index: 70;
  background: rgba(15, 23, 42, 0.24);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 14px;
}

.shortcut-sheet {
  width: min(360px, 100%);
  max-height: calc(100% - 8px);
  overflow: auto;
  border-radius: 12px;
  border: 0.5px solid var(--border);
  background: var(--bg-solid);
  box-shadow: var(--shadow-md);
  padding: 12px;
}

.sheet-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.sheet-header h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.close-btn {
  border: 0.5px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-secondary);
  border-radius: 6px;
  font-size: 10px;
  height: 20px;
  padding: 0 8px;
}

.sheet-group + .sheet-group {
  margin-top: 10px;
}

.group-title {
  margin: 0 0 5px 0;
  font-size: 11px;
  color: var(--text-tertiary);
}

.row {
  height: 24px;
  border-radius: 6px;
  padding: 0 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: var(--text-primary);
  font-size: 12px;
}

.row:hover {
  background: var(--bg-hover);
}

kbd {
  min-width: 38px;
  border-radius: 5px;
  border: 0.5px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 1;
  padding: 4px 6px;
  text-align: center;
}

.keys {
  display: flex;
  align-items: center;
  gap: 6px;
}

.keys em {
  font-style: normal;
  font-size: 10px;
  color: var(--text-tertiary);
}
</style>
