import nl from './nl.json';
import en from './en.json';

const dictionaries = { nl, en } as const;
type Locale = keyof typeof dictionaries;

/** Translate a dotted key. Falls back to NL then to the key itself. */
export function t(key: string, locale: string | null | undefined = 'nl'): string {
	const dict = (dictionaries as Record<string, Record<string, string>>)[locale ?? 'nl'] ?? nl;
	return (
		dict[key] ??
		(nl as Record<string, string>)[key] ??
		key
	);
}

export function isLocale(x: string | null | undefined): x is Locale {
	return x === 'nl' || x === 'en';
}
