// Tool-like object in wrong location (backend/services/, not backend/tools/)
const emailTool = {
  schema: { name: 'email', description: 'Send email', parameters: {} },
  function: async (currentUser: { userId: string }) => {
    return true
  },
}

export default emailTool
