import { apiFetch } from './core';

export type FollowState = 'none' | 'pending' | 'active' | 'self';

export type FollowListItem = {
	uuid: string;
	username: string;
	display_name: string;
	avatar_url?: string | null;
	is_private: boolean;
};

export type FollowListPage = {
	data: FollowListItem[];
	cursor: string | null;
	has_more: boolean;
};

export async function follow(username: string, cookies?: string): Promise<FollowState> {
	const res = await apiFetch(`/api/users/${encodeURIComponent(username)}/follow`, {
		method: 'POST',
		cookies
	});
	if (!res.ok) throw new Error(`follow ${res.status}`);
	return (await res.json()).follow_state;
}

export async function unfollow(username: string, cookies?: string): Promise<void> {
	const res = await apiFetch(`/api/users/${encodeURIComponent(username)}/follow`, {
		method: 'DELETE',
		cookies
	});
	if (!res.ok) throw new Error(`unfollow ${res.status}`);
}

export async function acceptFollow(from: string, cookies?: string): Promise<void> {
	const res = await apiFetch(`/api/users/${encodeURIComponent(from)}/follow/accept`, {
		method: 'POST',
		cookies
	});
	if (!res.ok) throw new Error(`acceptFollow ${res.status}`);
}

export async function rejectFollow(from: string, cookies?: string): Promise<void> {
	const res = await apiFetch(`/api/users/${encodeURIComponent(from)}/follow/accept`, {
		method: 'DELETE',
		cookies
	});
	if (!res.ok) throw new Error(`rejectFollow ${res.status}`);
}

export async function fetchFollowers(
	username: string,
	opts: { cursor?: string; limit?: number } = {},
	cookies?: string
): Promise<FollowListPage> {
	const qs = new URLSearchParams();
	if (opts.cursor) qs.set('cursor', opts.cursor);
	if (opts.limit) qs.set('limit', String(opts.limit));
	const res = await apiFetch(
		`/api/users/${encodeURIComponent(username)}/followers?${qs}`,
		{ cookies }
	);
	if (!res.ok) throw new Error(`followers ${res.status}`);
	return res.json();
}

export async function fetchFollowing(
	username: string,
	opts: { cursor?: string; limit?: number } = {},
	cookies?: string
): Promise<FollowListPage> {
	const qs = new URLSearchParams();
	if (opts.cursor) qs.set('cursor', opts.cursor);
	if (opts.limit) qs.set('limit', String(opts.limit));
	const res = await apiFetch(
		`/api/users/${encodeURIComponent(username)}/following?${qs}`,
		{ cookies }
	);
	if (!res.ok) throw new Error(`following ${res.status}`);
	return res.json();
}
