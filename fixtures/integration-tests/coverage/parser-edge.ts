const object = {
  "quoted": 'ok',
  1: 'numeric',
  wrappedList: ([`three`]),
  nonArray: 1,
  ...extra,
}

foo()
module = object
module.name = object
module.exports = 1
export default class C {}
