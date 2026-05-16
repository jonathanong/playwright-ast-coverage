import { expect, test } from 'vitest';

// GOOD: no mocking at all — no finding expected
test('adds numbers', () => {
    expect(1 + 1).toBe(2);
});
