import { reactive } from 'vue'

export interface DialogOptions {
  title?: string
  message: string
  confirmText?: string
}

interface DialogState {
  visible: boolean
  title: string
  message: string
  confirmText: string
}

const defaultTitle = '提示'
const defaultConfirmText = '确定'

export const dialogState = reactive<DialogState>({
  visible: false,
  title: defaultTitle,
  message: '',
  confirmText: defaultConfirmText,
})

let resolvePromise: (() => void) | null = null

export function showDialog(options: DialogOptions): Promise<void> {
  if (resolvePromise) {
    resolvePromise()
    resolvePromise = null
  }

  dialogState.title = options.title?.trim() || defaultTitle
  dialogState.message = options.message
  dialogState.confirmText = options.confirmText?.trim() || defaultConfirmText
  dialogState.visible = true

  return new Promise<void>((resolve) => {
    resolvePromise = resolve
  })
}

export function closeDialog() {
  if (!dialogState.visible) return
  dialogState.visible = false

  if (resolvePromise) {
    resolvePromise()
    resolvePromise = null
  }
}
