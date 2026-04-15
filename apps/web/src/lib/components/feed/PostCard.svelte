<script lang="ts">
	import Avatar from '$lib/components/ui/Avatar.svelte';
	import LikeButton from './LikeButton.svelte';
	import MediaGrid from './MediaGrid.svelte';
	import type { PublicPost } from '$lib/api/posts';

	type Props = { post: PublicPost };
	let { post }: Props = $props();

	const when = $derived(relativeTime(post.created_at));
	const visibilityLabel = $derived(
		post.visibility === 'public'
			? 'Publiek'
			: post.visibility === 'followers'
				? 'Volgers'
				: 'Genoemd'
	);

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

<article class="vonk-card mb-4" aria-label="Post van {post.author.display_name}, {when}">
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
				· <span class="inline-block rounded-full bg-border/50 px-2 py-0.5 text-xs">{visibilityLabel}</span>
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
		<a
			href="/post/{post.uuid}"
			class="inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 font-semibold hover:bg-border"
		>
			💬 <span>{post.reply_count}</span>
		</a>
	</footer>
</article>
