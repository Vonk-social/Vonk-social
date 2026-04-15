<script lang="ts">
	import { untrack } from 'svelte';
	import PostCard from '$lib/components/feed/PostCard.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import { fetchBookmarks, type PublicPost } from '$lib/api/posts';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	let posts = $state<PublicPost[]>(untrack(() => data.page.data));
	let cursor = $state<string | null>(untrack(() => data.page.cursor));
	let hasMore = $state<boolean>(untrack(() => data.page.has_more));
	let loadingMore = $state(false);

	async function loadMore() {
		if (loadingMore || !hasMore) return;
		loadingMore = true;
		try {
			const p = await fetchBookmarks({ cursor: cursor ?? undefined, limit: 20 });
			posts = [...posts, ...p.data];
			cursor = p.cursor;
			hasMore = p.has_more;
		} finally {
			loadingMore = false;
		}
	}
</script>

<svelte:head>
	<title>Vonk — Bookmarks</title>
</svelte:head>

<main class="mx-auto max-w-2xl px-4 py-6">
	<a href="/home" class="mb-4 inline-block text-sm font-semibold text-muted hover:text-ink">← Feed</a>
	<h1 class="mb-6 font-display text-2xl font-bold text-ink">🔖 Je bookmarks</h1>

	{#if posts.length === 0}
		<div class="vonk-card text-center text-muted">
			Nog niks gebookmarked. Tik op het 🔖 icoon onder een post om die hier te verzamelen.
		</div>
	{:else}
		{#each posts as p (p.uuid)}
			<PostCard post={p} user={data.user} />
		{/each}
		{#if hasMore}
			<div class="mt-4 text-center">
				<button
					class="rounded-full border border-border px-4 py-2 text-sm font-semibold text-ink hover:bg-border/40"
					disabled={loadingMore}
					onclick={loadMore}
				>{loadingMore ? 'Laden…' : 'Meer'}</button>
			</div>
		{/if}
	{/if}
</main>

<Toast />
