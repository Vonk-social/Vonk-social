/**
 * Vonk E2EE v1 — AES-256-GCM with X25519 ECDH key exchange.
 *
 *   sender: ephemeral keypair (privEph, pubEph)
 *           shared  = X25519(privEph, pubRecipient)
 *           ciphertext = AES-256-GCM(shared, nonce, plaintext)
 *           wire    = { pubEph, nonce, ciphertext }
 *
 *   recipient: pubEph from the envelope + own long-term privkey
 *              shared = X25519(privLong, pubEph)
 *              plaintext = AES-256-GCM-decrypt(shared, nonce, ciphertext)
 *
 * The long-term keypair is generated on first use and stored in
 * IndexedDB as CryptoKey-compatible base64url. The public half is
 * POSTed to `/api/users/me` (as `public_key`) so other users can
 * encrypt to it.
 *
 * This module never transmits private key material. The server only
 * ever sees public_key, ephemeral_pubkey, nonce, ciphertext.
 */

import { x25519 } from '@noble/curves/ed25519.js';
import { gcm } from '@noble/ciphers/aes.js';

const DB_NAME = 'vonk-e2ee';
const DB_STORE = 'keypair';
const DB_ROW = 'self';

type KeyRow = { priv: string; pub: string };

function openDb(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const req = indexedDB.open(DB_NAME, 1);
		req.onupgradeneeded = () => {
			req.result.createObjectStore(DB_STORE);
		};
		req.onsuccess = () => resolve(req.result);
		req.onerror = () => reject(req.error);
	});
}

async function readKey(): Promise<KeyRow | null> {
	const db = await openDb();
	return new Promise((resolve, reject) => {
		const tx = db.transaction(DB_STORE, 'readonly');
		const store = tx.objectStore(DB_STORE);
		const req = store.get(DB_ROW);
		req.onsuccess = () => resolve((req.result as KeyRow | undefined) ?? null);
		req.onerror = () => reject(req.error);
	});
}

async function writeKey(row: KeyRow): Promise<void> {
	const db = await openDb();
	return new Promise((resolve, reject) => {
		const tx = db.transaction(DB_STORE, 'readwrite');
		tx.objectStore(DB_STORE).put(row, DB_ROW);
		tx.oncomplete = () => resolve();
		tx.onerror = () => reject(tx.error);
	});
}

function b64u(bytes: Uint8Array): string {
	let bin = '';
	for (const b of bytes) bin += String.fromCharCode(b);
	return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function fromB64u(s: string): Uint8Array {
	const pad = '='.repeat((4 - (s.length % 4)) % 4);
	const norm = (s + pad).replace(/-/g, '+').replace(/_/g, '/');
	const bin = atob(norm);
	const out = new Uint8Array(bin.length);
	for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
	return out;
}

/**
 * Return the long-term keypair, generating + persisting one if this
 * browser has no existing keypair stored. Call on first access to the
 * snap/DM surface.
 */
export async function getOrCreateKeypair(): Promise<KeyRow> {
	const existing = await readKey();
	if (existing) return existing;

	const priv = x25519.utils.randomSecretKey();
	const pub = x25519.getPublicKey(priv);
	const row = { priv: b64u(priv), pub: b64u(pub) };
	await writeKey(row);

	// Register the public half server-side so others can encrypt to us.
	try {
		await fetch('/api/users/me', {
			method: 'PATCH',
			credentials: 'include',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ public_key: row.pub })
		});
	} catch {
		// The key is still usable locally; re-try next mount.
	}
	return row;
}

export type Envelope = {
	ephemeral_pubkey: string; // base64url
	nonce: string; // base64url (12 bytes)
	ciphertext: string; // base64url
};

/** Encrypt `plaintext` for the recipient whose X25519 pubkey is `recipientPub`. */
export async function encryptFor(
	recipientPub: string,
	plaintext: Uint8Array
): Promise<Envelope> {
	const ephPriv = x25519.utils.randomSecretKey();
	const ephPub = x25519.getPublicKey(ephPriv);
	const shared = x25519.getSharedSecret(ephPriv, fromB64u(recipientPub));
	const nonce = crypto.getRandomValues(new Uint8Array(12));
	const key = shared.slice(0, 32); // X25519 shared secret is already 32 bytes
	const ciphertext = gcm(key, nonce).encrypt(plaintext);
	return {
		ephemeral_pubkey: b64u(ephPub),
		nonce: b64u(nonce),
		ciphertext: b64u(ciphertext)
	};
}

/** Decrypt an envelope addressed to us. Throws on auth-tag mismatch. */
export async function decryptFrom(env: Envelope): Promise<Uint8Array> {
	const me = await getOrCreateKeypair();
	const privLong = fromB64u(me.priv);
	const pubEph = fromB64u(env.ephemeral_pubkey);
	const shared = x25519.getSharedSecret(privLong, pubEph);
	const key = shared.slice(0, 32);
	const nonce = fromB64u(env.nonce);
	const ct = fromB64u(env.ciphertext);
	return gcm(key, nonce).decrypt(ct);
}
