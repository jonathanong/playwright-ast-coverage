import { read } from '@data-stores/psql';

export async function getOrders() {
  return read(sql`/* getOrders */ SELECT id FROM orders`);
}
