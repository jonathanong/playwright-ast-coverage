function processXml(xmlBuffer: Buffer) {
  const xmlString = xmlBuffer.toString('utf-8');
  // Returns both the raw Buffer and a string derived from it — should be flagged
  return { xml: xmlBuffer, text: xmlString };
}
