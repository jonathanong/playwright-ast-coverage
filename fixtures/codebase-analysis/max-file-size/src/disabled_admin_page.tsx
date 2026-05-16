// guardrails-disable-file max-file-size: admin page intentionally keeps state and actions colocated
'use client';

import { useState } from 'react';

const pad01 = 1;
const pad02 = 2;
const pad03 = 3;
const pad04 = 4;
const pad05 = 5;
const pad06 = 6;
const pad07 = 7;
const pad08 = 8;
const pad09 = 9;
const pad10 = 10;
const pad11 = 11;
const pad12 = 12;

export default function DisabledAdminPage() {
  const [selected, setSelected] = useState<string[]>([]);
  function toggle(id: string) {
    setSelected(current =>
      current.includes(id) ? current.filter(value => value !== id) : [...current, id],
    );
  }
  function replaySelected() {
    return selected.map(id => `replay:${id}`);
  }
  return (
    <main>
      <button onClick={() => toggle('job-1')}>Toggle</button>
      <button onClick={replaySelected}>Replay</button>
    </main>
  );
}
