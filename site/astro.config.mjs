// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { satteri } from '@astrojs/markdown-satteri';
import { callouts } from './src/markdown/callouts.mjs';
import { steps } from './src/markdown/steps.mjs';
import { trees } from './src/markdown/trees.mjs';
import { relativeLinks } from './src/markdown/relative-links.mjs';
import { REPOSITORY } from './src/repository.mjs';
import { sidebar } from './src/sidebar.mjs';
import { TOPICS } from './src/topics.mjs';

// https://astro.build/config
export default defineConfig({
	markdown: {
		processor: satteri({ mdastPlugins: [relativeLinks, callouts, steps, trees] }),
	},
	integrations: [
		starlight({
			title: 'Dew Docs',
			description: 'Build desktop applets in Luau over a native DataModel.',
			social: [{ icon: 'github', label: 'GitHub', href: REPOSITORY }],
			// EACH TOPIC IS A DIRECTORY OF `docs/` (`src/topics.mjs`), and each of its
			// groups a subdirectory. A page is in the navigation by existing; see
			// `src/sidebar.mjs`, and `src/route-data.ts` for how a topic becomes a tab.
			sidebar: sidebar(TOPICS),
			routeMiddleware: './src/route-data.ts',
			components: {
				Header: './src/components/Header.astro',
				PageTitle: './src/components/PageTitle.astro',
				Footer: './src/components/Footer.astro',
				MarkdownContent: './src/components/MarkdownContent.astro',
			},
			pagination: false,
			expressiveCode: {
				emitExternalStylesheet: false,
				// ONE THEME IN BOTH MODES. Code sits on the System Monitor's dark panel
				// either way, and a light syntax theme on it is unreadable.
				themes: ['github-dark-default'],
				useThemedScrollbars: false,
				styleOverrides: {
					borderRadius: '8px',
					borderWidth: '1px',
					codeBackground: '#090b0f',
					borderColor: '#222734',
					uiFontFamily: 'var(--sl-font-code)',
					codeFontFamily: 'var(--sl-font-code)',
					uiFontSize: '0.8rem',
					codeFontSize: '0.85rem',
					frames: {
						frameBoxShadowCssValue: 'none',
						editorTabBarBackground: '#131620',
						editorActiveTabBackground: '#090b0f',
						editorActiveTabForeground: '#56a4f0',
						editorActiveTabBorderColor: '#222734',
						editorActiveTabIndicatorTopColor: 'transparent',
						editorActiveTabIndicatorBottomColor: 'transparent',
						terminalTitlebarBackground: '#11141c',
						terminalTitlebarDotsForeground: '#56a4f0',
						terminalTitlebarDotsOpacity: '0.85',
						terminalBackground: '#090b0f',
					},
				},
			},
			customCss: [
				'@fontsource-variable/inter',
				'@fontsource-variable/jetbrains-mono',
				'./src/styles/custom.css',
			],
		}),
	],
});
