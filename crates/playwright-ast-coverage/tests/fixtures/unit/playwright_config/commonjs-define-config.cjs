const config = {
  testDir: './commonjs-define-config-tests',
  testMatch: '**/*.commonjs-define-config.js',
  use: {
    baseURL: 'http://localhost:5100',
  },
};

module.exports = defineConfig(config);
