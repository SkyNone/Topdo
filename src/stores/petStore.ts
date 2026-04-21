import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { WindowMode, type PetSettings } from '../types/pet';

interface PetSettingsPayload {
  enabled: boolean;
  show_badge: boolean;
  animations: boolean;
  cat_position: { x: number; y: number };
  window_mode: string;
}

export const usePetStore = defineStore('pet', {
  state: (): PetSettings => ({
    enabled: true,
    showBadge: true,
    animations: true,
    catPosition: { x: 0, y: 0 },
    windowMode: WindowMode.Panel,
  }),
  actions: {
    async load() {
      const payload = await invoke<PetSettingsPayload>('get_pet_settings');
      this.enabled = payload.enabled;
      this.showBadge = payload.show_badge;
      this.animations = payload.animations;
      this.catPosition = payload.cat_position || { x: 0, y: 0 };
      this.windowMode = payload.window_mode === WindowMode.Cat ? WindowMode.Cat : WindowMode.Panel;
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
      });
    },
  },
});
