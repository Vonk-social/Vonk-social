<script lang="ts">
	import Button from '$lib/components/Button.svelte';
	import ThemeToggle from '$lib/components/ui/ThemeToggle.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import VonkLogo from '$lib/components/VonkLogo.svelte';
	import PostCard from '$lib/components/feed/PostCard.svelte';
	import PostComposer from '$lib/components/feed/PostComposer.svelte';
	import StoryTray from '$lib/components/stories/StoryTray.svelte';
	import StoryViewer from '$lib/components/stories/StoryViewer.svelte';
	import { fetchFeed, type StoryGroup } from '$lib/api/feed';
	import type { PublicPost } from '$lib/api/posts';
	import type { PageProps } from './$types';
	import { untrack } from 'svelte';

	let { data }: PageProps = $props();

	let posts = $state<PublicPost[]>(untrack(() => data.feed.data));
	let cursor = $state<string | null>(untrack(() => data.feed.cursor));
	let hasMore = $state<boolean>(untrack(() => data.feed.has_more));
	let loadingMore = $state(false);

	let activeStory = $state<StoryGroup | null>(null);

	async function loadMore() {
		if (loadingMore || !hasMore) return;
		loadingMore = true;
		try {
			const page = await fetchFeed({ cursor: cursor ?? undefined, limit: 20 });
			posts = [...posts, ...page.data];
			cursor = page.cursor;
			hasMore = page.has_more;
		} finally {
			loadingMore = false;
		}
	}

	function onPosted(post: PublicPost) {
		posts = [post, ...posts];
	}
</script>

<svelte:head>
	<title>Vonk — Home</title>
</svelte:head>

<main class="mx-auto max-w-2xl px-4 py-6">
	<!-- Slim top bar — primary nav lives in the BottomNav -->
	<header class="mb-6 flex items-center justify-between">
		<a href="/home" class="flex items-center gap-2">
			<VonkLogo size={32} />
			<span class="font-display text-xl font-bold text-ink">Vonk</span>
		</a>
		<nav class="flex items-center gap-1">
			<a href="/settings" class="rounded-full p-2 hover:bg-border" aria-label="Instellingen">⚙️</a>
			<ThemeToggle />
		</nav>
	</header>

	<!-- Stories -->
	<StoryTray groups={data.stories} onOpen={(g) => (activeStory = g)} />

	<!-- Composer -->
	<section class="mb-6">
		<PostComposer user={data.user} {onPosted} />
	</section>

	<!-- Feed -->
	<section>
		{#if posts.length === 0}
			<div class="vonk-card text-center text-muted">
				Niks in je feed. Volg wat mensen of post iets om de bal aan het rollen te krijgen.
			</div>
		{:else}
			{#each posts as p (p.uuid)}
				<PostCard post={p} user={data.user} />
			{/each}
			{#if hasMore}
				<div class="mt-4 text-center">
					<Button variant="ghost" disabled={loadingMore} onclick={loadMore}>
						{loadingMore ? 'Laden…' : 'Meer'}
					</Button>
				</div>
			{/if}
		{/if}
	</section>
</main>

<Toast />

{#if activeStory}
	<StoryViewer group={activeStory} user={data.user} onClose={() => (activeStory = null)} />
{/if}
