import { apiFetch } from './core';
import type { PostAuthor, PublicPost, MediaRef } from './posts';

export type StoryItem = {
	uuid: string;
	media: MediaRef[];
	created_at: string;
	expires_at?: string | null;
	viewed_by_me: boolean;
};

export type StoryGroup = {
	author: PostAuthor;
	items: StoryItem[];
	total_count: number;
	unseen_count: number;
};

export type FeedPage = {
	data: PublicPost[];
	cursor: string | null;
	has_more: boolean;
};

export async function fetchFeed(
	opts: { cursor?: string; limit?: number } = {},
	cookies?: string
): Promise<FeedPage> {
	const qs = new URLSearchParams();
	if (opts.cursor) qs.set('cursor', opts.cursor);
	if (opts.limit) qs.set('limit', String(opts.limit));
	const res = await apiFetch(`/api/feed?${qs}`, { cookies });
	if (!res.ok) throw new Error(`/api/feed ${res.status}`);
	return res.json();
}

export async function fetchStories(cookies?: string): Promise<StoryGroup[]> {
	const res = await apiFetch('/api/feed/stories', { cookies });
	if (!res.ok) throw new Error(`/api/feed/stories ${res.status}`);
	return (await res.json()).data;
}

export async function fetchUserPosts(
	username: string,
	opts: { cursor?: string; limit?: number } = {},
	cookies?: string
): Promise<FeedPage> {
	const qs = new URLSearchParams();
	if (opts.cursor) qs.set('cursor', opts.cursor);
	if (opts.limit) qs.set('limit', String(opts.limit));
	const res = await apiFetch(`/api/users/${encodeURIComponent(username)}/posts?${qs}`, {
		cookies
	});
	if (!res.ok) throw new Error(`/api/users/${username}/posts ${res.status}`);
	return res.json();
}
