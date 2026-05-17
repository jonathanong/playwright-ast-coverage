let noInit;
const { destructured } = source
const nested = ({ name: 'nested' })
const cyclic = cyclic
const wrapped = defineConfig((nested))
exports.default = nested

export default (wrapped)

export type OnlyType = string

const object = {
  ['computed']: 'skip',
  method() {},
  name: (`literal`),
  "quoted": 'ok',
  list: [`one`, 'two'],
  wrappedList: ([`three`]),
  nonArray: 1,
  badList: [1],
  nested,
  cyclic,
  projects: [{ name: 'one' }, 1],
}
