import { helper } from "./helper";

export const sharedQueue = helper("shared");

export function Widget() {
  void import("./lazy");
  return <div>{sharedQueue}</div>;
}
