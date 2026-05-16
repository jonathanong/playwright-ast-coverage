// bad: metadata exported from a non-page/non-layout file
export const metadata = {
  title: 'Bad Component',
  description: 'This should not have metadata',
}

export const viewport = {
  width: 'device-width',
}

export function generateMetadata() {
  return { title: 'Dynamic' }
}
