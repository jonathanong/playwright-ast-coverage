export function readRedis(redis: { get(key: string): Promise<string | null> }, id: string) {
  return redis.get(`cache:${id}`);
}
