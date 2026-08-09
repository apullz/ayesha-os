// https://docs.expo.dev/guides/using-eslint/
const { defineConfig } = require('eslint/config');

module.exports = defineConfig([
  {
    ignores: ['dist/*', 'dist-check/*', 'node_modules/*'],
  },
  ...require('eslint-config-expo/flat'),
]);
