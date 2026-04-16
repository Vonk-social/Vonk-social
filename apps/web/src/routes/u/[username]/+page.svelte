<script lang="ts">
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import PostCard from '$lib/components/feed/PostCard.svelte';
	import FollowButton from '$lib/components/follow/FollowButton.svelte';
	import Toast from '$lib/components/ui/Toast.svelte';
	import { fetchUserPosts } from '$lib/api/feed';
	import type { PublicPost } from '$lib/api/posts';
	import type { PageProps } from './$types';
	import { untrack } from 'svelte';

	let { data }: PageProps = $props();

	let posts = $state<PublicPost[]>(untrack(() => data.posts.data));
	let cursor = $state<string | null>(untrack(() => data.posts.cursor));
	let hasMore = $state<boolean>(untrack(() => data.posts.has_more));
	let loadingMore = $state(false);

	async function loadMore() {
		if (loadingMore || !hasMore) return;
		loadingMore = true;
		try {
			const page = await fetchUserPosts(data.profile.username, {
				cursor: cursor ?? undefined,
				limit: 20
			});
			posts = [...posts, ...page.data];
			cursor = page.cursor;
			hasMore = page.has_more;
		} finally {
			loadingMore = false;
		}
	}
</script>

<svelte:head>
	<title>@{data.profile.username} — Vonk</title>
</svelte:head>

<main class="mx-auto max-w-2xl px-4 py-6">
	<a href="/home" class="mb-4 inline-block text-sm font-semibold text-muted hover:text-ink">← Feed</a>

	<header class="vonk-card">
		<div class="flex flex-wrap items-center gap-4">
			<Avatar url={data.profile.avatar_url} name={data.profile.display_name} size={96} />
			<div class="min-w-0 flex-1">
				<h1 class="font-display text-2xl font-bold text-ink">{data.profile.display_name}</h1>
				<p class="text-muted">@{data.profile.username}{data.profile.is_private ? ' · 🔒' : ''}</p>

				{#if data.profile.location_city || data.profile.location_country}
					<p class="mt-1 text-sm text-muted">
						📍 {[data.profile.location_city, data.profile.location_country].filter(Boolean).join(', ')}
					</p>
				{/if}

				{#if data.profile.bio && data.profile.bio.trim()}
					<p class="mt-2 whitespace-pre-wrap text-ink">{data.profile.bio}</p>
				{:else if data.profile.follow_state === 'self'}
					<p class="mt-2 text-sm text-muted italic">
						Nog geen bio. <a href="/settings" class="text-terracotta hover:underline">Voeg er een toe →</a>
					</p>
				{/if}
				<div class="mt-3 flex flex-wrap gap-4 text-sm">
					<a href="/u/{data.profile.username}/followers" class="hover:underline">
						<strong class="text-ink">{data.profile.followers_count}</strong>
						<span class="text-muted">volgers</span>
					</a>
					<a href="/u/{data.profile.username}/following" class="hover:underline">
						<strong class="text-ink">{data.profile.following_count}</strong>
						<span class="text-muted">volgend</span>
					</a>
				</div>
			</div>
			<div class="flex flex-col gap-2 self-start">
				{#if data.profile.follow_state === 'self'}
					<a
						href="/settings"
						class="inline-block rounded-full border border-border bg-white px-5 py-2 text-center text-sm font-semibold text-ink hover:bg-border/40"
					>Bewerk profiel</a>
				{:else}
					<FollowButton username={data.profile.username} initial={data.profile.follow_state} />
				{/if}
			</div>
		</div>
	</header>

	<!-- Vrienden blok — alleen op je eigen profiel -->
	{#if data.profile.follow_state === 'self'}
		<section class="mt-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
			<a
				href="/u/{data.profile.username}/followers"
				class="vonk-card flex flex-col items-center gap-1 py-4 transition-colors hover:bg-border/20"
			>
				<span class="text-2xl font-bold text-ink">{data.profile.followers_count}</span>
				<span class="text-sm text-muted">Volgers</span>
			</a>
			<a
				href="/u/{data.profile.username}/following"
				class="vonk-card flex flex-col items-center gap-1 py-4 transition-colors hover:bg-border/20"
			>
				<span class="text-2xl font-bold text-ink">{data.profile.following_count}</span>
				<span class="text-sm text-muted">Volgend</span>
			</a>
			<a
				href="/invite"
				class="vonk-card flex flex-col items-center gap-1 py-4 transition-colors hover:bg-border/20"
			>
				<span class="text-2xl">✉️</span>
				<span class="text-sm text-muted">Uitnodigen</span>
			</a>
			<a
				href="/bookmarks"
				class="vonk-card flex flex-col items-center gap-1 py-4 transition-colors hover:bg-border/20"
			>
				<span class="text-2xl">🔖</span>
				<span class="text-sm text-muted">Bookmarks</span>
			</a>
		</section>
	{/if}

	<section class="mt-6">
		<h2 class="mb-3 font-display text-lg font-bold text-ink">Posts</h2>
		{#if posts.length === 0}
			<div class="vonk-card text-center text-muted">Nog niks gepost.</div>
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
	</section>
</main>

<Toast />
