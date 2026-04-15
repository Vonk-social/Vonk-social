<script lang="ts">
	import { untrack } from 'svelte';
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import LikeButton from './LikeButton.svelte';
	import BookmarkButton from './BookmarkButton.svelte';
	import RepostButton from './RepostButton.svelte';
	import ShareButton from './ShareButton.svelte';
	import PostMenu from './PostMenu.svelte';
	import MediaGrid from './MediaGrid.svelte';
	import PostComposer from './PostComposer.svelte';
	import QuotedPost from './QuotedPost.svelte';
	import Self from './PostCard.svelte';
	import type { PublicPost } from '$lib/api/posts';
	import { fetchReplies } from '$lib/api/posts';
	import type { SessionUser } from '$lib/api/core';
	import { toasts } from '$lib/stores/toasts';

	type Props = {
		post: PublicPost;
		user?: SessionUser | null;
		depth?: number;
	};
	const MAX_DEPTH = 1;

	let { post, user = null, depth = 0 }: Props = $props();

	let expanded = $state(false);
	let composing = $state(false);
	let replies = $state<PublicPost[]>([]);
	let cursor = $state<string | null>(null);
	let hasMore = $state(false);
	let loading = $state(false);
	let loaded = $state(false);
	let replyCount = $state(untrack(() => post.reply_count));
	// Pinned state tracked locally so the PostMenu can toggle it without
	// refetching the whole feed.
	let pinnedAt = $state(untrack(() => post.pinned_at ?? null));
	let deleted = $state(false);

	const when = $derived(relativeTime(post.created_at));
	const visibilityLabel = $derived(
		post.visibility === 'public'
			? 'Publiek'
			: post.visibility === 'followers'
				? 'Volgers'
				: 'Genoemd'
	);
	const canExpandInline = $derived(depth < MAX_DEPTH);
	const isAuthor = $derived(user?.uuid === post.author.uuid);
	const isQuoteRepost = $derived(
		!!post.repost_of_uuid && !!post.content && post.content.trim().length > 0
	);
	const isPureRepost = $derived(
		!!post.repost_of_uuid && (!post.content || post.content.trim().length === 0)
	);

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
		composing = expanded;
		if (expanded && !loaded) await load();
	}

	function onReplied(reply: PublicPost) {
		replies = [...replies, reply];
		replyCount += 1;
		composing = false;
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

{#if !deleted}
	<article
		class:ml-8={depth > 0}
		class:border-l-2={depth > 0}
		class:border-border={depth > 0}
		class:pl-4={depth > 0}
		class="vonk-card mb-4"
		aria-label="Post van {post.author.display_name}, {when}"
	>
		<!-- Pinned badge: only on profile views where pinned_at is set -->
		{#if pinnedAt}
			<p class="mb-2 text-xs font-semibold text-terracotta-dark">📌 Vastgemaakt</p>
		{/if}

		<!-- Pure-repost header: "@someone heeft dit gereposted" -->
		{#if isPureRepost}
			<p class="mb-2 text-xs font-semibold text-sage">
				🔁 {post.author.display_name} heeft dit geboost
			</p>
		{/if}

		<header class="flex items-start gap-3">
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
			{#if isAuthor}
				<PostMenu
					postUuid={post.uuid}
					pinned={pinnedAt !== null}
					onPinnedChange={(p) => (pinnedAt = p ? new Date().toISOString() : null)}
					onDeleted={() => (deleted = true)}
				/>
			{/if}
		</header>

		{#if post.content && !isPureRepost}
			<p class="mt-3 whitespace-pre-wrap text-ink" style="word-break: break-word;">
				{post.content}
			</p>
		{/if}

		<MediaGrid media={post.media} />

		<!-- Quoted post (quote-repost shows the original inside a sub-card) -->
		{#if isQuoteRepost && post.repost_of_uuid}
			<QuotedPost uuid={post.repost_of_uuid} />
		{/if}

		<!-- Pure-repost renders the ORIGINAL's preview as the body -->
		{#if isPureRepost && post.repost_of_uuid}
			<QuotedPost uuid={post.repost_of_uuid} />
		{/if}

		<footer class="mt-3 flex items-center gap-1 text-sm text-muted">
			<LikeButton postUuid={post.uuid} initial={post.liked_by_me} count={post.like_count} />
			{#if canExpandInline}
				<button
					type="button"
					onclick={toggle}
					aria-expanded={expanded}
					aria-controls="replies-{post.uuid}"
					class="inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 font-semibold text-muted transition-colors hover:bg-border hover:text-terracotta"
					class:bg-border={expanded}
				>
					<svg viewBox="0 0 24 24" class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="2">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M21 12c0 4.5-4 8-9 8a9.7 9.7 0 01-3.8-.75L3 21l1.25-4.5A8 8 0 013 12c0-4.4 4-8 9-8s9 3.6 9 8z"
						/>
					</svg>
					<span class="tabular-nums">{replyCount}</span>
					<span class="sr-only">{expanded ? 'Sluit reacties' : 'Open reacties'}</span>
				</button>
			{:else}
				<a
					href="/post/{post.uuid}"
					class="inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 font-semibold text-muted hover:bg-border hover:text-terracotta"
				>
					<svg viewBox="0 0 24 24" class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="2">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M21 12c0 4.5-4 8-9 8a9.7 9.7 0 01-3.8-.75L3 21l1.25-4.5A8 8 0 013 12c0-4.4 4-8 9-8s9 3.6 9 8z"
						/>
					</svg>
					<span class="tabular-nums">{replyCount}</span>
				</a>
			{/if}
			<RepostButton
				postUuid={post.uuid}
				authorUsername={post.author.username}
				initial={post.reposted_by_me}
				count={post.repost_count}
			/>
			<BookmarkButton
				postUuid={post.uuid}
				initial={post.bookmarked_by_me}
				count={post.bookmark_count}
			/>
			<span class="flex-1"></span>
			<ShareButton postUuid={post.uuid} authorUsername={post.author.username} />
		</footer>

		{#if expanded}
			<section id="replies-{post.uuid}" class="mt-4 border-t border-border pt-4">
				{#if user && composing}
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
				{:else if replies.length === 0 && !composing}
					<p class="text-sm text-muted">Nog geen reacties.</p>
				{:else if replies.length > 0}
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

				{#if user && !composing}
					<div class="mt-3 text-center">
						<button
							type="button"
							class="text-sm font-semibold text-terracotta hover:underline"
							onclick={() => (composing = true)}
						>Reageer opnieuw</button>
					</div>
				{/if}
			</section>
		{/if}
	</article>
{/if}
