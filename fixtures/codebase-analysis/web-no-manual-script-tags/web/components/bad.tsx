export const injectAnalytics = () => {
  const s = document.createElement('script')
  s.src = 'https://analytics.example.com/a.js'
  document.head.appendChild(s)
}
