export function aliasFunction(value: string) {
  return originalFunction(value);
}

function originalFunction(value: string) {
  return value.trim();
}
