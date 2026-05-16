const createCrawler = () => {}
const createCrawlerForHostname = () => {}

// Invalid: export specifier aliases
export {
  createCrawler as createCrawlerRecord,
  createCrawlerForHostname as createCrawlerForHostnameRecord,
}
