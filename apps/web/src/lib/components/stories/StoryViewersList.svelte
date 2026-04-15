<script lang="ts">
	/**
	 * Author-only modal: shows who has viewed a story. Rendered as an overlay
	 * on top of the StoryViewer when the current viewer is the story's author
	 * and taps the eye icon.
	 */
	import { onMount } from 'svelte';
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import { fetchStoryViewers, type StoryViewer } from '$lib/api/posts';

	type Props = { storyUuid: string; onClose: () => void };
	let { storyUuid, onClose }: Props = $props();

	let viewers = $state<StoryViewer[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			viewers = await fetchStoryViewers(storyUuid);
		} catch (e) {
			error = (e as Error).message;
		} finally {
			loading = false;
		}
	});

	function onKey(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}

	function relTime(iso: string): string {
		const s = Math.max(0, Math.round((Date.now() - new Date(iso).getTime()) / 1000));
		if (s < 60) return 'zojuist';
		if (s < 3600) return `${Math.round(s / 60)} min`;
		if (s < 86400) return `${Math.round(s / 3600)} u`;
		return `${Math.round(s / 86400)} d`;
	}
</script>

<svelte:window on:keydown={onKey} />

<div
	role="dialog"
	aria-modal="true"
	aria-label="Kijkers van deze story"
	class="fixed inset-0 z-[60] flex items-end justify-center bg-black/60 p-4 sm:items-center"
>
	<button
		type="button"
		class="absolute inset-0 cursor-default"
		aria-label="Sluit"
		onclick={onClose}
	></button>

	<div class="relative w-full max-w-md rounded-[var(--radius-card)] bg-surface shadow-xl">
		<header class="flex items-center justify-between border-b border-border p-4">
			<h2 class="font-display text-lg font-bold text-ink">
				👁 Gezien door {viewers.length}
			</h2>
			<button
				type="button"
				onclick={onClose}
				class="rounded-full p-1 text-muted hover:bg-border"
				aria-label="Sluit"
			>✕</button>
		</header>

		<div class="max-h-96 overflow-y-auto">
			{#if loading}
				<p class="p-6 text-center text-sm text-muted">Laden…</p>
			{:else if error}
				<p class="p-6 text-center text-sm text-terracotta">{error}</p>
			{:else if viewers.length === 0}
				<p class="p-6 text-center text-sm text-muted">Nog niemand heeft deze story gezien.</p>
			{:else}
				<ul class="divide-y divide-border">
					{#each viewers as v (v.uuid)}
						<li>
							<a
								href="/u/{v.username}"
								class="flex items-center gap-3 p-3 transition-colors hover:bg-border/30"
							>
								<Avatar url={v.avatar_url} name={v.display_name} size={40} />
								<div class="min-w-0 flex-1">
									<p class="truncate font-semibold text-ink">{v.display_name}</p>
									<p class="truncate text-sm text-muted">@{v.username}</p>
								</div>
								<span class="text-xs text-muted">{relTime(v.viewed_at)}</span>
							</a>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
</div>
