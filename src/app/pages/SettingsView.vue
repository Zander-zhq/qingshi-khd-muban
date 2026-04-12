<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import Button from 'primevue/button'
import InputText from 'primevue/inputtext'
import Select from 'primevue/select'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

type DownloadContent = 'video_and_cover' | 'video' | 'cover'
type ParseContent = 'video_and_image' | 'video' | 'image'

const settings = ref({
  output_dir: '',
  parse_content: 'video_and_image' as ParseContent,
  download_content: 'video_and_cover' as DownloadContent,
  concurrent: 5,
  add_seq: true,
  remove_topics: false,
  remove_at: false,
})

const parseContentOptions = [
  { label: '视频及图文', value: 'video_and_image' },
  { label: '仅视频', value: 'video' },
  { label: '仅图文', value: 'image' },
]

const contentOptions = [
  { label: '视频及封面', value: 'video_and_cover' },
  { label: '仅视频', value: 'video' },
  { label: '仅封面', value: 'cover' },
]

const concurrentOptions = [1, 3, 5, 10, 15, 20]

const saved = ref(false)
const loading = ref(true)

async function loadSettings() {
  try {
    const all = await invoke<Record<string, string>>('get_all_settings')
    if (all.parse_content) settings.value.parse_content = all.parse_content as ParseContent
    if (all.download_content) settings.value.download_content = all.download_content as DownloadContent
    if (all.concurrent) settings.value.concurrent = parseInt(all.concurrent) || 5
    if (all.add_seq !== undefined) settings.value.add_seq = all.add_seq === 'true'
    if (all.remove_topics !== undefined) settings.value.remove_topics = all.remove_topics === 'true'
    if (all.remove_at !== undefined) settings.value.remove_at = all.remove_at === 'true'

    if (all.output_dir) {
      settings.value.output_dir = all.output_dir
    } else {
      const defaultDir = await invoke<string>('get_download_dir')
      settings.value.output_dir = defaultDir
      await saveSetting('output_dir', defaultDir)
    }
  } catch { /* ignore */ }
  loading.value = false
}

async function saveSetting(key: string, value: string) {
  try {
    await invoke('set_setting', { key, value })
    saved.value = true
    setTimeout(() => { saved.value = false }, 2000)
  } catch { /* ignore */ }
}

async function chooseDir() {
  try {
    const selected = await open({ directory: true, multiple: false, title: '选择下载目录' })
    if (selected) {
      settings.value.output_dir = selected
    }
  } catch { /* ignore */ }
}

const filenamePreview = computed(() => {
  let name = '她又去了更远的地方 #巴黎 #旅行碎片 @白昼小熊'
  if (settings.value.remove_topics) name = name.replace(/#[^\s#@]+/g, '').trim()
  if (settings.value.remove_at) name = name.replace(/@[^\s#@]+/g, '').trim()
  name = name.replace(/\s+/g, ' ').trim()
  const seq = settings.value.add_seq ? '1.' : ''
  return `${seq}${name || '未命名'}.mp4`
})

watch(() => settings.value.output_dir, (v) => saveSetting('output_dir', v))
watch(() => settings.value.parse_content, (v) => saveSetting('parse_content', v))
watch(() => settings.value.download_content, (v) => saveSetting('download_content', v))
watch(() => settings.value.concurrent, (v) => saveSetting('concurrent', String(v)))
watch(() => settings.value.add_seq, (v) => saveSetting('add_seq', String(v)))
watch(() => settings.value.remove_topics, (v) => saveSetting('remove_topics', String(v)))
watch(() => settings.value.remove_at, (v) => saveSetting('remove_at', String(v)))

onMounted(loadSettings)
</script>

<template>
  <div class="st-page">
    <div class="st-header">
      <h2>下载设置</h2>
      <span v-if="saved" class="st-saved"><i class="pi pi-check-circle" /> 已保存</span>
    </div>

    <div v-if="!loading" class="st-card">
      <!-- 下载目录 -->
      <div class="st-field">
        <label class="st-label">下载目录</label>
        <div class="st-dir-input">
          <InputText v-model="settings.output_dir" placeholder="请选择或输入视频保存路径" size="small" style="flex: 1" />
          <Button icon="pi pi-folder-open" severity="secondary" size="small" @click="chooseDir" />
        </div>
      </div>

      <!-- 解析内容 + 下载内容 + 并发数 -->
      <div class="st-row-2col">
        <div class="st-field">
          <label class="st-label">解析内容</label>
          <Select v-model="settings.parse_content" :options="parseContentOptions" optionLabel="label" optionValue="value" size="small" style="width: 160px" />
        </div>
        <div class="st-field">
          <label class="st-label">下载内容</label>
          <Select v-model="settings.download_content" :options="contentOptions" optionLabel="label" optionValue="value" size="small" style="width: 160px" />
        </div>
      </div>
      <div class="st-row-2col">
        <div class="st-field">
          <label class="st-label">并发数</label>
          <Select v-model="settings.concurrent" :options="concurrentOptions" size="small" style="width: 100px" />
        </div>
      </div>

      <!-- 文件名称 -->
      <div class="st-field">
        <label class="st-label">文件名称</label>
        <div class="st-checks-inline">
          <label class="st-check"><input type="checkbox" v-model="settings.add_seq" /><span>添加序号前缀</span></label>
          <label class="st-check"><input type="checkbox" v-model="settings.remove_topics" /><span>去掉 #话题</span></label>
          <label class="st-check"><input type="checkbox" v-model="settings.remove_at" /><span>去掉 @提及</span></label>
        </div>
        <p v-if="!settings.add_seq" class="st-warn"><i class="pi pi-exclamation-triangle" /> 不加序号时同名文件会被覆盖</p>
      </div>

      <!-- 文件名预览 -->
      <div class="st-preview">
        <span class="st-preview-label">预览：</span>{{ filenamePreview }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.st-page {
  padding: 20px 28px;
}

.st-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
}

.st-header h2 {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 600;
}

.st-saved {
  color: var(--p-green-500);
  font-size: 0.8rem;
  display: flex;
  align-items: center;
  gap: 4px;
}

.st-card {
  background: var(--p-surface-0);
  border: 1px solid var(--p-surface-200);
  border-radius: 8px;
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.st-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.st-label {
  font-weight: 500;
  font-size: 0.85rem;
  color: var(--p-surface-600);
}

.st-dir-input {
  display: flex;
  gap: 6px;
}

.st-row-2col {
  display: flex;
  gap: 24px;
}

.st-checks-inline {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
}

.st-check {
  display: flex;
  align-items: center;
  gap: 5px;
  cursor: pointer;
  font-size: 0.85rem;
}

.st-check input[type="checkbox"] {
  width: 14px;
  height: 14px;
  cursor: pointer;
}

.st-warn {
  margin: 0;
  font-size: 0.78rem;
  color: var(--p-orange-500);
  display: flex;
  align-items: center;
  gap: 4px;
}

.st-preview {
  background: var(--p-surface-50);
  border-radius: 4px;
  padding: 6px 10px;
  font-size: 0.8rem;
  color: var(--p-surface-500);
  font-family: monospace;
  word-break: break-all;
}

.st-preview-label {
  color: var(--p-surface-400);
}
</style>
