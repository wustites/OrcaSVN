export interface CommandResult {
  success: boolean
  output: string
  error?: string
}

export interface SvnStatus {
  path: string
  status: string
  status_code: string
  prop_status: string
  locked: boolean
  history: boolean
  switched: boolean
}

export interface SvnLogEntry {
  revision: number
  author: string
  date: string
  message: string
}

export interface SvnInfo {
  path: string
  url: string
  repository_root: string
  revision: number
  node_kind: string
  schedule: string
}

export interface DiffResult {
  path: string
  diff: string
  old_revision: number
  new_revision: number
}

export interface BlameLine {
  revision: number
  author: string
  line: string
}
