import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    projects: [
      {
        test: {
          name: 'unit',
          include: ['backend/**/*.test.mts'],
        },
      },
      {
        test: {
          name: 'openai',
          include: ['integration/**/*.test.mts'],
        },
      },
      {
        test: {
          name: 'mixed',
          include: ['mixed/**/*.test.mts'],
        },
      },
    ],
  },
})
