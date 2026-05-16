import { expect, test } from 'vitest';
import { db } from '../db.mts';

// BAD: retry on deadlock masks the real problem
async function withDeadlockRetry<T>(fn: () => Promise<T>, retries = 3): Promise<T> {
    try {
        return await fn();
    } catch (err: any) {
        // retry deadlock errors up to `retries` times
        if (retries > 0 && err.code === '40P01') {
            return withDeadlockRetry(fn, retries - 1);
        }
        throw err;
    }
}

test('transfers funds with deadlock retries', async () => {
    const result = await withDeadlockRetry(() =>
        db.transaction(async (tx) => {
            await tx.execute(`UPDATE accounts SET balance = balance - 100 WHERE id = 'A'`);
            await tx.execute(`UPDATE accounts SET balance = balance + 100 WHERE id = 'B'`);
        })
    );
    expect(result).toBeDefined();
});
