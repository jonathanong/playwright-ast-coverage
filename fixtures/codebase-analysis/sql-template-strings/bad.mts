import psql from '@data-stores/psql';
await psql.query('SELECT id FROM users WHERE id = $1');
