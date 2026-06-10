export function getStatusClass(statusCode: string): string {
  switch (statusCode) {
    case 'added': return 'status-added'
    case 'modified': return 'status-modified'
    case 'deleted': return 'status-deleted'
    case 'unversioned': return 'status-unversioned'
    case 'missing': return 'status-missing'
    case 'conflicted': return 'status-conflicted'
    case 'replaced': return 'status-replaced'
    case 'obstructed': return 'status-obstructed'
    default: return ''
  }
}

export function getStatusLabelKey(statusCode: string): string {
  switch (statusCode) {
    case 'added': return 'status.added'
    case 'modified': return 'status.modified'
    case 'deleted': return 'status.deleted'
    case 'unversioned': return 'status.unversioned'
    case 'missing': return 'status.missing'
    case 'conflicted': return 'status.conflicted'
    case 'replaced': return 'status.replaced'
    case 'obstructed': return 'status.obstructed'
    default: return 'common.noData'
  }
}
