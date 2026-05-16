import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    projects: [
      {
        extends: true,
        test: {
          name: 'backend',
          include: ['backend/**/*.mts'],
          exclude: ['backend/**/*.mock.test.mts'],
        },
      },
      {
        extends: true,
        test: {
          name: 'backend-mocks',
          include: ['backend/**/*.mock.test.mts'],
        },
      },
      {
        extends: true,
        test: {
          name: 'all',
          include: ['**/*.generated.test.mts'],
        },
      },
    ],
  },
})
