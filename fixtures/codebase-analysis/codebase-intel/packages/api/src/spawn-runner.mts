import { exec } from 'node:child_process';

export function runTarget() {
  exec('tsx packages/api/src/spawn-target.mts');
}
