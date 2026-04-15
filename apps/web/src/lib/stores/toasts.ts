import { writable } from 'svelte/store';

export type ToastKind = 'info' | 'success' | 'error';
export type Toast = { id: number; kind: ToastKind; message: string };

let nextId = 1;

function createStore() {
	const { subscribe, update } = writable<Toast[]>([]);
	return {
		subscribe,
		push: (kind: ToastKind, message: string, ttlMs = 4000) => {
			const id = nextId++;
			update((t) => [...t, { id, kind, message }]);
			if (ttlMs > 0) {
				setTimeout(() => update((t) => t.filter((x) => x.id !== id)), ttlMs);
			}
		},
		dismiss: (id: number) => update((t) => t.filter((x) => x.id !== id))
	};
}

export const toasts = createStore();
