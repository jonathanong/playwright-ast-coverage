export type ToolSearchResult = {
  query: string
  results: string[]
}

export const searchTool = {
  schema: { name: 'search', description: 'Search', parameters: {} },
  function: async (notCurrentUser: { userId: string }) => {
    return []
  },
}
