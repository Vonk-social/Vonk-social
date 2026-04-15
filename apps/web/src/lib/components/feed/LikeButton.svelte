<script lang="ts">
	import { untrack } from 'svelte';
	import { likePost, unlikePost } from '$lib/api/posts';
	import { toasts } from '$lib/stores/toasts';

	type Props = {
		postUuid: string;
		initial: boolean;
		/** Author-only. Undefined for other users' posts. When defined, the
		 *  button shows the count next to the icon. */
		count?: number;
	};
	let { postUuid, initial, count }: Props = $props();

	let liked = $state(untrack(() => initial));
	let localCount = $state(untrack(() => count));
	let pending = $state(false);

	async function toggle() {
		if (pending) return;
		pending = true;
		const prev = liked;
		liked = !liked;
		// Optimistically bump the author-visible count too.
		if (typeof localCount === 'number') {
			localCount += liked ? 1 : -1;
			if (localCount < 0) localCount = 0;
		}
		try {
			if (liked) await likePost(postUuid);
			else await unlikePost(postUuid);
		} catch (e) {
			liked = prev;
			if (typeof localCount === 'number') localCount += liked ? 1 : -1;
			toasts.push('error', (e as Error).message);
		} finally {
			pending = false;
		}
	}
</script>

<button
	type="button"
	onclick={toggle}
	aria-pressed={liked}
	aria-label={liked ? 'Ont-vonk' : 'Vonk'}
	class={[
		'group inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 text-sm font-semibold transition-colors',
		liked
			? 'bg-terracotta/10 text-terracotta'
			: 'text-muted hover:bg-border hover:text-terracotta'
	].join(' ')}
>
	<svg
		aria-hidden="true"
		viewBox="0 0 24 24"
		class="h-5 w-5 transition-transform"
		class:scale-110={liked}
		fill={liked ? 'currentColor' : 'none'}
		stroke="currentColor"
		stroke-width="2"
	>
		<!-- Spark: a stylised lightning / vonk. -->
		<path stroke-linecap="round" stroke-linejoin="round" d="M13 3l-2 6h5l-7 12 2-8H6l7-10z" />
	</svg>
	{#if typeof localCount === 'number'}
		<span class="tabular-nums" aria-label="{localCount} Vonken">{localCount}</span>
	{:else}
		<span class="sr-only">{liked ? 'Vonk aan' : 'Vonk uit'}</span>
	{/if}
</button>
