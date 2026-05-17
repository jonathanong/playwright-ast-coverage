const list = [1, 2, 3];
const object = { a: 1, b: 2 };
let counter = 0;

if (counter > 0) {
  counter = counter + 1;
} else {
  counter = 0;
}

for (let i = 0; i < 1; i++) {
  counter += i;
}

for (counter = 0; counter < 1; counter++) {
  counter;
}

for (const item of list) {
  counter += item;
}

for (const key in object) {
  counter += object[key];
}

try {
  counter = object?.missing?.(counter);
} catch {
  counter = 0;
} finally {
  counter = counter ?? 1;
}

switch (counter) {
  case 0:
    counter = 1;
    break;
  default:
    counter = 2;
}

function named() {
  return counter;
}

class Box {
  value() {
    return object["a"];
  }
}

export const exported = (counter++, counter satisfies number);
export default function DefaultFunction() {
  return named();
}
