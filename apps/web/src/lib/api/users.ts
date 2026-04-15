import { apiFetch } from './core';
import type { FollowState } from './follows';

export type UserCard = {
	uuid: string;
	username: string;
	display_name: string;
	bio?: string | null;
	avatar_url?: string | null;
	is_private: boolean;
	follow_state: FollowState;
};

export async function searchUsers(q: string, cookies?: string): Promise<UserCard[]> {
	const res = await apiFetch(`/api/users/search?q=${encodeURIComponent(q)}`, { cookies });
	if (!res.ok) throw new Error(`searchUsers ${res.status}`);
	return (await res.json()).data;
}

export async function suggestUsers(cookies?: string): Promise<UserCard[]> {
	const res = await apiFetch('/api/users/suggestions', { cookies });
	if (!res.ok) throw new Error(`suggestUsers ${res.status}`);
	return (await res.json()).data;
}
