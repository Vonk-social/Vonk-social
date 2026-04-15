<script lang="ts">
	import { untrack } from 'svelte';
	import { bookmarkPost, unbookmarkPost } from '$lib/api/posts';
	import { toasts } from '$lib/stores/toasts';

	type Props = {
		postUuid: string;
		initial: boolean;
		/** Author-only. Undefined for other users' posts. */
		count?: number;
	};
	let { postUuid, initial, count }: Props = $props();

	let bookmarked = $state(untrack(() => initial));
	let localCount = $state(untrack(() => count));
	let pending = $state(false);

	async function toggle() {
		if (pending) return;
		pending = true;
		const prev = bookmarked;
		bookmarked = !bookmarked;
		if (typeof localCount === 'number') {
			localCount += bookmarked ? 1 : -1;
			if (localCount < 0) localCount = 0;
		}
		try {
			if (bookmarked) await bookmarkPost(postUuid);
			else await unbookmarkPost(postUuid);
		} catch (e) {
			bookmarked = prev;
			if (typeof localCount === 'number') localCount += bookmarked ? 1 : -1;
			toasts.push('error', (e as Error).message);
		} finally {
			pending = false;
		}
	}
</script>

<button
	type="button"
	onclick={toggle}
	aria-pressed={bookmarked}
	aria-label={bookmarked ? 'Verwijder bookmark' : 'Bookmark'}
	class={[
		'inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 text-sm font-semibold transition-colors',
		bookmarked ? 'bg-amber/10 text-amber' : 'text-muted hover:bg-border hover:text-amber'
	].join(' ')}
>
	<svg
		viewBox="0 0 24 24"
		class="h-5 w-5"
		fill={bookmarked ? 'currentColor' : 'none'}
		stroke="currentColor"
		stroke-width="2"
	>
		<path stroke-linecap="round" stroke-linejoin="round" d="M6 4h12v16l-6-4-6 4V4z" />
	</svg>
	{#if typeof localCount === 'number'}
		<span class="tabular-nums">{localCount}</span>
	{:else}
		<span class="sr-only">{bookmarked ? 'Gebookmarked' : 'Bookmark'}</span>
	{/if}
</button>
