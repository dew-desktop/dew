// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { satteri } from '@astrojs/markdown-satteri';
import { relativeLinks } from './src/relative-links.mjs';
import { group } from './src/sidebar.mjs';

// https://astro.build/config
export default defineConfig({
	markdown: {
		processor: satteri({ mdastPlugins: [relativeLinks] }),
	},
	integrations: [
		starlight({
			title: 'Dew Docs',
			description: 'The modern desktop applet platform powered by sandboxed Luau and Vello rendering.',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/dew-desktop/dew' }],
			// EACH GROUP IS A DIRECTORY OF `docs/`, so a page is in the navigation by
			// existing. See `src/sidebar.mjs` for why this is not `autogenerate`.
			sidebar: [group('Guides', 'guides'), group('Reference', 'reference'), group('Specification', 'spec')],
			customCss: [
				'./src/styles/custom.css',
			],
		}),
	],
});
