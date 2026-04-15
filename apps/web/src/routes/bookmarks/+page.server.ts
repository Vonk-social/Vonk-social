import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { fetchBookmarks } from '$lib/api/posts';

export const load: PageServerLoad = async ({ locals, request }) => {
	if (!locals.user) throw redirect(303, '/');
	if (locals.user.needs_onboarding) throw redirect(303, '/onboarding/username');

	const cookies = request.headers.get('cookie') ?? '';
	const page = await fetchBookmarks({ limit: 20 }, cookies).catch(() => ({
		data: [],
		cursor: null,
		has_more: false
	}));
	return { user: locals.user, page };
};
