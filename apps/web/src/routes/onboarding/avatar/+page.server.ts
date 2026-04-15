import { fail, redirect } from '@sveltejs/kit';
import type { Actions } from './$types';

export const actions: Actions = {
	upload: async ({ request, cookies, fetch }) => {
		const data = await request.formData();
		const file = data.get('file');
		if (!(file instanceof File) || file.size === 0) {
			return fail(400, { error: 'Geen bestand geselecteerd' });
		}

		const cookieHeader = cookies
			.getAll()
			.map((c) => `${c.name}=${c.value}`)
			.join('; ');

		const upstream = new FormData();
		upstream.append('file', file);

		const res = await fetch('/api/users/me/avatar', {
			method: 'POST',
			headers: { cookie: cookieHeader },
			body: upstream
		});

		if (!res.ok) {
			const body = (await res.json().catch(() => ({}))) as {
				error?: { message?: string };
			};
			return fail(res.status, { error: body.error?.message ?? 'Upload mislukt' });
		}

		throw redirect(303, '/onboarding/friends');
	},

	skip: () => {
		throw redirect(303, '/onboarding/friends');
	}
};
