import { expect, test, vi } from 'vitest';

// GOOD: correctly named *.mock.test.mts because it uses vi.mock
vi.mock('@/services/payment', () => ({
    charge: vi.fn().mockResolvedValue({ status: 'ok' }),
}));

test('charges card via mock', async () => {
    const { charge } = await import('@/services/payment');
    const result = await charge({ amount: 100, currency: 'USD' });
    expect(result.status).toBe('ok');
});
