import { fail, redirect } from '@sveltejs/kit';
import type { Actions } from './$types';
import { patchMe } from '$lib/api';

export const actions: Actions = {
	default: async ({ request, cookies }) => {
		const data = await request.formData();
		const username = String(data.get('username') ?? '').trim();
		const display_name = String(data.get('display_name') ?? '').trim();

		if (username.length < 3) {
			return fail(400, { error: 'Gebruikersnaam moet minstens 3 tekens zijn', username });
		}

		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');

		try {
			await patchMe(
				{
					username,
					display_name: display_name || undefined
				},
				cookieHeader
			);
		} catch (err) {
			const e = err as Error & { code?: string };
			return fail(400, { error: e.message, code: e.code, username });
		}

		throw redirect(303, '/onboarding/avatar');
	}
};
