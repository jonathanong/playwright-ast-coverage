// backend/services/email/send.mts
const result = await openai.chat.completions.create({
  model: 'gpt-4o',
  messages: [{ role: 'user', content: 'hello' }],
})
