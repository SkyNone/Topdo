import { computed, ref } from 'vue';

export type ThemePreference = 'system' | 'light' | 'dark';
export type ResolvedTheme = 'light' | 'dark';

const STORAGE_KEY = 'topdo_theme_preference_v1';

const themePreference = ref<ThemePreference>('system');
const systemTheme = ref<ResolvedTheme>('light');
let initialized = false;
let mediaQuery: MediaQueryList | null = null;
let mediaHandler: ((event: MediaQueryListEvent) => void) | null = null;

const resolvedTheme = computed<ResolvedTheme>(() =>
  themePreference.value === 'system' ? systemTheme.value : themePreference.value
);

function safeReadPreference(): ThemePreference {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === 'light' || raw === 'dark' || raw === 'system') {
      return raw;
    }
  } catch {
    // ignore
  }
  return 'system';
}

function detectSystemTheme(): ResolvedTheme {
  if (typeof window === 'undefined' || !window.matchMedia) return 'light';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function applyThemeAttribute() {
  if (typeof document === 'undefined') return;
  const html = document.documentElement;
  html.setAttribute('data-theme', resolvedTheme.value);
}

function persistPreference() {
  try {
    localStorage.setItem(STORAGE_KEY, themePreference.value);
  } catch {
    // ignore
  }
}

export function initializeTheme() {
  if (initialized) return;
  initialized = true;

  themePreference.value = safeReadPreference();
  systemTheme.value = detectSystemTheme();
  applyThemeAttribute();

  if (typeof window !== 'undefined' && window.matchMedia) {
    mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaHandler = (event: MediaQueryListEvent) => {
      systemTheme.value = event.matches ? 'dark' : 'light';
      if (themePreference.value === 'system') {
        applyThemeAttribute();
      }
    };
    mediaQuery.addEventListener('change', mediaHandler);
  }
}

export function setThemePreference(next: ThemePreference) {
  themePreference.value = next;
  persistPreference();
  applyThemeAttribute();
}

export function toggleThemeQuickly() {
  if (themePreference.value === 'system') {
    setThemePreference(resolvedTheme.value === 'dark' ? 'light' : 'dark');
    return;
  }
  setThemePreference(themePreference.value === 'dark' ? 'light' : 'dark');
}

export function useThemeState() {
  return {
    themePreference,
    resolvedTheme
  };
}

