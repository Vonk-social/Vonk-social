import { apiFetch } from './core';

export type UploadedMedia = {
	uuid: string;
	media_type: 'image';
	width: number;
	height: number;
	variants: { thumb?: string; medium?: string; full?: string };
};

/**
 * Upload an image or blob to `POST /api/media`. Returns the media UUID you
 * attach to a post, story or snap.
 */
export async function uploadMedia(file: File | Blob, cookies?: string): Promise<UploadedMedia> {
	const fd = new FormData();
	const fileName = file instanceof File ? file.name : 'capture.webp';
	fd.append('file', file, fileName);
	const res = await apiFetch('/api/media', {
		method: 'POST',
		body: fd,
		cookies
	});
	if (!res.ok) {
		const err = (await res.json().catch(() => ({}))) as { error?: { message?: string } };
		throw new Error(err.error?.message ?? `uploadMedia ${res.status}`);
	}
	return (await res.json()).data;
}
