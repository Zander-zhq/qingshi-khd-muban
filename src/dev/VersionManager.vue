<script setup lang="ts">
import { ref, onMounted } from 'vue'
import InputText from 'primevue/inputtext'
import Button from 'primevue/button'
import { VERSION } from '../brand'

interface VersionRecord {
  version: string
  date: string
  content: string
  downloadUrl: string
}

const versions = ref<VersionRecord[]>([])
const showEditor = ref(false)
const editing = ref<VersionRecord>({ version: '', date: '', content: '', downloadUrl: '' })
const editingIndex = ref(-1)
const successMsg = ref('')

function loadVersions() {
  try {
    const raw = localStorage.getItem('version_records')
    versions.value = raw ? JSON.parse(raw) : []
  } catch {
    versions.value = []
  }
}

function saveVersions() {
  localStorage.setItem('version_records', JSON.stringify(versions.value))
}

onMounted(loadVersions)

function addVersion() {
  editing.value = {
    version: VERSION,
    date: new Date().toISOString().split('T')[0],
    content: '',
    downloadUrl: '',
  }
  editingIndex.value = -1
  showEditor.value = true
}

function editVersion(idx: number) {
  editing.value = { ...versions.value[idx] }
  editingIndex.value = idx
  showEditor.value = true
}

function deleteVersion(idx: number) {
  versions.value.splice(idx, 1)
  saveVersions()
}

function saveCurrentVersion() {
  if (editingIndex.value >= 0) {
    versions.value[editingIndex.value] = { ...editing.value }
  } else {
    versions.value.unshift({ ...editing.value })
  }
  saveVersions()
  showEditor.value = false
  successMsg.value = '保存成功'
  setTimeout(() => successMsg.value = '', 2000)
}

function exportUpdateJson() {
  const latest = versions.value[0]
  if (!latest) return
  const manifest = {
    version: latest.version,
    notes: latest.content,
    pub_date: latest.date,
    platforms: {
      'windows-x86_64': {
        signature: '',
        url: latest.downloadUrl,
      },
    },
  }
  const json = JSON.stringify(manifest, null, 2)
  const blob = new Blob([json], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'update-manifest.json'
  a.click()
  URL.revokeObjectURL(url)
}
</script>

<template>
  <div class="dev-page">
    <div class="dev-header">
      <h1>版本升级管理</h1>
      <span class="dev-badge">仅开发环境可见</span>
      <span class="dev-current">当前版本：{{ VERSION }}</span>
      <Button label="新增版本" icon="pi pi-plus" class="p-button-sm" style="margin-left:auto" @click="addVersion" />
      <Button label="导出更新清单" icon="pi pi-download" class="p-button-sm p-button-secondary" @click="exportUpdateJson" />
    </div>

    <div v-if="successMsg" class="dev-success">{{ successMsg }}</div>

    <div class="version-list">
      <div v-for="(v, idx) in versions" :key="idx" class="version-card">
        <div class="version-header">
          <strong>{{ v.version }}</strong>
          <span class="version-date">{{ v.date }}</span>
        </div>
        <div class="version-content">{{ v.content }}</div>
        <div v-if="v.downloadUrl" class="version-url">下载：{{ v.downloadUrl }}</div>
        <div class="version-actions">
          <Button label="编辑" icon="pi pi-pencil" class="p-button-sm" @click="editVersion(idx)" />
          <Button label="删除" icon="pi pi-trash" class="p-button-sm p-button-danger" @click="deleteVersion(idx)" />
        </div>
      </div>
      <div v-if="versions.length === 0" class="version-empty">暂无版本记录</div>
    </div>

    <div v-if="showEditor" class="editor-overlay" @click.self="showEditor = false">
      <div class="editor-box">
        <h3>{{ editingIndex >= 0 ? '编辑版本' : '新增版本' }}</h3>
        <div class="editor-form">
          <label>版本号</label>
          <InputText v-model="editing.version" placeholder="V1.1.1" />
          <label>发布日期</label>
          <InputText v-model="editing.date" placeholder="2026-04-03" type="date" />
          <label>更新内容</label>
          <textarea v-model="editing.content" placeholder="本次更新内容..." class="content-area"></textarea>
          <label>下载地址（可选）</label>
          <InputText v-model="editing.downloadUrl" placeholder="https://..." />
        </div>
        <div class="editor-actions">
          <Button label="保存" icon="pi pi-check" @click="saveCurrentVersion" />
          <Button label="取消" class="p-button-secondary" @click="showEditor = false" />
        </div>
      </div>
    </div>

    <div class="dev-nav">
      <a href="/dev/brand">← 品牌管理</a>
      <a href="/login">← 返回登录</a>
    </div>
  </div>
</template>

<style scoped>
.dev-page { padding: 24px; max-width: 960px; margin: 0 auto; font-family: system-ui, sans-serif; }
.dev-header { display: flex; align-items: center; gap: 12px; margin-bottom: 20px; flex-wrap: wrap; }
.dev-header h1 { margin: 0; font-size: 1.4rem; color: #0f172a; }
.dev-badge { font-size: 0.7rem; padding: 2px 8px; background: #fef3c7; color: #92400e; border-radius: 4px; font-weight: 600; }
.dev-current { font-size: 0.82rem; color: #64748b; }
.dev-success { padding: 10px 16px; background: #f0fdf4; border: 1px solid #86efac; color: #166534; border-radius: 8px; margin-bottom: 16px; font-size: 0.88rem; }

.version-list { display: flex; flex-direction: column; gap: 12px; }
.version-card { background: #fff; border: 1.5px solid #e2e8f0; border-radius: 12px; padding: 16px; }
.version-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
.version-header strong { font-size: 1.05rem; color: #0f172a; }
.version-date { font-size: 0.78rem; color: #94a3b8; }
.version-content { font-size: 0.88rem; color: #475569; line-height: 1.7; white-space: pre-wrap; margin-bottom: 8px; }
.version-url { font-size: 0.78rem; color: #3b82f6; word-break: break-all; margin-bottom: 8px; }
.version-actions { display: flex; gap: 6px; }
.version-empty { text-align: center; padding: 40px; color: #94a3b8; font-size: 0.92rem; }

.editor-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 1000; }
.editor-box { background: #fff; border-radius: 16px; padding: 24px; width: 480px; }
.editor-box h3 { margin: 0 0 16px; font-size: 1.1rem; }
.editor-form { display: flex; flex-direction: column; gap: 8px; }
.editor-form label { font-size: 0.82rem; font-weight: 600; color: #475569; margin-top: 4px; }
.editor-form input { width: 100%; height: 36px; border: 1.5px solid #e2e8f0; border-radius: 8px; padding: 0 10px; font-size: 0.88rem; }
.content-area { width: 100%; height: 120px; border: 1.5px solid #e2e8f0; border-radius: 8px; padding: 10px; font-size: 0.88rem; font-family: inherit; resize: vertical; }
.editor-actions { display: flex; gap: 10px; margin-top: 16px; justify-content: flex-end; }

.dev-nav { margin-top: 24px; display: flex; gap: 20px; }
.dev-nav a { color: #3b82f6; font-size: 0.88rem; text-decoration: none; }
.dev-nav a:hover { text-decoration: underline; }
</style>
