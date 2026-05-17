import React from "react";

const list = [1, 2, 3];
const object = { a: 1, b: 2 };
let counter = 0;
const optional = object?.["a"];
const optionalCall = object?.missing?.(counter);
const spreadObject = { ...object };
const spreadArray = [...list];
const asserted = (counter as number)!;
const tagged = String.raw`count-${counter}`;

label:
{
  counter++;
}

if (counter > 0) {
  counter = counter + 1;
} else {
  throw new Error("no counter");
}

while (counter < 2) {
  counter++;
}

do {
  counter--;
} while (counter > 1);

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

object["a"]++;

try {
  counter = object?.missing?.();
} catch (error) {
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

function Named() {
  return <span>{counter}</span>;
}

class Box {
  render() {
    return <section>{new Widget(counter)}</section>;
  }
}

export const Value = (
  <Component
    attr={"value" as string}
    fallback={<span />}
    element=<span />
    {...{ label: `count-${counter}` }}
  >
    <>
      {list.map((item) => item ? <span>{item}</span> : <em />)}
      {...list}
    </>
  </Component>
);

export function Exported() {
  return <Named />;
}

export class ExportedBox {
  render() {
    return <Box />;
  }
}

export default class DefaultBox {
  render() {
    return <section>{optional}{optionalCall}{spreadObject.a}{spreadArray[0]}{asserted}{tagged}</section>;
  }
}

export const Defaulted = function () {
  const fn = function* () {
    yield counter;
  };
  const result = (counter++, counter satisfies number);
  return <button onClick={() => { counter = result; }}>{fn().next().value}</button>;
};
