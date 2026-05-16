import { expect, test } from 'vitest';
import { db } from '../db.mts';

test('transfers funds without retry logic', async () => {
    // GOOD: no retry-on-deadlock; transaction ordering is fixed at the source
    const result = await db.transaction(async (tx) => {
        // Always lock in a consistent order (A before B) to prevent deadlocks
        await tx.execute(`UPDATE accounts SET balance = balance - 100 WHERE id = 'A'`);
        await tx.execute(`UPDATE accounts SET balance = balance + 100 WHERE id = 'B'`);
        return { success: true };
    });
    expect(result.success).toBe(true);
});
