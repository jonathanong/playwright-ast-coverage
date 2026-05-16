import { read, write } from '@data-stores/psql';

export async function doTooMuch(id: string) {
  await read(sql`/* a */ SELECT * FROM users WHERE id = $1`, [id]);
  await read(sql`/* b */ SELECT * FROM roles WHERE user_id = $1`, [id]);
  await write(sql`/* c */ INSERT INTO logs VALUES ($1)`, [id]);
}
