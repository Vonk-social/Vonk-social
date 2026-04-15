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

/** Public post shape. NEVER has `like_count` — server strips it per CLAUDE.md §7. */
export type PublicPost = {
	uuid: string;
	author: PostAuthor;
	content?: string | null;
	media: MediaRef[];
	post_type: PostType;
	visibility: Visibility;
	reply_to_uuid?: string | null;
	reply_count: number;
	is_edited: boolean;
	expires_at?: string | null;
	created_at: string;
	liked_by_me: boolean;
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
