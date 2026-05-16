import { exec } from 'node:child_process';

export function run() {
    return exec('tsx ./child.mts');
}
