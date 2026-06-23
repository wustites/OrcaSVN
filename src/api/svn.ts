import { invoke } from '@tauri-apps/api/core'
import type { CommandResult, SvnStatus, SvnLogEntry, SvnInfo, DiffResult, BlameLine } from '@/types'

export async function svnCheckout(
  url: string,
  path: string,
  revision?: number
): Promise<CommandResult> {
  return invoke<CommandResult>('svn_checkout', { url, path, revision })
}

export async function svnUpdate(path: string, revision?: number): Promise<CommandResult> {
  return invoke<CommandResult>('svn_update', { path, revision })
}

export async function svnCommit(
  path: string,
  message: string,
  files?: string[]
): Promise<CommandResult> {
  return invoke<CommandResult>('svn_commit', { path, message, files })
}

export async function svnStatus(path: string): Promise<SvnStatus[]> {
  return invoke<SvnStatus[]>('svn_status', { path })
}

export async function svnLog(
  path: string,
  limit?: number,
  startRev?: number,
  endRev?: number,
  keyword?: string,
  dateFrom?: string,
  dateTo?: string
): Promise<SvnLogEntry[]> {
  return invoke<SvnLogEntry[]>('svn_log', { path, limit, startRev, endRev, keyword, dateFrom, dateTo })
}

export async function svnInfo(path: string): Promise<SvnInfo> {
  return invoke<SvnInfo>('svn_info', { path })
}

export async function svnDiff(
  workspacePath: string,
  file: string,
  oldRev?: number,
  newRev?: number
): Promise<DiffResult> {
  return invoke<DiffResult>('svn_diff', { workspacePath, file, oldRev, newRev })
}

export async function svnBlame(workspacePath: string, file: string): Promise<BlameLine[]> {
  return invoke<BlameLine[]>('svn_blame', { workspacePath, file })
}

export async function svnAdd(path: string, files: string[]): Promise<CommandResult> {
  return invoke<CommandResult>('svn_add', { path, files })
}

export async function svnDelete(path: string, files: string[]): Promise<CommandResult> {
  return invoke<CommandResult>('svn_delete', { path, files })
}

export async function svnRevert(path: string, files: string[]): Promise<CommandResult> {
  return invoke<CommandResult>('svn_revert', { path, files })
}

export async function deleteUnversioned(path: string, files: string[]): Promise<CommandResult> {
  return invoke<CommandResult>('delete_unversioned', { path, files })
}

export async function svnResolve(
  path: string,
  files: string[],
  strategy: string
): Promise<CommandResult> {
  return invoke<CommandResult>('svn_resolve', { path, files, strategy })
}

export async function svnCleanup(path: string): Promise<CommandResult> {
  return invoke<CommandResult>('svn_cleanup', { path })
}

export async function svnSwitch(path: string, url: string): Promise<CommandResult> {
  return invoke<CommandResult>('svn_switch', { path, url })
}

export async function svnMerge(
  path: string,
  source: string,
  revStart: number,
  revEnd: number
): Promise<CommandResult> {
  return invoke<CommandResult>('svn_merge', { path, source, revStart, revEnd })
}

export type OpenWorkspaceTarget = 'explorer' | 'vscode' | 'terminal'

export async function openWorkspaceTarget(
  path: string,
  target: OpenWorkspaceTarget
): Promise<void> {
  return invoke<void>('open_workspace_target', { path, target })
}
