import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { nextTick } from 'vue'
import type { Router } from 'vue-router'
import { logger } from './logger'

const LAYOUT = {
  login: { width: 420, height: 640, minWidth: 420, minHeight: 640, resizable: false },
  main: { width: 1440, height: 900, minWidth: 1200, minHeight: 760, resizable: true },
} as const

async function prepareWindow(layout: keyof typeof LAYOUT) {
  const l = LAYOUT[layout]
  await invoke('prepare_window', {
    width: l.width,
    height: l.height,
    minWidth: l.minWidth,
    minHeight: l.minHeight,
    resizable: l.resizable,
  })
}

async function revealWindow() {
  await invoke('reveal_window')
}

export async function switchToMainLayout(router: Router) {
  logger.log('window', '切换到主布局 (hide → resize → route → show)')
  try {
    await prepareWindow('main')
    await router.push('/main/dashboard')
    await nextTick()
    await new Promise(r => setTimeout(r, 80))
    await revealWindow()
    logger.log('window', '切换到主布局完成')
  } catch (e) {
    logger.error('window', '切换到主布局失败', e)
    await revealWindow().catch(() => {})
    throw e
  }
}

export async function switchToLoginLayout(router: Router, targetPath: string = '/login') {
  logger.log('window', '切换到登录布局 (hide → resize → route → show)', { targetPath })
  try {
    await prepareWindow('login')
    await router.push(targetPath)
    await nextTick()
    await new Promise(r => setTimeout(r, 80))
    await revealWindow()
    logger.log('window', '切换到登录布局完成')
  } catch (e) {
    logger.error('window', '切换到登录布局失败', e)
    await revealWindow().catch(() => {})
    throw e
  }
}

export async function ensureLoginSize() {
  try {
    const win = getCurrentWindow()
    const l = LAYOUT.login
    const size = await win.innerSize()
    if (size.width > l.width + 50 || size.height > l.height + 50) {
      logger.log('window', '检测到窗口过大，调整为登录尺寸', { current: `${size.width}x${size.height}` })
      await win.setResizable(true)
      await win.setMinSize({ type: 'Logical', width: l.minWidth, height: l.minHeight })
      await win.setSize({ type: 'Logical', width: l.width, height: l.height })
      await win.setResizable(l.resizable)
      await win.center()
    }
  } catch { /* ignore */ }
}

export async function showWindow() {
  await invoke('reveal_window')
}

export async function exitApp() {
  logger.log('window', '退出应用')
  await invoke('exit_app')
}
