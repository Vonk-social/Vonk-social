<script lang="ts">
	import { untrack } from 'svelte';
	import RepostDialog from './RepostDialog.svelte';
	import { unrepostPost, type PublicPost } from '$lib/api/posts';
	import { toasts } from '$lib/stores/toasts';

	type Props = {
		postUuid: string;
		authorUsername: string;
		initial: boolean;
		count?: number;
		onReposted?: (p: PublicPost) => void;
	};
	let { postUuid, authorUsername, initial, count, onReposted }: Props = $props();

	let reposted = $state(untrack(() => initial));
	let localCount = $state(untrack(() => count));
	let dialogOpen = $state(false);
	let pending = $state(false);

	async function click() {
		if (pending) return;
		if (reposted) {
			// Second click = undo. No dialog to stay consistent with Twitter UX.
			pending = true;
			const prev = reposted;
			reposted = false;
			if (typeof localCount === 'number') localCount = Math.max(0, localCount - 1);
			try {
				await unrepostPost(postUuid);
			} catch (e) {
				reposted = prev;
				if (typeof localCount === 'number') localCount += 1;
				toasts.push('error', (e as Error).message);
			} finally {
				pending = false;
			}
		} else {
			dialogOpen = true;
		}
	}

	function onDialogReposted(p: PublicPost) {
		reposted = true;
		if (typeof localCount === 'number') localCount += 1;
		onReposted?.(p);
	}
</script>

<button
	type="button"
	onclick={click}
	aria-pressed={reposted}
	aria-label={reposted ? 'Repost ongedaan maken' : 'Repost'}
	class={[
		'inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 text-sm font-semibold transition-colors',
		reposted ? 'bg-sage/20 text-sage' : 'text-muted hover:bg-border hover:text-sage'
	].join(' ')}
>
	<svg
		viewBox="0 0 24 24"
		class="h-5 w-5"
		fill="none"
		stroke="currentColor"
		stroke-width="2"
	>
		<path
			stroke-linecap="round"
			stroke-linejoin="round"
			d="M17 1l4 4-4 4M3 11V9a4 4 0 014-4h14M7 23l-4-4 4-4M21 13v2a4 4 0 01-4 4H3"
		/>
	</svg>
	{#if typeof localCount === 'number'}
		<span class="tabular-nums">{localCount}</span>
	{:else}
		<span class="sr-only">Repost</span>
	{/if}
</button>

<RepostDialog
	{postUuid}
	{authorUsername}
	bind:open={dialogOpen}
	onClose={() => (dialogOpen = false)}
	onReposted={onDialogReposted}
/>
