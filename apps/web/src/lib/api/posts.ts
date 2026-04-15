import { apiFetch } from './core';

export type Visibility = 'public' | 'followers' | 'mentioned';
export type PostType = 'post' | 'story';

export type MediaRef = {
	uuid: string;
	media_type: 'image' | 'video';
	width?: number | null;
	height?: number | null;
	blurhash?: string | null;
	alt_text?: string | null;
	variants: { thumb?: string; medium?: string; full?: string };
};

export type PostAuthor = {
	uuid: string;
	username: string;
	display_name: string;
	avatar_url?: string | null;
};

/**
 * Public post shape. `like_count` / `bookmark_count` / `repost_count` are
 * only present when the requester is the post's author (server-side
 * `#[serde(skip_serializing_if)]`). Display code must check for presence
 * before rendering — absence means "not yours".
 */
export type PublicPost = {
	uuid: string;
	author: PostAuthor;
	content?: string | null;
	media: MediaRef[];
	post_type: PostType;
	visibility: Visibility;
	reply_to_uuid?: string | null;
	repost_of_uuid?: string | null;
	reply_count: number;
	is_edited: boolean;
	expires_at?: string | null;
	pinned_at?: string | null;
	created_at: string;
	liked_by_me: boolean;
	bookmarked_by_me: boolean;
	reposted_by_me: boolean;
	/** Author-only counters. Omitted when requester isn't the author. */
	like_count?: number;
	bookmark_count?: number;
	repost_count?: number;
};

export type CreatePostBody = {
	content?: string;
	media_uuids?: string[];
	post_type?: PostType;
	visibility?: Visibility;
	reply_to_uuid?: string;
	mentions?: string[];
};

export async function createPost(body: CreatePostBody, cookies?: string): Promise<PublicPost> {
	const res = await apiFetch('/api/posts', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body),
		cookies
	});
	if (!res.ok) throw await asError(res);
	return (await res.json()).data;
}

export async function fetchPost(uuid: string, cookies?: string): Promise<PublicPost> {
	const res = await apiFetch(`/api/posts/${uuid}`, { cookies });
	if (!res.ok) throw await asError(res);
	return (await res.json()).data;
}

export async function patchPost(
	uuid: string,
	body: { content?: string },
	cookies?: string
): Promise<PublicPost> {
	const res = await apiFetch(`/api/posts/${uuid}`, {
		method: 'PATCH',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body),
		cookies
	});
	if (!res.ok) throw await asError(res);
	return (await res.json()).data;
}

export async function deletePost(uuid: string, cookies?: string): Promise<void> {
	const res = await apiFetch(`/api/posts/${uuid}`, { method: 'DELETE', cookies });
	if (!res.ok && res.status !== 404) throw await asError(res);
}

export async function likePost(uuid: string, cookies?: string): Promise<void> {
	const res = await apiFetch(`/api/posts/${uuid}/like`, { method: 'POST', cookies });
	if (!res.ok) throw await asError(res);
}

export async function unlikePost(uuid: string, cookies?: string): Promise<void> {
	const res = await apiFetch(`/api/posts/${uuid}/like`, { method: 'DELETE', cookies });
	if (!res.ok) throw await asError(res);
}

export async function markStoryViewed(uuid: string, cookies?: string): Promise<void> {
	await apiFetch(`/api/posts/${uuid}/viewed`, { method: 'POST', cookies }).catch(() => {});
}

export async function fetchReplies(
	parentUuid: string,
	opts: { cursor?: string; limit?: number } = {},
	cookies?: string
): Promise<{ data: PublicPost[]; cursor: string | null; has_more: boolean }> {
	const qs = new URLSearchParams();
	if (opts.cursor) qs.set('cursor', opts.cursor);
	if (opts.limit) qs.set('limit', String(opts.limit));
	const res = await apiFetch(`/api/posts/${parentUuid}/replies?${qs}`, { cookies });
	if (!res.ok) throw await asError(res);
	return res.json();
}

async function asError(res: Response): Promise<Error & { code?: string; status: number }> {
	const body = (await res.json().catch(() => ({}))) as { error?: { code?: string; message?: string } };
	const e = new Error(body.error?.message ?? `HTTP ${res.status}`) as Error & {
		code?: string;
		status: number;
	};
	e.code = body.error?.code;
	e.status = res.status;
	return e;
}

// ── Bookmarks ───────────────────────────────────────────────

export async function bookmarkPost(uuid: string): Promise<void> {
	const res = await apiFetch(`/api/posts/${uuid}/bookmark`, { method: 'POST' });
	if (!res.ok) throw await asError(res);
}

export async function unbookmarkPost(uuid: string): Promise<void> {
	const res = await apiFetch(`/api/posts/${uuid}/bookmark`, { method: 'DELETE' });
	if (!res.ok) throw await asError(res);
}

export type BookmarksPage = {
	data: PublicPost[];
	cursor: string | null;
	has_more: boolean;
};

export async function fetchBookmarks(
	opts: { cursor?: string; limit?: number } = {},
	cookies?: string
): Promise<BookmarksPage> {
	const qs = new URLSearchParams();
	if (opts.cursor) qs.set('cursor', opts.cursor);
	if (opts.limit) qs.set('limit', String(opts.limit));
	const res = await apiFetch(`/api/bookmarks?${qs}`, { cookies });
	if (!res.ok) throw await asError(res);
	return res.json();
}

// ── Reposts ─────────────────────────────────────────────────

export async function repostPost(
	uuid: string,
	opts: { comment?: string; visibility?: Visibility } = {}
): Promise<PublicPost> {
	const body: Record<string, unknown> = {};
	if (opts.comment) body.comment = opts.comment;
	if (opts.visibility) body.visibility = opts.visibility;
	const res = await apiFetch(`/api/posts/${uuid}/repost`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	});
	if (!res.ok) throw await asError(res);
	return (await res.json()).data;
}

export async function unrepostPost(uuid: string): Promise<void> {
	const res = await apiFetch(`/api/posts/${uuid}/repost`, { method: 'DELETE' });
	if (!res.ok) throw await asError(res);
}

// ── Pin ─────────────────────────────────────────────────────

export async function pinPost(uuid: string): Promise<void> {
	const res = await apiFetch(`/api/posts/${uuid}/pin`, { method: 'POST' });
	if (!res.ok) throw await asError(res);
}

export async function unpinPost(uuid: string): Promise<void> {
	const res = await apiFetch(`/api/posts/${uuid}/pin`, { method: 'DELETE' });
	if (!res.ok) throw await asError(res);
}

// ── Story viewers (author-only) ─────────────────────────────

export type StoryViewer = {
	uuid: string;
	username: string;
	display_name: string;
	avatar_url?: string | null;
	viewed_at: string;
};

export async function fetchStoryViewers(uuid: string): Promise<StoryViewer[]> {
	const res = await apiFetch(`/api/posts/${uuid}/viewers`);
	if (!res.ok) throw await asError(res);
	return (await res.json()).data;
}
