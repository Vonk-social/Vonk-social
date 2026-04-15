<script lang="ts">
	/**
	 * Compact, linked preview of a post being quoted (when `repost_of_uuid` is
	 * set and the outer post has a comment — i.e. a quote-repost). Fetches the
	 * original lazily on mount. If loading fails (deleted / not visible), we
	 * show a muted "Origineel niet beschikbaar" placeholder.
	 */
	import { onMount } from 'svelte';
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import { fetchPost, type PublicPost } from '$lib/api/posts';

	type Props = { uuid: string };
	let { uuid }: Props = $props();

	let original = $state<PublicPost | null>(null);
	let gone = $state(false);
	let loading = $state(true);

	onMount(async () => {
		try {
			original = await fetchPost(uuid);
		} catch {
			gone = true;
		} finally {
			loading = false;
		}
	});
</script>

<a
	href={original ? `/post/${original.uuid}` : '#'}
	class="mt-3 block rounded-xl border border-border bg-surface/50 p-3 transition-colors hover:bg-border/30"
	aria-label={original ? `Origineel van ${original.author.display_name}` : 'Origineel'}
>
	{#if loading}
		<p class="text-sm text-muted">Origineel laden…</p>
	{:else if gone || !original}
		<p class="text-sm text-muted italic">Origineel niet beschikbaar.</p>
	{:else}
		<header class="flex items-center gap-2">
			<Avatar url={original.author.avatar_url} name={original.author.display_name} size={24} />
			<span class="font-semibold text-ink">{original.author.display_name}</span>
			<span class="text-xs text-muted">@{original.author.username}</span>
		</header>
		{#if original.content}
			<p class="mt-2 line-clamp-3 text-sm text-ink" style="word-break: break-word;">
				{original.content}
			</p>
		{/if}
		{#if original.media.length > 0}
			<p class="mt-1 text-xs text-muted">📷 {original.media.length} foto{original.media.length === 1 ? '' : '\'s'}</p>
		{/if}
	{/if}
</a>
