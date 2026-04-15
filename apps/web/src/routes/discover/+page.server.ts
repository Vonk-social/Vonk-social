import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { suggestUsers } from '$lib/api/users';

export const load: PageServerLoad = async ({ locals, request }) => {
	if (!locals.user) throw redirect(303, '/');
	const cookies = request.headers.get('cookie') ?? '';
	const suggestions = await suggestUsers(cookies).catch(() => []);
	return { user: locals.user, suggestions };
};
