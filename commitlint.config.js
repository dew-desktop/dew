const activeScopes = [
  'core',      // src-tauri/
  'luau',      // src-tauri/src/luau/
  'ui',        // ui/
  'types',     // types/
  'scripts',   // scripts/
  'mods',      // mods/
  'deps',      // VERSION, root config files
];

module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'scope-enum': [2, 'always', activeScopes],
    'scope-case': [2, 'always', 'lower-case'],
    'type-enum': [
      2,
      'always',
      ['feat', 'fix', 'perf', 'refactor', 'docs', 'chore', 'test', 'ci'],
    ],
  },
};
