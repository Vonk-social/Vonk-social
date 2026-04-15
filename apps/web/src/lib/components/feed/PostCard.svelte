<script lang="ts">
	import { untrack } from 'svelte';
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import LikeButton from './LikeButton.svelte';
	import MediaGrid from './MediaGrid.svelte';
	import PostComposer from './PostComposer.svelte';
	import Self from './PostCard.svelte';
	import type { PublicPost } from '$lib/api/posts';
	import { fetchReplies } from '$lib/api/posts';
	import type { SessionUser } from '$lib/api/core';
	import { toasts } from '$lib/stores/toasts';

	type Props = {
		post: PublicPost;
		/** Current user — enables inline reply composer. Optional so public
		 *  timelines can still render a card without composer access. */
		user?: SessionUser | null;
		/** Nesting depth. Replies render at depth + 1. Beyond maxDepth the
		 *  reply button links to /post/:uuid instead of expanding inline. */
		depth?: number;
	};
	const MAX_DEPTH = 1;

	let { post, user = null, depth = 0 }: Props = $props();

	let expanded = $state(false);
	let replies = $state<PublicPost[]>([]);
	let cursor = $state<string | null>(null);
	let hasMore = $state(false);
	let loading = $state(false);
	let loaded = $state(false);
	let replyCount = $state(untrack(() => post.reply_count));

	const when = $derived(relativeTime(post.created_at));
	const visibilityLabel = $derived(
		post.visibility === 'public'
			? 'Publiek'
			: post.visibility === 'followers'
				? 'Volgers'
				: 'Genoemd'
	);
	const canExpandInline = $derived(depth < MAX_DEPTH);

	async function load() {
		if (loading || loaded) return;
		loading = true;
		try {
			const page = await fetchReplies(post.uuid, { limit: 20 });
			replies = page.data;
			cursor = page.cursor;
			hasMore = page.has_more;
			loaded = true;
		} catch (e) {
			toasts.push('error', (e as Error).message);
		} finally {
			loading = false;
		}
	}

	async function loadMore() {
		if (!hasMore || loading) return;
		loading = true;
		try {
			const page = await fetchReplies(post.uuid, { cursor: cursor ?? undefined, limit: 20 });
			replies = [...replies, ...page.data];
			cursor = page.cursor;
			hasMore = page.has_more;
		} finally {
			loading = false;
		}
	}

	async function toggle() {
		if (!canExpandInline) return;
		expanded = !expanded;
		if (expanded && !loaded) await load();
	}

	function onReplied(reply: PublicPost) {
		replies = [...replies, reply];
		replyCount += 1;
	}

	function relativeTime(iso: string): string {
		const then = new Date(iso).getTime();
		const now = Date.now();
		const s = Math.max(0, Math.round((now - then) / 1000));
		if (s < 60) return 'zojuist';
		const m = Math.round(s / 60);
		if (m < 60) return `${m} min`;
		const h = Math.round(m / 60);
		if (h < 24) return `${h} u`;
		const d = Math.round(h / 24);
		if (d < 7) return `${d} d`;
		return new Date(iso).toLocaleDateString('nl-BE');
	}
</script>

<article
	class:ml-8={depth > 0}
	class:border-l-2={depth > 0}
	class:border-border={depth > 0}
	class:pl-4={depth > 0}
	class="vonk-card mb-4"
	aria-label="Post van {post.author.display_name}, {when}"
>
	<header class="flex items-center gap-3">
		<a href="/u/{post.author.username}" class="shrink-0">
			<Avatar url={post.author.avatar_url} name={post.author.display_name} size={44} />
		</a>
		<div class="min-w-0 flex-1">
			<a href="/u/{post.author.username}" class="font-bold text-ink hover:underline">
				{post.author.display_name}
			</a>
			<p class="text-sm text-muted">
				@{post.author.username} · {when}
				{#if post.is_edited} · bewerkt{/if}
				·
				<a
					href="/post/{post.uuid}"
					class="hover:underline"
					title="Open deze post op een eigen pagina"
				>
					<span class="inline-block rounded-full bg-border/50 px-2 py-0.5 text-xs">
						{visibilityLabel}
					</span>
				</a>
			</p>
		</div>
	</header>

	{#if post.content}
		<p class="mt-3 whitespace-pre-wrap text-ink" style="word-break: break-word;">
			{post.content}
		</p>
	{/if}

	<MediaGrid media={post.media} />

	<footer class="mt-3 flex items-center gap-2 text-sm text-muted">
		<LikeButton postUuid={post.uuid} initial={post.liked_by_me} />
		{#if canExpandInline}
			<button
				type="button"
				onclick={toggle}
				aria-expanded={expanded}
				aria-controls="replies-{post.uuid}"
				class="inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 font-semibold hover:bg-border"
				class:bg-border={expanded}
			>
				💬 <span>{replyCount}</span>
				<span class="sr-only">{expanded ? 'Sluit reacties' : 'Open reacties'}</span>
			</button>
		{:else}
			<a
				href="/post/{post.uuid}"
				class="inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 font-semibold hover:bg-border"
			>
				💬 <span>{replyCount}</span>
			</a>
		{/if}
	</footer>

	{#if expanded}
		<section id="replies-{post.uuid}" class="mt-4 border-t border-border pt-4">
			{#if user}
				<div class="mb-3">
					<PostComposer
						{user}
						replyToUuid={post.uuid}
						placeholder="Antwoord op @{post.author.username}…"
						onPosted={onReplied}
					/>
				</div>
			{/if}

			{#if loading && replies.length === 0}
				<p class="text-sm text-muted">Reacties laden…</p>
			{:else if replies.length === 0}
				<p class="text-sm text-muted">Nog geen reacties. Wees de eerste.</p>
			{:else}
				{#each replies as r (r.uuid)}
					<Self post={r} {user} depth={depth + 1} />
				{/each}
				{#if hasMore}
					<div class="mt-2 text-center">
						<button
							type="button"
							class="rounded-full border border-border px-4 py-1.5 text-sm font-semibold text-ink hover:bg-border/40"
							disabled={loading}
							onclick={loadMore}
						>{loading ? 'Laden…' : 'Meer reacties'}</button>
					</div>
				{/if}
			{/if}
		</section>
	{/if}
</article>
