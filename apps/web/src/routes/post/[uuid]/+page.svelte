<script lang="ts">
	import PostCard from '$lib/components/feed/PostCard.svelte';
	import PostComposer from '$lib/components/feed/PostComposer.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import type { PublicPost } from '$lib/api/posts';
	import { fetchReplies } from '$lib/api/posts';
	import type { PageProps } from './$types';
	import { untrack } from 'svelte';

	let { data }: PageProps = $props();

	let replies = $state<PublicPost[]>(untrack(() => data.replies.data));
	let cursor = $state<string | null>(untrack(() => data.replies.cursor));
	let hasMore = $state<boolean>(untrack(() => data.replies.has_more));
	let loadingMore = $state(false);

	async function loadMore() {
		if (loadingMore || !hasMore) return;
		loadingMore = true;
		try {
			const page = await fetchReplies(data.post.uuid, { cursor: cursor ?? undefined });
			replies = [...replies, ...page.data];
			cursor = page.cursor;
			hasMore = page.has_more;
		} finally {
			loadingMore = false;
		}
	}

	function onPosted(p: PublicPost) {
		replies = [...replies, p];
	}
</script>

<svelte:head>
	<title>Vonk — @{data.post.author.username}</title>
</svelte:head>

<main class="mx-auto max-w-2xl px-4 py-6">
	<a href="/home" class="mb-4 inline-block text-sm font-semibold text-muted hover:text-ink">← Terug</a>

	<PostCard post={data.post} />

	<section class="mt-6 mb-4">
		<h2 class="mb-3 font-display text-lg font-bold text-ink">Reacties</h2>
		<PostComposer
			user={data.user}
			replyToUuid={data.post.uuid}
			placeholder="Antwoord op @{data.post.author.username}…"
			{onPosted}
		/>
	</section>

	<section>
		{#if replies.length === 0}
			<p class="text-center text-muted">Nog geen antwoorden. Jij eerst?</p>
		{:else}
			{#each replies as r (r.uuid)}
				<PostCard post={r} />
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
	</section>
</main>

<Toast />
