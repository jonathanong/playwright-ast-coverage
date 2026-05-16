// Test file for handler.mts — exercises TestOf edge (test correspondence)
import { handleRequest } from './handler.mts';

const query = { id: 'test-id', limit: 10 };
console.assert(typeof handleRequest(query) === 'string');
