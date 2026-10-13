import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { docsSchema } from '@astrojs/starlight/schema';

// The pages are the repository's `docs/`, not a copy of them. That directory is
// plain Markdown, readable on GitHub and with nothing to import; a page composed
// from components belongs to the site, in `src/pages/`. As with Starlight's own
// loader, a file beginning with `_` is not a page.
export const collections = {
	docs: defineCollection({
		loader: glob({ base: '../docs', pattern: '**/[^_]*.md' }),
		schema: docsSchema(),
	}),
};
