export const callOpenAI = /* no-mistakes: integration=openai */ async () => {
  return 'ok'
}

export const callOpenAIExpression = /* no-mistakes: integration=openai */ async () => 'ok'

export const callAnthropic = /* no-mistakes: integration=anthropic */ async () => {
  return 'wrong'
}
