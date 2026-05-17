import { defineConfig } from 'vitest/config'

export default defineConfig(({
  test: {
    name: 'root',
    include: ['root/**/*.test.ts'],
    exclude: 'root/**/*.skip.ts',
    projects: [
      {
        test: {
          name: 'nested',
          include: [`src/**/*.test.ts`],
          exclude: ['src/**/*.skip.ts'],
        },
      },
      {},
      1,
    ],
  },
}))
