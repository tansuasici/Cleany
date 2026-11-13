import { invoke } from '@tauri-apps/api/core'

export interface AppInfo {
  name: string
  version: string
  os: string
  arch: string
}

export const getAppInfo = () => invoke<AppInfo>('app_info')
