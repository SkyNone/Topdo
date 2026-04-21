import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { CatLevel, WindowMode, type PetSettings } from '../types/pet';

interface PetSettingsPayload {
  enabled: boolean;
  show_badge: boolean;
  animations: boolean;
  cat_position: { x: number; y: number };
  window_mode: string;
  daily_progress_date?: string;
  daily_progress_level?: number;
}

export const usePetStore = defineStore('pet', {
  state: (): PetSettings => ({
    enabled: true,
    showBadge: true,
    animations: true,
    catPosition: { x: 0, y: 0 },
    windowMode: WindowMode.Panel,
    dailyProgressDate: '',
    dailyProgressLevel: CatLevel.Sleeping,
  }),
  actions: {
    async load() {
      const payload = await invoke<PetSettingsPayload>('get_pet_settings');
      this.enabled = payload.enabled;
      this.showBadge = payload.show_badge;
      this.animations = payload.animations;
      this.catPosition = payload.cat_position || { x: 0, y: 0 };
      this.windowMode = payload.window_mode === WindowMode.Cat ? WindowMode.Cat : WindowMode.Panel;
      this.dailyProgressDate = payload.daily_progress_date || '';
      this.dailyProgressLevel =
        payload.daily_progress_level === CatLevel.Crowned
          ? CatLevel.Crowned
          : payload.daily_progress_level === CatLevel.Happy
            ? CatLevel.Happy
            : payload.daily_progress_level === CatLevel.Awake
              ? CatLevel.Awake
              : CatLevel.Sleeping;
    },

    async save(partial?: Partial<PetSettings>) {
      if (partial) {
        Object.assign(this, partial);
      }
      await invoke('save_pet_settings', {
        enabled: this.enabled,
        showBadge: this.showBadge,
        show_badge: this.showBadge,
        animations: this.animations,
        catX: this.catPosition.x,
        cat_x: this.catPosition.x,
        catY: this.catPosition.y,
        cat_y: this.catPosition.y,
        windowMode: this.windowMode,
        window_mode: this.windowMode,
        dailyProgressDate: this.dailyProgressDate,
        daily_progress_date: this.dailyProgressDate,
        dailyProgressLevel: this.dailyProgressLevel,
        daily_progress_level: this.dailyProgressLevel,
      });
    },

    async syncDailyProgress(date: string, level: CatLevel) {
      if (this.dailyProgressDate === date && this.dailyProgressLevel === level) return;
      this.dailyProgressDate = date;
      this.dailyProgressLevel = level;
      await this.save();
    },
  },
});
