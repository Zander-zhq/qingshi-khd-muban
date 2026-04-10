import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { nextTick } from 'vue'
import type { Router } from 'vue-router'
import { logger } from './logger'
import { getTemplate } from '../brand'

const LOGIN_LAYOUTS: Record<string, { width: number; height: number }> = {
  green:  { width: 480, height: 720 },
  orange: { width: 760, height: 560 },
  dark:   { width: 520, height: 680 },
}
const MAIN_LAYOUT = { width: 1440, height: 900, minWidth: 1200, minHeight: 760 }

function getLoginLayout() {
  const base = LOGIN_LAYOUTS[getTemplate()] || LOGIN_LAYOUTS.green
  return { ...base, minWidth: base.width, minHeight: base.height, resizable: false }
}

function getMainLayout() {
  return { ...MAIN_LAYOUT, resizable: true }
}

async function prepareWindow(layout: 'login' | 'main') {
  const l = layout === 'login' ? getLoginLayout() : getMainLayout()
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
    const l = getLoginLayout()
    const size = await win.innerSize()
    if (size.width > l.width + 50 || size.height > l.height + 50) {
      logger.log('window', '检测到窗口过大，调整为登录尺寸', { current: `${size.width}x${size.height}` })
      await win.setResizable(true)
      await win.setMinSize({ type: 'Logical', width: l.minWidth, height: l.minHeight } as any)
      await win.setSize({ type: 'Logical', width: l.width, height: l.height } as any)
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
