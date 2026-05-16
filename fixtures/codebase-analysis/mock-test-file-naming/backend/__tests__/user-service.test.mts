import { expect, test, vi } from 'vitest';

// BAD: this file uses vi.mock but is not named *.mock.test.mts
vi.mock('@/services/user', () => ({
    getUser: vi.fn().mockResolvedValue({ id: 'u1', name: 'Alice' }),
}));

test('returns mocked user', async () => {
    const { getUser } = await import('@/services/user');
    const user = await getUser('u1');
    expect(user.name).toBe('Alice');
});
