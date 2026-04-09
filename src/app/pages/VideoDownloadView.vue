<script setup lang="ts">
import { ref, computed } from 'vue'
import Textarea from 'primevue/textarea'
import Button from 'primevue/button'
import DataTable from 'primevue/datatable'
import Column from 'primevue/column'
import ProgressBar from 'primevue/progressbar'
import TabView from 'primevue/tabview'
import TabPanel from 'primevue/tabpanel'
import ConfirmDialog from 'primevue/confirmdialog'
import { useConfirm } from 'primevue/useconfirm'

const confirm = useConfirm()

interface VideoItem {
  id: number
  video_id: string
  platform: string
  title: string
  author: string
  duration: number
  cover_url: string
  video_url: string
  digg_count: number
  comment_count: number
  share_count: number
  status: 'parsed' | 'queued' | 'downloading' | 'completed' | 'failed'
  progress: number
  file_path: string
  file_size: number
  error_msg: string
  speed: string
  created_at: string
}

const linkText = ref('')
const isParsing = ref(false)
const parseProgress = ref({ current: 0, total: 0 })
const activeTab = ref(0)

const allVideos = ref<VideoItem[]>([])

const parsedVideos = computed(() => allVideos.value.filter(v => v.status === 'parsed'))
const downloadingVideos = computed(() => allVideos.value.filter(v => v.status === 'queued' || v.status === 'downloading'))
const failedVideos = computed(() => allVideos.value.filter(v => v.status === 'failed'))
const completedVideos = computed(() => allVideos.value.filter(v => v.status === 'completed'))

const selectedParsed = ref<VideoItem[]>([])

const tabCounts = computed(() => ({
  parsed: parsedVideos.value.length,
  downloading: downloadingVideos.value.length,
  failed: failedVideos.value.length,
  completed: completedVideos.value.length,
}))

function formatDuration(seconds: number): string {
  if (!seconds) return '--'
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
}

function formatFileSize(bytes: number): string {
  if (!bytes) return '--'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`
}

function formatCount(n: number): string {
  if (!n) return '0'
  if (n >= 10000) return `${(n / 10000).toFixed(1)}万`
  return String(n)
}

async function startParse() {
  const lines = linkText.value
    .split('\n')
    .map(l => l.trim())
    .filter(l => l.length > 0)

  if (lines.length === 0) return

  isParsing.value = true
  parseProgress.value = { current: 0, total: lines.length }

  // TODO: 实际解析逻辑将通过 Tauri invoke 调用 Rust 端
  // 这里先用模拟数据演示 UI 效果
  for (let i = 0; i < lines.length; i++) {
    await new Promise(resolve => setTimeout(resolve, 100))
    parseProgress.value.current = i + 1

    allVideos.value.push({
      id: Date.now() + i,
      video_id: `mock_${Date.now()}_${i}`,
      platform: '抖音',
      title: `示例视频 ${i + 1} - ${lines[i].substring(0, 30)}`,
      author: '测试作者',
      duration: Math.floor(Math.random() * 300) + 10,
      cover_url: '',
      video_url: '',
      digg_count: Math.floor(Math.random() * 100000),
      comment_count: Math.floor(Math.random() * 5000),
      share_count: Math.floor(Math.random() * 2000),
      status: 'parsed',
      progress: 0,
      file_path: '',
      file_size: 0,
      error_msg: '',
      speed: '',
      created_at: new Date().toLocaleString(),
    })
  }

  isParsing.value = false
  activeTab.value = 0
}

function downloadSelected() {
  for (const item of selectedParsed.value) {
    item.status = 'queued'
  }
  selectedParsed.value = []
  activeTab.value = 1
  // TODO: invoke Rust 端开始下载
}

function downloadAll() {
  for (const item of parsedVideos.value) {
    item.status = 'queued'
  }
  selectedParsed.value = []
  activeTab.value = 1
}

function retryFailed(item: VideoItem) {
  item.status = 'queued'
  item.error_msg = ''
  item.progress = 0
}

function retryAllFailed() {
  for (const item of failedVideos.value) {
    item.status = 'queued'
    item.error_msg = ''
    item.progress = 0
  }
}

function removeFailed(item: VideoItem) {
  const idx = allVideos.value.findIndex(v => v.id === item.id)
  if (idx !== -1) allVideos.value.splice(idx, 1)
}

function clearHistory() {
  confirm.require({
    message: '确定要清空所有历史记录吗？此操作不可撤销。',
    header: '清空历史记录',
    icon: 'pi pi-exclamation-triangle',
    acceptLabel: '确定清空',
    rejectLabel: '取消',
    acceptClass: 'p-button-danger',
    accept: () => {
      allVideos.value = []
      selectedParsed.value = []
      // TODO: invoke Rust 端清空 SQLite
    },
  })
}
</script>

<template>
  <div class="vd-page">
    <ConfirmDialog />

    <!-- 链接输入区 -->
    <section class="vd-input-section">
      <div class="vd-input-header">
        <span class="vd-input-title">
          <i class="pi pi-link"></i>
          粘贴视频链接
        </span>
        <span class="vd-input-hint">一行一个，支持抖音分享链接和主页链接</span>
      </div>
      <Textarea
        v-model="linkText"
        :disabled="isParsing"
        placeholder="请粘贴视频链接，一行一个&#10;例如：&#10;https://v.douyin.com/ixxxxxx/&#10;https://www.douyin.com/user/MS4wLjAB..."
        :autoResize="false"
        class="vd-textarea"
      />
      <div class="vd-action-bar">
        <div class="vd-action-left">
          <Button
            :label="isParsing ? '解析中...' : '开始解析'"
            :icon="isParsing ? 'pi pi-spin pi-spinner' : 'pi pi-search'"
            :disabled="isParsing || !linkText.trim()"
            @click="startParse"
            class="vd-parse-btn"
          />
          <Transition name="fade">
            <div v-if="isParsing" class="vd-progress-info">
              <ProgressBar
                :value="parseProgress.total ? Math.round(parseProgress.current / parseProgress.total * 100) : 0"
                :showValue="false"
                class="vd-progress-bar"
              />
              <span class="vd-progress-text">
                已解析 {{ parseProgress.current }} / {{ parseProgress.total }} 条
              </span>
            </div>
          </Transition>
        </div>
      </div>
    </section>

    <!-- 标签页区域 -->
    <section class="vd-tabs-section">
      <TabView v-model:activeIndex="activeTab" class="vd-tabview">
        <!-- 解析中 -->
        <TabPanel value="0">
          <template #header>
            <span class="vd-tab-header">
              <i class="pi pi-list"></i>
              解析中
              <span v-if="tabCounts.parsed > 0" class="vd-tab-badge">{{ tabCounts.parsed }}</span>
            </span>
          </template>

          <div class="vd-tab-toolbar">
            <div class="vd-toolbar-left">
              <span v-if="selectedParsed.length > 0" class="vd-selected-info">
                已选 {{ selectedParsed.length }} 项
              </span>
            </div>
            <div class="vd-toolbar-right">
              <Button
                label="下载选中"
                icon="pi pi-download"
                size="small"
                :disabled="selectedParsed.length === 0"
                @click="downloadSelected"
              />
              <Button
                label="全部下载"
                icon="pi pi-cloud-download"
                size="small"
                severity="secondary"
                :disabled="parsedVideos.length === 0"
                @click="downloadAll"
              />
            </div>
          </div>

          <DataTable
            v-model:selection="selectedParsed"
            :value="parsedVideos"
            dataKey="id"
            scrollable
            scrollHeight="flex"
            :virtualScrollerOptions="{ itemSize: 52 }"
            class="vd-table"
            size="small"
            stripedRows
          >
            <Column selectionMode="multiple" headerStyle="width: 3rem" />
            <Column header="#" headerStyle="width: 3.5rem">
              <template #body="{ index }">{{ index + 1 }}</template>
            </Column>
            <Column field="title" header="标题" style="min-width: 240px">
              <template #body="{ data }">
                <div class="vd-cell-title">
                  <img v-if="data.cover_url" :src="data.cover_url" class="vd-cover" alt="" />
                  <div v-else class="vd-cover-placeholder"><i class="pi pi-video"></i></div>
                  <span class="vd-title-text">{{ data.title || '未知标题' }}</span>
                </div>
              </template>
            </Column>
            <Column field="author" header="作者" style="min-width: 100px" />
            <Column field="duration" header="时长" headerStyle="width: 5rem">
              <template #body="{ data }">{{ formatDuration(data.duration) }}</template>
            </Column>
            <Column field="digg_count" header="点赞" headerStyle="width: 5.5rem">
              <template #body="{ data }">{{ formatCount(data.digg_count) }}</template>
            </Column>
          </DataTable>
        </TabPanel>

        <!-- 下载中 -->
        <TabPanel value="1">
          <template #header>
            <span class="vd-tab-header">
              <i class="pi pi-cloud-download"></i>
              下载中
              <span v-if="tabCounts.downloading > 0" class="vd-tab-badge vd-badge-blue">{{ tabCounts.downloading }}</span>
            </span>
          </template>

          <DataTable
            :value="downloadingVideos"
            dataKey="id"
            scrollable
            scrollHeight="flex"
            :virtualScrollerOptions="{ itemSize: 52 }"
            class="vd-table"
            size="small"
            stripedRows
          >
            <Column header="#" headerStyle="width: 3.5rem">
              <template #body="{ index }">{{ index + 1 }}</template>
            </Column>
            <Column field="title" header="标题" style="min-width: 240px">
              <template #body="{ data }">
                <div class="vd-cell-title">
                  <img v-if="data.cover_url" :src="data.cover_url" class="vd-cover" alt="" />
                  <div v-else class="vd-cover-placeholder"><i class="pi pi-video"></i></div>
                  <span class="vd-title-text">{{ data.title || '未知标题' }}</span>
                </div>
              </template>
            </Column>
            <Column field="author" header="作者" style="min-width: 100px" />
            <Column header="进度" style="min-width: 160px">
              <template #body="{ data }">
                <div class="vd-progress-cell">
                  <ProgressBar :value="Math.round(data.progress * 100)" :showValue="false" class="vd-dl-progress" />
                  <span class="vd-dl-percent">{{ Math.round(data.progress * 100) }}%</span>
                </div>
              </template>
            </Column>
            <Column field="speed" header="速度" headerStyle="width: 6rem">
              <template #body="{ data }">{{ data.speed || '等待中' }}</template>
            </Column>
          </DataTable>
        </TabPanel>

        <!-- 下载失败 -->
        <TabPanel value="2">
          <template #header>
            <span class="vd-tab-header">
              <i class="pi pi-exclamation-circle"></i>
              下载失败
              <span v-if="tabCounts.failed > 0" class="vd-tab-badge vd-badge-red">{{ tabCounts.failed }}</span>
            </span>
          </template>

          <div class="vd-tab-toolbar">
            <div class="vd-toolbar-left"></div>
            <div class="vd-toolbar-right">
              <Button
                label="全部重试"
                icon="pi pi-refresh"
                size="small"
                severity="warn"
                :disabled="failedVideos.length === 0"
                @click="retryAllFailed"
              />
            </div>
          </div>

          <DataTable
            :value="failedVideos"
            dataKey="id"
            scrollable
            scrollHeight="flex"
            :virtualScrollerOptions="{ itemSize: 52 }"
            class="vd-table"
            size="small"
            stripedRows
          >
            <Column header="#" headerStyle="width: 3.5rem">
              <template #body="{ index }">{{ index + 1 }}</template>
            </Column>
            <Column field="title" header="标题" style="min-width: 200px">
              <template #body="{ data }">
                <div class="vd-cell-title">
                  <img v-if="data.cover_url" :src="data.cover_url" class="vd-cover" alt="" />
                  <div v-else class="vd-cover-placeholder"><i class="pi pi-video"></i></div>
                  <span class="vd-title-text">{{ data.title || '未知标题' }}</span>
                </div>
              </template>
            </Column>
            <Column field="author" header="作者" style="min-width: 100px" />
            <Column field="error_msg" header="失败原因" style="min-width: 160px">
              <template #body="{ data }">
                <span class="vd-error-text">{{ data.error_msg || '未知错误' }}</span>
              </template>
            </Column>
            <Column header="操作" headerStyle="width: 8rem">
              <template #body="{ data }">
                <div class="vd-actions">
                  <Button icon="pi pi-refresh" size="small" text rounded title="重试" @click="retryFailed(data)" />
                  <Button icon="pi pi-trash" size="small" text rounded severity="danger" title="删除" @click="removeFailed(data)" />
                </div>
              </template>
            </Column>
          </DataTable>
        </TabPanel>

        <!-- 已完成 -->
        <TabPanel value="3">
          <template #header>
            <span class="vd-tab-header">
              <i class="pi pi-check-circle"></i>
              已完成
              <span v-if="tabCounts.completed > 0" class="vd-tab-badge vd-badge-green">{{ tabCounts.completed }}</span>
            </span>
          </template>

          <DataTable
            :value="completedVideos"
            dataKey="id"
            scrollable
            scrollHeight="flex"
            :virtualScrollerOptions="{ itemSize: 52 }"
            class="vd-table"
            size="small"
            stripedRows
          >
            <Column header="#" headerStyle="width: 3.5rem">
              <template #body="{ index }">{{ index + 1 }}</template>
            </Column>
            <Column field="title" header="标题" style="min-width: 200px">
              <template #body="{ data }">
                <div class="vd-cell-title">
                  <img v-if="data.cover_url" :src="data.cover_url" class="vd-cover" alt="" />
                  <div v-else class="vd-cover-placeholder"><i class="pi pi-video"></i></div>
                  <span class="vd-title-text">{{ data.title || '未知标题' }}</span>
                </div>
              </template>
            </Column>
            <Column field="author" header="作者" style="min-width: 100px" />
            <Column field="file_size" header="文件大小" headerStyle="width: 6rem">
              <template #body="{ data }">{{ formatFileSize(data.file_size) }}</template>
            </Column>
            <Column field="created_at" header="下载时间" headerStyle="width: 10rem" />
          </DataTable>
        </TabPanel>
      </TabView>

      <!-- 底部操作栏 -->
      <div class="vd-bottom-bar">
        <div class="vd-bottom-left">
          <span class="vd-total-info">
            共 {{ allVideos.length }} 条记录
          </span>
        </div>
        <div class="vd-bottom-right">
          <Button
            label="清空历史记录"
            icon="pi pi-trash"
            size="small"
            severity="danger"
            text
            :disabled="allVideos.length === 0"
            @click="clearHistory"
          />
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.vd-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  gap: 12px;
  overflow: hidden;
}

/* ═══ 输入区 ═══ */
.vd-input-section {
  flex-shrink: 0;
  background: #fff;
  border-radius: 14px;
  padding: 16px 18px;
  box-shadow: 0 2px 12px rgba(15, 23, 42, 0.06);
}

.vd-input-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}

.vd-input-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: #0f172a;
  display: flex;
  align-items: center;
  gap: 6px;
}

.vd-input-title i {
  color: var(--app-primary, #22c55e);
}

.vd-input-hint {
  font-size: 0.78rem;
  color: #94a3b8;
}

.vd-textarea {
  width: 100%;
  height: 120px !important;
  font-size: 0.85rem;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  resize: none;
}

.vd-action-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 10px;
}

.vd-action-left {
  display: flex;
  align-items: center;
  gap: 14px;
  flex: 1;
}

.vd-parse-btn {
  flex-shrink: 0;
}

.vd-progress-info {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  max-width: 360px;
}

.vd-progress-bar {
  flex: 1;
  height: 8px;
}

.vd-progress-text {
  font-size: 0.8rem;
  color: #64748b;
  white-space: nowrap;
}

/* ═══ 标签页 ═══ */
.vd-tabs-section {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: #fff;
  border-radius: 14px;
  box-shadow: 0 2px 12px rgba(15, 23, 42, 0.06);
  overflow: hidden;
}

.vd-tabview {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

:deep(.p-tabview-panels) {
  flex: 1;
  min-height: 0;
  padding: 0;
}

:deep(.p-tabview-panel) {
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: 0;
}

:deep(.p-tabview-nav) {
  border-bottom: 1px solid #e2e8f0;
  padding: 0 12px;
}

:deep(.p-tabview-nav li .p-tabview-nav-link) {
  padding: 10px 16px;
  font-size: 0.85rem;
}

.vd-tab-header {
  display: flex;
  align-items: center;
  gap: 6px;
}

.vd-tab-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 18px;
  padding: 0 5px;
  border-radius: 9px;
  font-size: 0.7rem;
  font-weight: 600;
  background: #e2e8f0;
  color: #475569;
}

.vd-badge-blue { background: #dbeafe; color: #2563eb; }
.vd-badge-red { background: #fee2e2; color: #dc2626; }
.vd-badge-green { background: #dcfce7; color: #16a34a; }

/* ═══ 工具栏 ═══ */
.vd-tab-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px;
  border-bottom: 1px solid #f1f5f9;
  flex-shrink: 0;
}

.vd-toolbar-right {
  display: flex;
  gap: 8px;
}

.vd-selected-info {
  font-size: 0.8rem;
  color: #64748b;
}

/* ═══ 表格 ═══ */
.vd-table {
  flex: 1;
  min-height: 0;
}

:deep(.p-datatable-wrapper) {
  flex: 1;
}

.vd-cell-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.vd-cover {
  width: 40px;
  height: 40px;
  border-radius: 6px;
  object-fit: cover;
  flex-shrink: 0;
}

.vd-cover-placeholder {
  width: 40px;
  height: 40px;
  border-radius: 6px;
  background: #f1f5f9;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: #94a3b8;
}

.vd-title-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.83rem;
}

.vd-error-text {
  font-size: 0.8rem;
  color: #dc2626;
}

.vd-actions {
  display: flex;
  gap: 2px;
}

/* ═══ 下载进度单元格 ═══ */
.vd-progress-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.vd-dl-progress {
  flex: 1;
  height: 6px;
}

.vd-dl-percent {
  font-size: 0.78rem;
  color: #64748b;
  min-width: 36px;
  text-align: right;
}

/* ═══ 底部操作栏 ═══ */
.vd-bottom-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px;
  border-top: 1px solid #f1f5f9;
  flex-shrink: 0;
}

.vd-total-info {
  font-size: 0.8rem;
  color: #94a3b8;
}

/* ═══ 过渡动画 ═══ */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.25s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
