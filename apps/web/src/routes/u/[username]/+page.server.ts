import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { apiFetch } from '$lib/api/core';
import { fetchUserPosts } from '$lib/api/feed';

export const load: PageServerLoad = async ({ locals, params, request }) => {
	if (!locals.user) throw redirect(303, '/');
	const cookies = request.headers.get('cookie') ?? '';

	const profileRes = await apiFetch(`/api/users/${encodeURIComponent(params.username)}`, {
		cookies
	});
	if (profileRes.status === 404) throw error(404, 'Gebruiker niet gevonden');
	if (!profileRes.ok) throw error(500, 'Kon profiel niet laden');
	const profile = (await profileRes.json()).data as {
		uuid: string;
		username: string;
		display_name: string;
		bio: string;
		avatar_url?: string | null;
		banner_url?: string | null;
		location_city?: string | null;
		location_country?: string | null;
		created_at: string;
		is_private: boolean;
		followers_count: number;
		following_count: number;
		follow_state: 'none' | 'pending' | 'active' | 'self';
	};

	const posts = await fetchUserPosts(params.username, { limit: 20 }, cookies).catch(() => ({
		data: [],
		cursor: null,
		has_more: false
	}));

	return { user: locals.user, profile, posts };
};
