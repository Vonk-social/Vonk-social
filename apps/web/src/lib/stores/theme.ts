import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type Theme = 'light' | 'dark';
const STORAGE_KEY = 'vonk-theme';

function initialTheme(): Theme {
	if (!browser) return 'light';
	const stored = localStorage.getItem(STORAGE_KEY);
	if (stored === 'light' || stored === 'dark') return stored;
	// No explicit choice yet: respect OS preference.
	return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function applyTheme(t: Theme) {
	if (!browser) return;
	document.documentElement.classList.toggle('dark', t === 'dark');
	document.documentElement.style.colorScheme = t;
	// Update PWA theme-color so the browser chrome (address bar on mobile)
	// matches the current palette.
	const meta = document.querySelector('meta[name="theme-color"]');
	if (meta) meta.setAttribute('content', t === 'dark' ? '#1A120B' : '#FF6C2F');
}

function createStore() {
	const t = initialTheme();
	const { subscribe, set, update } = writable<Theme>(t);
	if (browser) {
		applyTheme(t);
	}
	return {
		subscribe,
		set(next: Theme) {
			if (browser) {
				localStorage.setItem(STORAGE_KEY, next);
				applyTheme(next);
			}
			set(next);
		},
		toggle() {
			update((cur) => {
				const next: Theme = cur === 'dark' ? 'light' : 'dark';
				if (browser) {
					localStorage.setItem(STORAGE_KEY, next);
					applyTheme(next);
				}
				return next;
			});
		}
	};
}

export const theme = createStore();
