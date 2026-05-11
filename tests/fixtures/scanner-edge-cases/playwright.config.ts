const example = "escaped \" testDir: './wrong', testMatch: 'wrong.ts'";

/*
projects: [
  { testDir: './wrong-project', testMatch: 'wrong-project.ts' },
]
*/

export default {
  testDir: './tests/e2e',
  testMatch: [
    '**/*.spec.ts',
    // '**/*.commented.ts',
  ],
  use: {
    baseURL: 'http://localhost:3000',
  },
  projects: [
    { name: 'chromium', testMatch: '**/*.spec.ts' },
  ],
};
