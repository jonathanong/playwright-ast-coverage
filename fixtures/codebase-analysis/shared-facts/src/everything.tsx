import { Widget } from "./widget";

declare const vi: { mock(specifier: string): void };
declare function it(name: string, test: () => void): void;

vi.mock("./mocked");

it("tracks integration facts", () => {
  Widget();
});
