import { findDiffMatch, indexDiff } from '../utils/diffDocument'
let text = ''
self.onmessage = (event: MessageEvent<
  { type: 'index'; text: string } | { type: 'find'; id: number; query: string; from: number; direction: 'next' | 'previous' }
>) => {
  const request = event.data
  if (request.type === 'index') {
    text = request.text
    const index = indexDiff(text)
    self.postMessage({ type: 'index', index }, { transfer: [index.offsets.buffer, index.oldLines.buffer, index.newLines.buffer] })
  } else {
    self.postMessage({ type: 'find', id: request.id, offset: findDiffMatch(text, request.query, request.from, request.direction) })
  }
}
