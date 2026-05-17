test.describe('suite', () => {})
test.skip('skip', () => {})
const dynamic = 'name'
test(dynamic, 1)
test('function callback', function () {
  foo.bar.baz()
  ;(() => {})()
  function nested() {
    nestedOnly()
  }
})
describe.skip('skipped suite', () => {})
helper(1)
