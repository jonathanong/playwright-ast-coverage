const base = {
  name: 'commonjs',
  testDir: './src',
  testMatch: '**/*.test.ts',
  testIgnore: ['**/*.skip.ts'],
}

module.exports = defineConfig(base)
