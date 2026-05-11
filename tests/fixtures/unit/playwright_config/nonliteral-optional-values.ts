const BASE_URL = process.env.BASE_URL ?? 'http://localhost:3000';
const TEST_ID = process.env.TEST_ID ?? 'data-pw';

export default {
  testDir: './tests',
  use: { baseURL: BASE_URL, testIdAttribute: TEST_ID },
};
