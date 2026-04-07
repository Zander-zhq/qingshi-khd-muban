<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { VERSION, getBrand } from '../brand'
import { useUserStore } from '../stores/user'
import { getAppCredentials } from '../utils/config'
import { checkUpdate, uploadExe, publishVersion } from '../api/version'
import type { UpdateVersion } from '../api/version'

const userStore = useUserStore()
const brand = getBrand()

const loading = ref(false)
const toastMsg = ref('')
let toastTimer: ReturnType<typeof setTimeout> | null = null
function toast(msg: string) {
  toastMsg.value = msg
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => toastMsg.value = '', 3000)
}

const versions = ref<UpdateVersion[]>([])
const currentVersion = VERSION.replace(/^[vV]/, '')

async function loadVersions() {
  loading.value = true
  try {
    const { appId } = await getAppCredentials()
    const res = await checkUpdate(appId, '0.0.0')
    versions.value = res.updates || []
  } catch { versions.value = [] }
  finally { loading.value = false }
}

function nextVersion(v: string): string {
  const parts = v.split('.').map(Number)
  parts[parts.length - 1]++
  return parts.join('.')
}

const showEditor = ref(false)
const editVersion = ref('')
const editDescription = ref('')
const editForceUpdate = ref(false)
const richRef = ref<HTMLElement | null>(null)

function addVersion() {
  const latest = versions.value.length > 0 ? versions.value[0].version : currentVersion
  editVersion.value = nextVersion(latest)
  editDescription.value = ''
  editForceUpdate.value = false
  showEditor.value = true
}

function onRichInput() {
  if (richRef.value) editDescription.value = richRef.value.innerHTML
}

function richExec(cmd: string, value?: string) {
  document.execCommand(cmd, false, value)
  richRef.value?.focus()
}

// build
const showBuild = ref(false)
const buildStatus = ref<'idle' | 'building' | 'done' | 'error'>('idle')
const buildLogs = ref('')
const buildOutputPath = ref('')
const pendingVersion = ref('')

let buildLogUnlisten: UnlistenFn | null = null
let buildCompleteUnlisten: UnlistenFn | null = null

async function setupBuildListeners() {
  buildLogUnlisten = await listen<string>('build-log', (e) => {
    buildLogs.value += e.payload + '\n'
  })
  buildCompleteUnlisten = await listen<{ success: boolean; output_path?: string; error?: string }>('build-complete', async (e) => {
    if (e.payload.success && e.payload.output_path) {
      buildOutputPath.value = e.payload.output_path
      buildLogs.value += `\n✅ 构建成功!\n输出路径: ${e.payload.output_path}\n`
      buildStatus.value = 'done'
      await startUpload(e.payload.output_path)
    } else {
      buildLogs.value += `\n❌ 构建失败: ${e.payload.error}\n`
      buildStatus.value = 'error'
    }
  })
}

function cleanupBuildListeners() {
  buildLogUnlisten?.()
  buildCompleteUnlisten?.()
}

async function startBuildAndPublish() {
  if (!editVersion.value.trim()) { toast('请填写版本号'); return }

  pendingVersion.value = editVersion.value.trim()
  showEditor.value = false
  buildLogs.value = `[${new Date().toLocaleTimeString()}] 开始打包版本 ${pendingVersion.value}…\n`
  buildOutputPath.value = ''
  buildStatus.value = 'building'
  uploadProgress.value = 0
  uploadStatus.value = 'idle'
  showBuild.value = true

  try {
    const { getBrandLogo: getLogo, resolveImageUrl: resolve } = await import('../brand')
    let logoData = getLogo()
    if (!logoData.startsWith('data:')) {
      const logoUrl = brand.logo ? resolve(brand.logo) : ''
      if (logoUrl && logoUrl.startsWith('http')) {
        buildLogs.value += `正在下载品牌图标…\n`
        try {
          const { fetch: tauriFetch } = await import('@tauri-apps/plugin-http')
          const resp = await tauriFetch(logoUrl)
          if (resp.ok) {
            const blob = await resp.blob()
            const blobUrl = URL.createObjectURL(blob)
            const SIZE = 256
            logoData = await new Promise<string>((resolve, reject) => {
              const img = new Image()
              img.crossOrigin = 'anonymous'
              img.onload = () => {
                const canvas = document.createElement('canvas')
                canvas.width = SIZE; canvas.height = SIZE
                const ctx = canvas.getContext('2d')!
                ctx.imageSmoothingQuality = 'high'
                ctx.drawImage(img, 0, 0, SIZE, SIZE)
                resolve(canvas.toDataURL('image/png'))
              }
              img.onerror = () => reject(new Error('Image load failed'))
              img.src = blobUrl
            })
            URL.revokeObjectURL(blobUrl)
          }
        } catch { /* use default icon */ }
      }
    }

    await invoke('start_version_build', {
      brandName: brand.brand_name,
      productName: brand.product_name || brand.brand_name,
      logoData: logoData.startsWith('data:') ? logoData : '',
    })
  } catch (e: unknown) {
    buildStatus.value = 'error'
    buildLogs.value += `\n❌ 错误: ${e}\n`
  }
}

const uploadProgress = ref(0)
const uploadStatus = ref<'idle' | 'uploading' | 'done' | 'error'>('idle')
const uploadError = ref('')

async function startUpload(outputPath: string) {
  buildLogs.value += `\n正在上传安装包到服务器…\n`
  uploadStatus.value = 'uploading'
  uploadProgress.value = 0

  try {
    const { appId } = await getAppCredentials()
    const response = await fetch(`file://${outputPath.replace(/\\/g, '/')}`)
    const blob = await response.blob()
    const file = new File([blob], outputPath.split(/[\\/]/).pop() || 'setup.exe', { type: 'application/octet-stream' })

    const res = await uploadExe(
      userStore.token, appId, pendingVersion.value, file,
      (p) => { uploadProgress.value = p },
    )

    buildLogs.value += `上传成功: ${res.url}\n`
    uploadStatus.value = 'done'

    buildLogs.value += `正在发布版本信息…\n`
    await publishVersion(userStore.token, {
      app_id: appId,
      version: pendingVersion.value,
      description: editDescription.value,
      force_update: editForceUpdate.value,
      download_url: res.url,
    })
    buildLogs.value += `✅ 版本 ${pendingVersion.value} 发布成功!\n`
    toast('版本发布成功')
    await loadVersions()
  } catch (err) {
    uploadStatus.value = 'error'
    uploadError.value = err instanceof Error ? err.message : String(err)
    buildLogs.value += `❌ 上传/发布失败: ${uploadError.value}\n`
  }
}

function closeBuild() {
  if (buildStatus.value === 'building') return
  showBuild.value = false
}

onMounted(async () => {
  await loadVersions()
  await setupBuildListeners()
  const running = await invoke<boolean>('is_build_running')
  if (running) { buildStatus.value = 'building'; showBuild.value = true }
})
onUnmounted(cleanupBuildListeners)
</script>

<template>
  <div class="vm-page">
    <div class="vm-header">
      <h1>版本升级管理</h1>
      <span class="vm-badge">仅开发环境可见</span>
      <span class="vm-current">当前版本：V{{ currentVersion }}</span>
      <div class="vm-header-actions">
        <button class="vm-btn vm-btn--primary" @click="addVersion" :disabled="loading">
          <i class="pi pi-plus"></i> 新增版本
        </button>
      </div>
    </div>

    <Transition name="toast">
      <div v-if="toastMsg" class="vm-toast">{{ toastMsg }}</div>
    </Transition>

    <div v-if="loading && !versions.length" class="vm-loading">加载中…</div>

    <!-- 版本表格 -->
    <div class="vm-table-wrap">
      <table class="vm-table">
        <thead>
          <tr>
            <th style="width:50px">状态</th>
            <th style="width:100px">版本号</th>
            <th>更新说明</th>
            <th style="width:80px">强制更新</th>
            <th style="width:160px">发布时间</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(v, idx) in versions" :key="idx">
            <td>
              <span v-if="idx === 0" class="vm-tag vm-tag--latest">最新</span>
              <span v-else class="vm-tag">历史</span>
            </td>
            <td><strong>{{ v.version }}</strong></td>
            <td><div class="vm-desc-cell" v-html="v.description || '<span class=vm-no-desc>暂无</span>'"></div></td>
            <td>
              <span v-if="v.force_update" class="vm-tag vm-tag--force">是</span>
              <span v-else class="vm-tag vm-tag--optional">否</span>
            </td>
            <td class="vm-date-cell">{{ v.created_at }}</td>
          </tr>
          <tr v-if="versions.length === 0 && !loading">
            <td colspan="5" class="vm-empty">暂无版本记录</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 新增版本弹窗 -->
    <Transition name="modal">
      <div v-if="showEditor" class="vm-overlay" @click.self="showEditor = false">
        <div class="vm-editor">
          <div class="vm-editor-header">
            <h3>新增版本</h3>
            <button class="vm-editor-close" @click="showEditor = false"><i class="pi pi-times"></i></button>
          </div>
          <div class="vm-editor-body">
            <div class="vm-field">
              <label>版本号 <em>*</em></label>
              <input v-model="editVersion" placeholder="1.1.2" />
            </div>
            <div class="vm-field">
              <label>更新说明</label>
              <div class="vm-toolbar">
                <button @click="richExec('bold')" title="加粗"><b>B</b></button>
                <button @click="richExec('italic')" title="斜体"><i>I</i></button>
                <button @click="richExec('underline')" title="下划线"><u>U</u></button>
                <span class="vm-toolbar-sep"></span>
                <button @click="richExec('insertUnorderedList')" title="列表">• 列表</button>
                <button @click="richExec('insertOrderedList')" title="有序列表">1. 列表</button>
              </div>
              <div ref="richRef" class="vm-richtext" contenteditable="true" @input="onRichInput" v-html="editDescription"></div>
            </div>
            <div class="vm-field">
              <label class="vm-check">
                <input type="checkbox" v-model="editForceUpdate" />
                <span>强制更新</span>
              </label>
            </div>
          </div>
          <div class="vm-editor-footer">
            <button class="vm-btn vm-btn--outline" @click="showEditor = false">取消</button>
            <button class="vm-btn vm-btn--primary" @click="startBuildAndPublish">
              <i class="pi pi-box"></i> 打包并发布
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <!-- 构建进度弹窗 -->
    <Transition name="modal">
      <div v-if="showBuild" class="vm-overlay" @click.self="closeBuild">
        <div class="vm-build">
          <div class="vm-build-header">
            <h3>打包版本 V{{ pendingVersion }}</h3>
            <span v-if="buildStatus === 'building'" class="vm-build-status vm-build-status--building">构建中…</span>
            <span v-else-if="uploadStatus === 'uploading'" class="vm-build-status vm-build-status--building">上传中 {{ uploadProgress }}%</span>
            <span v-else-if="buildStatus === 'done' && uploadStatus === 'done'" class="vm-build-status vm-build-status--done">发布成功</span>
            <span v-else-if="buildStatus === 'error' || uploadStatus === 'error'" class="vm-build-status vm-build-status--error">失败</span>
            <button v-if="buildStatus !== 'building'" class="vm-editor-close" @click="closeBuild"><i class="pi pi-times"></i></button>
          </div>
          <div class="vm-build-body">
            <pre class="vm-build-log">{{ buildLogs }}</pre>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.vm-page { padding: 24px; max-width: 960px; margin: 0 auto; font-family: system-ui, -apple-system, sans-serif; }
.vm-header { display: flex; align-items: center; gap: 12px; margin-bottom: 20px; flex-wrap: wrap; }
.vm-header h1 { margin: 0; font-size: 1.4rem; font-weight: 800; color: #0f172a; }
.vm-badge { font-size: 0.7rem; padding: 2px 8px; background: #fef3c7; color: #92400e; border-radius: 4px; font-weight: 600; }
.vm-current { font-size: 0.82rem; color: #64748b; }
.vm-header-actions { margin-left: auto; display: flex; gap: 8px; }

.vm-btn {
  display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px;
  border: 1.5px solid #e2e8f0; border-radius: 8px; background: #fff;
  font-size: 0.82rem; font-weight: 600; font-family: inherit; cursor: pointer; transition: all 0.15s;
}
.vm-btn--primary { background: var(--app-primary, #22c55e); color: #fff; border-color: var(--app-primary, #22c55e); }
.vm-btn--primary:hover { opacity: 0.9; }
.vm-btn--outline { background: #fff; color: #64748b; }
.vm-btn--outline:hover { border-color: #94a3b8; }
.vm-btn:disabled { opacity: 0.5; cursor: default; }

.vm-toast {
  position: fixed; top: 20px; left: 50%; transform: translateX(-50%);
  padding: 10px 24px; background: #0f172a; color: #fff; border-radius: 8px;
  font-size: 0.88rem; z-index: 9999; box-shadow: 0 4px 20px rgba(0,0,0,0.2);
}

.vm-loading, .vm-empty { text-align: center; padding: 48px; color: #94a3b8; font-size: 0.92rem; }

/* 表格 */
.vm-table-wrap { border: 1.5px solid #e2e8f0; border-radius: 12px; overflow: hidden; background: #fff; }
.vm-table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
.vm-table thead { background: #f8fafc; }
.vm-table th {
  padding: 12px 14px; text-align: left; font-weight: 700; color: #475569;
  border-bottom: 1.5px solid #e2e8f0; font-size: 0.78rem; white-space: nowrap;
}
.vm-table td { padding: 12px 14px; border-bottom: 1px solid #f1f5f9; vertical-align: top; }
.vm-table tbody tr:last-child td { border-bottom: none; }
.vm-table tbody tr:hover { background: #fafcfe; }

.vm-tag {
  display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 0.7rem; font-weight: 600;
  background: #f1f5f9; color: #64748b;
}
.vm-tag--latest { background: #f0fdf4; color: #16a34a; }
.vm-tag--force { background: #fef2f2; color: #dc2626; }
.vm-tag--optional { background: #f8fafc; color: #94a3b8; }

.vm-desc-cell { line-height: 1.6; max-height: 80px; overflow: hidden; }
.vm-desc-cell :deep(p) { margin: 2px 0; }
.vm-desc-cell :deep(ul) { margin: 2px 0; padding-left: 18px; }
.vm-no-desc { color: #cbd5e1; font-style: italic; }
.vm-date-cell { font-size: 0.78rem; color: #94a3b8; white-space: nowrap; }

/* Editor modal */
.vm-overlay {
  position: fixed; inset: 0; background: rgba(15,23,42,0.45); backdrop-filter: blur(2px);
  display: flex; align-items: center; justify-content: center; z-index: 1000;
}
.vm-editor {
  width: 600px; max-width: 90vw; background: #fff; border-radius: 16px;
  box-shadow: 0 20px 60px rgba(15,23,42,0.2); overflow: hidden;
}
.vm-editor-header {
  display: flex; align-items: center; justify-content: space-between; padding: 20px 24px 16px;
  border-bottom: 1px solid #f1f5f9;
}
.vm-editor-header h3 { margin: 0; font-size: 1.1rem; font-weight: 700; }
.vm-editor-close {
  width: 32px; height: 32px; border: none; background: #f1f5f9; border-radius: 8px;
  cursor: pointer; display: flex; align-items: center; justify-content: center; color: #64748b;
}
.vm-editor-body { padding: 20px 24px; display: flex; flex-direction: column; gap: 14px; }
.vm-editor-footer {
  display: flex; justify-content: flex-end; gap: 10px; padding: 16px 24px 20px;
  border-top: 1px solid #f1f5f9;
}

.vm-field { display: flex; flex-direction: column; gap: 6px; }
.vm-field label { font-size: 0.82rem; font-weight: 600; color: #475569; }
.vm-field label em { color: #ef4444; }
.vm-field input[type="text"], .vm-field input:not([type]) {
  height: 38px; border: 1.5px solid #e2e8f0; border-radius: 8px; padding: 0 12px;
  font-size: 0.88rem; font-family: inherit; transition: border-color 0.15s;
}
.vm-field input:focus { outline: none; border-color: var(--app-primary, #22c55e); }

.vm-toolbar {
  display: flex; gap: 4px; padding: 6px 8px; background: #f8fafc;
  border: 1.5px solid #e2e8f0; border-bottom: none; border-radius: 8px 8px 0 0;
}
.vm-toolbar button {
  padding: 4px 10px; border: 1px solid #e2e8f0; border-radius: 4px; background: #fff;
  font-size: 0.78rem; cursor: pointer; font-family: inherit; color: #475569;
}
.vm-toolbar button:hover { background: #f1f5f9; }
.vm-toolbar-sep { width: 1px; background: #e2e8f0; margin: 0 4px; }

.vm-richtext {
  min-height: 180px; border: 1.5px solid #e2e8f0; border-top: none; border-radius: 0 0 8px 8px;
  padding: 12px; font-size: 0.88rem; font-family: inherit; line-height: 1.6; outline: none;
}
.vm-richtext:focus { border-color: var(--app-primary, #22c55e); }

.vm-check {
  display: flex; align-items: center; gap: 8px; cursor: pointer; flex-direction: row !important;
}
.vm-check input { cursor: pointer; }

/* Build modal */
.vm-build {
  width: 680px; max-width: 90vw; max-height: 80vh; background: #fff; border-radius: 16px;
  box-shadow: 0 20px 60px rgba(15,23,42,0.2); overflow: hidden; display: flex; flex-direction: column;
}
.vm-build-header {
  display: flex; align-items: center; gap: 12px; padding: 20px 24px 16px;
  border-bottom: 1px solid #f1f5f9;
}
.vm-build-header h3 { margin: 0; font-size: 1.1rem; font-weight: 700; flex: 1; }
.vm-build-status { font-size: 0.78rem; padding: 3px 10px; border-radius: 6px; font-weight: 600; }
.vm-build-status--building { background: #fef3c7; color: #92400e; }
.vm-build-status--done { background: #f0fdf4; color: #166534; }
.vm-build-status--error { background: #fef2f2; color: #dc2626; }
.vm-build-body { padding: 16px 24px 20px; overflow: auto; flex: 1; }
.vm-build-log {
  font-family: 'Cascadia Code', Consolas, monospace; font-size: 0.78rem;
  line-height: 1.6; color: #334155; white-space: pre-wrap; word-break: break-all;
  background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 10px;
  padding: 16px; margin: 0; max-height: 50vh; overflow: auto;
}

/* Transitions */
.toast-enter-active, .toast-leave-active { transition: all 0.3s ease; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translateX(-50%) translateY(-10px); }
.modal-enter-active, .modal-leave-active { transition: opacity 0.2s ease; }
.modal-enter-from, .modal-leave-to { opacity: 0; }
.modal-enter-active .vm-editor, .modal-enter-active .vm-build,
.modal-leave-active .vm-editor, .modal-leave-active .vm-build { transition: transform 0.2s ease; }
.modal-enter-from .vm-editor, .modal-enter-from .vm-build { transform: scale(0.95); }
.modal-leave-to .vm-editor, .modal-leave-to .vm-build { transform: scale(0.95); }
</style>
