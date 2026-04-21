export enum CatLevel {
  Sleeping = 1,
  Awake = 2,
  Happy = 3,
  Crowned = 4,
}

export enum CatTransientAnim {
  None = 'none',
  NewTask = 'new_task',
  Celebrate = 'celebrate',
}

export enum WindowMode {
  Panel = 'panel',
  Cat = 'cat',
  Island = 'island',
}

export interface CatState {
  level: CatLevel;
  transientAnim: CatTransientAnim;
  completionRate: number;
  totalTasks: number;
  doneTasks: number;
  remainingTasks: number;
  sprite: string;
}

export interface PetSettings {
  enabled: boolean;
  showBadge: boolean;
  animations: boolean;
  catPosition: { x: number; y: number };
  windowMode: WindowMode.Panel | WindowMode.Cat;
  dailyProgressDate: string;
  dailyProgressLevel: CatLevel;
}
