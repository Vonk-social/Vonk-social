<script lang="ts">
	import { viewSnap } from '$lib/api/snaps';
	import { toasts } from '$lib/stores/toasts';

	type Props = { uuid: string; onClose: () => void };
	let { uuid, onClose }: Props = $props();

	let url = $state<string | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let remainingMs = $state(30_000);
	let timer: ReturnType<typeof setInterval> | null = null;

	async function load() {
		try {
			const res = await viewSnap(uuid);
			url = res.url;
			const expiresAt = new Date(res.expires_at).getTime();
			remainingMs = Math.max(0, expiresAt - Date.now());
			timer = setInterval(() => {
				remainingMs = Math.max(0, expiresAt - Date.now());
				if (remainingMs === 0) onClose();
			}, 100);
		} catch (e) {
			const err = e as Error & { code?: string };
			error =
				err.code === 'snap_already_viewed'
					? 'Deze snap is al bekeken.'
					: err.message || 'Kon snap niet openen';
			toasts.push('error', error);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		load();
		return () => {
			if (timer) clearInterval(timer);
		};
	});

	const seconds = $derived(Math.ceil(remainingMs / 1000));
</script>

<div
	role="dialog"
	aria-modal="true"
	aria-label="Snap bekijken"
	class="fixed inset-0 z-50 flex items-center justify-center bg-black"
>
	<button
		type="button"
		class="absolute right-4 top-4 z-10 rounded-full bg-white/20 px-3 py-1 font-semibold text-white hover:bg-white/30"
		onclick={onClose}
	>✕</button>

	{#if loading}
		<p class="text-white">Ophalen…</p>
	{:else if error}
		<p class="max-w-xs p-6 text-center text-white">{error}</p>
	{:else if url}
		<img
			src={url}
			alt="snap"
			class="max-h-full max-w-full object-contain"
			draggable="false"
		/>
		<div
			class="absolute bottom-6 left-1/2 -translate-x-1/2 rounded-full bg-black/60 px-4 py-2 text-white"
			aria-live="polite"
		>
			⏱ {seconds}s
		</div>
	{/if}
</div>
