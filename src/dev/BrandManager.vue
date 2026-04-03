<script setup lang="ts">
import { ref, onMounted } from 'vue'
import type { BrandConfig } from '../brand'
import { getBrand, setBrand, VERSION } from '../brand'
import InputText from 'primevue/inputtext'
import Button from 'primevue/button'

const brands = ref<BrandConfig[]>([])
const editingBrand = ref<BrandConfig | null>(null)
const showEditor = ref(false)
const successMsg = ref('')

const TEMPLATES = [
  { id: 'green', label: '青拾（绿色）' },
  { id: 'orange', label: '华云（橙色）' },
  { id: 'dark', label: '科技（暗黑）' },
]

function loadBrands() {
  try {
    const raw = localStorage.getItem('brand_list')
    brands.value = raw ? JSON.parse(raw) : [getBrand()]
  } catch {
    brands.value = [getBrand()]
  }
}

function saveBrands() {
  localStorage.setItem('brand_list', JSON.stringify(brands.value))
}

onMounted(loadBrands)

function addBrand() {
  editingBrand.value = {
    id: '',
    brand_name: '',
    product_name: '',
    template: 'green',
    logo: 'app-icon.png',
    window_title: '',
    about: '',
    login_size: { width: 420, height: 640 },
    main_size: { width: 1440, height: 900, minWidth: 1200, minHeight: 760 },
  }
  showEditor.value = true
}

function editBrand(b: BrandConfig) {
  editingBrand.value = { ...b, login_size: { ...b.login_size }, main_size: { ...b.main_size } }
  showEditor.value = true
}

function deleteBrand(id: string) {
  brands.value = brands.value.filter(b => b.id !== id)
  saveBrands()
}

function saveCurrent() {
  if (!editingBrand.value) return
  const idx = brands.value.findIndex(b => b.id === editingBrand.value!.id)
  if (idx >= 0) {
    brands.value[idx] = editingBrand.value
  } else {
    brands.value.push(editingBrand.value)
  }
  saveBrands()
  showEditor.value = false
  successMsg.value = '保存成功'
  setTimeout(() => successMsg.value = '', 2000)
}

function applyBrand(b: BrandConfig) {
  setBrand(b)
  successMsg.value = `已切换到「${b.brand_name}」，刷新页面生效`
  setTimeout(() => window.location.reload(), 1000)
}

function exportBrandJson(b: BrandConfig) {
  const json = JSON.stringify(b, null, 2)
  const blob = new Blob([json], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `brand-${b.id}.json`
  a.click()
  URL.revokeObjectURL(url)
}
</script>

<template>
  <div class="dev-page">
    <div class="dev-header">
      <h1>品牌配置管理</h1>
      <span class="dev-badge">仅开发环境可见</span>
      <Button label="新增品牌" icon="pi pi-plus" class="p-button-sm" style="margin-left:auto" @click="addBrand" />
    </div>

    <div v-if="successMsg" class="dev-success">{{ successMsg }}</div>

    <div class="brand-grid">
      <div v-for="b in brands" :key="b.id" class="brand-card" :class="{ 'brand-card--active': getBrand().id === b.id }">
        <div class="brand-card-header">
          <strong>{{ b.brand_name || '未命名' }}</strong>
          <span class="brand-tpl">{{ b.template }}</span>
        </div>
        <div class="brand-card-body">
          <div>产品：{{ b.product_name }}</div>
          <div>窗口标题：{{ b.window_title }}</div>
          <div>登录尺寸：{{ b.login_size.width }}×{{ b.login_size.height }}</div>
          <div>主窗口：{{ b.main_size.width }}×{{ b.main_size.height }}</div>
        </div>
        <div class="brand-card-actions">
          <Button label="应用" icon="pi pi-check" class="p-button-sm p-button-success" @click="applyBrand(b)" />
          <Button label="编辑" icon="pi pi-pencil" class="p-button-sm" @click="editBrand(b)" />
          <Button label="导出" icon="pi pi-download" class="p-button-sm p-button-secondary" @click="exportBrandJson(b)" />
          <Button label="删除" icon="pi pi-trash" class="p-button-sm p-button-danger" @click="deleteBrand(b.id)" />
        </div>
      </div>
    </div>

    <div v-if="showEditor && editingBrand" class="editor-overlay" @click.self="showEditor = false">
      <div class="editor-box">
        <h3>{{ editingBrand.id ? '编辑品牌' : '新增品牌' }}</h3>
        <div class="editor-form">
          <label>ID（唯一标识）</label>
          <InputText v-model="editingBrand.id" placeholder="如 qingshi" />
          <label>品牌名称</label>
          <InputText v-model="editingBrand.brand_name" placeholder="如 青拾" />
          <label>产品名</label>
          <InputText v-model="editingBrand.product_name" placeholder="如 视频下载" />
          <label>模板</label>
          <select v-model="editingBrand.template" class="tpl-select">
            <option v-for="t in TEMPLATES" :key="t.id" :value="t.id">{{ t.label }}</option>
          </select>
          <label>Logo 文件名</label>
          <InputText v-model="editingBrand.logo" placeholder="app-icon.png" />
          <label>窗口标题</label>
          <InputText v-model="editingBrand.window_title" placeholder="青拾·视频下载" />
          <label>关于信息</label>
          <InputText v-model="editingBrand.about" placeholder="© 2024-2026 青拾" />
          <label>登录窗口宽×高</label>
          <div class="size-row">
            <InputText v-model.number="editingBrand.login_size.width" type="number" />
            <span>×</span>
            <InputText v-model.number="editingBrand.login_size.height" type="number" />
          </div>
          <label>主窗口宽×高</label>
          <div class="size-row">
            <InputText v-model.number="editingBrand.main_size.width" type="number" />
            <span>×</span>
            <InputText v-model.number="editingBrand.main_size.height" type="number" />
          </div>
        </div>
        <div class="editor-actions">
          <Button label="保存" icon="pi pi-check" @click="saveCurrent" />
          <Button label="取消" class="p-button-secondary" @click="showEditor = false" />
        </div>
      </div>
    </div>

    <div class="dev-nav">
      <a href="/dev/version">版本管理 →</a>
      <a href="/login">← 返回登录</a>
    </div>
  </div>
</template>

<style scoped>
.dev-page { padding: 24px; max-width: 960px; margin: 0 auto; font-family: system-ui, sans-serif; }
.dev-header { display: flex; align-items: center; gap: 12px; margin-bottom: 20px; }
.dev-header h1 { margin: 0; font-size: 1.4rem; color: #0f172a; }
.dev-badge { font-size: 0.7rem; padding: 2px 8px; background: #fef3c7; color: #92400e; border-radius: 4px; font-weight: 600; }
.dev-success { padding: 10px 16px; background: #f0fdf4; border: 1px solid #86efac; color: #166534; border-radius: 8px; margin-bottom: 16px; font-size: 0.88rem; }

.brand-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px; }
.brand-card { background: #fff; border: 1.5px solid #e2e8f0; border-radius: 12px; padding: 16px; }
.brand-card--active { border-color: #22c55e; box-shadow: 0 0 0 2px rgba(34, 197, 94, 0.15); }
.brand-card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
.brand-card-header strong { font-size: 1rem; color: #0f172a; }
.brand-tpl { font-size: 0.72rem; padding: 2px 8px; background: #f1f5f9; border-radius: 4px; color: #64748b; }
.brand-card-body { font-size: 0.82rem; color: #64748b; line-height: 1.8; margin-bottom: 12px; }
.brand-card-actions { display: flex; gap: 6px; flex-wrap: wrap; }

.editor-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 1000; }
.editor-box { background: #fff; border-radius: 16px; padding: 24px; width: 420px; max-height: 80vh; overflow-y: auto; }
.editor-box h3 { margin: 0 0 16px; font-size: 1.1rem; }
.editor-form { display: flex; flex-direction: column; gap: 8px; }
.editor-form label { font-size: 0.82rem; font-weight: 600; color: #475569; margin-top: 4px; }
.editor-form input, .tpl-select { width: 100%; height: 36px; border: 1.5px solid #e2e8f0; border-radius: 8px; padding: 0 10px; font-size: 0.88rem; }
.tpl-select { background: #fff; }
.size-row { display: flex; align-items: center; gap: 8px; }
.size-row input { flex: 1; }
.size-row span { color: #94a3b8; }
.editor-actions { display: flex; gap: 10px; margin-top: 16px; justify-content: flex-end; }

.dev-nav { margin-top: 24px; display: flex; gap: 20px; }
.dev-nav a { color: #3b82f6; font-size: 0.88rem; text-decoration: none; }
.dev-nav a:hover { text-decoration: underline; }
</style>
