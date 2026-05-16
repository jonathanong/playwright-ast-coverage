import { read } from '@data-stores/psql';
const result = await read(sql`SELECT id FROM users WHERE id = $1`, [id]);
