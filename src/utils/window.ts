import { invoke } from '@tauri-apps/api/core'
import { logger } from './logger'

export async function activateMainWindow() {
  logger.log('window', '激活主窗口')
  await invoke('activate_main_window')
}

export async function activateLoginWindow() {
  logger.log('window', '激活登录窗口')
  await invoke('activate_login_window')
}

export async function exitApp() {
  logger.log('window', '退出应用')
  await invoke('exit_app')
}
