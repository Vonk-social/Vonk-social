<script lang="ts">
	import { untrack } from 'svelte';
	import { likePost, unlikePost } from '$lib/api/posts';
	import { toasts } from '$lib/stores/toasts';

	type Props = { postUuid: string; initial: boolean };
	let { postUuid, initial }: Props = $props();

	let liked = $state(untrack(() => initial));
	let pending = $state(false);

	async function toggle() {
		if (pending) return;
		pending = true;
		const prev = liked;
		liked = !liked; // optimistic
		try {
			if (liked) await likePost(postUuid);
			else await unlikePost(postUuid);
		} catch (e) {
			liked = prev;
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
	class="inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 text-sm font-semibold transition-colors"
	class:text-terracotta={liked}
	class:text-muted={!liked}
	class:bg-terracotta={liked}
	class:bg-opacity-10={liked}
	class:hover:bg-border={!liked}
>
	<!-- Spark/heart hybrid -->
	<svg aria-hidden="true" viewBox="0 0 24 24" class="h-5 w-5" fill={liked ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2">
		<path stroke-linecap="round" stroke-linejoin="round"
			d="M13 3l-2 6h5l-7 12 2-8H6l7-10z" />
	</svg>
	<span class="sr-only">{liked ? 'Vonk aan' : 'Vonk uit'}</span>
</button>
