declare namespace App {
	interface Locals {
		/** Set by `src/route-data.ts` for every Starlight page. */
		dew: {
			topics: { label: string; href: string; current: boolean }[];
			crumbs: { label: string; href: string | undefined }[];
		};
	}
}
