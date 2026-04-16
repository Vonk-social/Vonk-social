import { redirect } from '@sveltejs/kit';
import type { PageServerLoad, Actions } from './$types';
import { fetchFeed } from '$lib/api/feed';
import { fetchStories } from '$lib/api/feed';
import { apiFetch } from '$lib/api/core';

export const load: PageServerLoad = async ({ locals, request }) => {
	if (!locals.user) throw redirect(303, '/');
	if (locals.user.needs_onboarding) throw redirect(303, '/onboarding/username');

	const cookies = request.headers.get('cookie') ?? '';
	// Parallel — stories tray + first feed page + connection count for invite CTA.
	const [feed, stories, connectionCount] = await Promise.all([
		fetchFeed({ limit: 20 }, cookies).catch(() => ({ data: [], cursor: null, has_more: false })),
		fetchStories(cookies).catch(() => []),
		fetchConnectionCount(locals.user.username, cookies)
	]);
	return { user: locals.user, feed, stories, connectionCount };
};

async function fetchConnectionCount(username: string, cookies: string): Promise<number> {
	try {
		const res = await apiFetch(`/api/users/${encodeURIComponent(username)}`, {
			method: 'GET',
			cookies
		});
		if (!res.ok) return 0;
		const body = (await res.json()) as {
			data: { followers_count?: number; following_count?: number };
		};
		return (body.data.followers_count ?? 0) + (body.data.following_count ?? 0);
	} catch {
		return 0;
	}
}

export const actions: Actions = {
	logout: async ({ fetch, cookies }) => {
		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');
		await fetch('/api/auth/logout', {
			method: 'POST',
			headers: { cookie: cookieHeader }
		}).catch(() => {});
		cookies.delete('vonk_access', { path: '/' });
		cookies.delete('vonk_refresh', { path: '/api/auth' });
		throw redirect(303, '/');
	}
};
