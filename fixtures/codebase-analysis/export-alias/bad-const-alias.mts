const getUserEntityCacheKeys = () => ['user:1', 'user:2']

// Invalid: export const that aliases another variable
export const getUserCacheKeys = getUserEntityCacheKeys
