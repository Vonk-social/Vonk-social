import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { fetchSnapInbox, fetchSentSnaps } from '$lib/api/snaps';

export const load: PageServerLoad = async ({ locals, request }) => {
	if (!locals.user) throw redirect(303, '/');
	const cookies = request.headers.get('cookie') ?? '';
	const [inbox, sent] = await Promise.all([
		fetchSnapInbox(cookies).catch(() => []),
		fetchSentSnaps(cookies).catch(() => [])
	]);
	return { user: locals.user, inbox, sent };
};
