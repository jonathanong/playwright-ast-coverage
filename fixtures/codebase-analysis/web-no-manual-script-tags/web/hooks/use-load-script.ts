// approved location for document.createElement('script')
export const useLoadScript = (src: string) => {
  const s = document.createElement('script')
  s.src = src
  document.head.appendChild(s)
}
