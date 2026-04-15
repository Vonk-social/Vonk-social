import { redirect } from '@sveltejs/kit';
import type { PageServerLoad, Actions } from './$types';
import { fetchFeed } from '$lib/api/feed';
import { fetchStories } from '$lib/api/feed';

export const load: PageServerLoad = async ({ locals, request }) => {
	if (!locals.user) throw redirect(303, '/');
	if (locals.user.needs_onboarding) throw redirect(303, '/onboarding/username');

	const cookies = request.headers.get('cookie') ?? '';
	// Parallel — stories tray + first feed page.
	const [feed, stories] = await Promise.all([
		fetchFeed({ limit: 20 }, cookies).catch(() => ({ data: [], cursor: null, has_more: false })),
		fetchStories(cookies).catch(() => [])
	]);
	return { user: locals.user, feed, stories };
};

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
