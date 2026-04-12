<script setup lang="ts">
import { reactive, ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import DataTable from 'primevue/datatable'
import Column from 'primevue/column'
import Button from 'primevue/button'
import Dialog from 'primevue/dialog'
import InputText from 'primevue/inputtext'
import Textarea from 'primevue/textarea'
import Select from 'primevue/select'
import Tag from 'primevue/tag'
import ConfirmDialog from 'primevue/confirmdialog'
import { useConfirm } from 'primevue/useconfirm'

const confirm = useConfirm()

interface PlatformAccount {
  id: number
  platform: string
  name: string
  avatar: string
  cookies: string | null
  status: string
  remark: string
  created_at: string
  updated_at: string
}

const PLATFORMS = [
  { value: 'douyin', label: 'DY', url: 'https://www.douyin.com' },
  { value: 'kuaishou', label: 'KS', url: 'https://www.kuaishou.com' },
  { value: 'bilibili', label: 'BLB', url: 'https://www.bilibili.com' },
]

function platformLabel(p: string) {
  return PLATFORMS.find(x => x.value === p)?.label ?? p
}
function platformUrl(p: string) {
  return PLATFORMS.find(x => x.value === p)?.url ?? ''
}
function statusSeverity(s: string): 'success' | 'danger' | 'secondary' {
  if (s === 'active') return 'success'
  if (s === 'inactive') return 'danger'
  return 'secondary'
}
function statusText(s: string) {
  if (s === 'active') return '在线'
  if (s === 'inactive') return '离线'
  return '未登录'
}
function truncateCookies(c: string | null) {
  if (!c) return '-'
  return c.length > 30 ? c.slice(0, 30) + '...' : c
}

const loading = ref(false)
const accounts = ref<PlatformAccount[]>([])
const loggingInId = ref<number | null>(null)
const errorMsg = ref('')

const dialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const formSubmitting = ref(false)
const form = reactive({
  id: null as number | null,
  platform: 'douyin',
  name: '',
  cookies: '',
  remark: '',
})

let unlistenLogin: (() => void) | null = null
let pendingRemark = ''
let statusInterval: ReturnType<typeof setInterval> | null = null

async function fetchList() {
  loading.value = true
  try {
    accounts.value = await invoke<PlatformAccount[]>('list_download_accounts')
  } catch (e) {
    console.error('加载账号列表失败', e)
  } finally {
    loading.value = false
  }
}

async function checkAllStatus() {
  for (const row of accounts.value) {
    if (!row.cookies) continue
    try {
      const isOnline = await invoke<boolean>('check_download_cookie_status', {
        cookies: row.cookies,
        platform: row.platform,
      })
      const newStatus = isOnline ? 'active' : 'inactive'
      if (row.status !== newStatus) {
        await invoke('update_platform_account', { id: row.id, updates: { status: newStatus } })
        row.status = newStatus
      }
    } catch { /* ignore */ }
  }
}

function resetForm() {
  form.id = null
  form.platform = 'douyin'
  form.name = ''
  form.cookies = ''
  form.remark = ''
}

function onAdd() {
  resetForm()
  dialogMode.value = 'create'
  dialogVisible.value = true
}

function onEdit(row: PlatformAccount) {
  form.id = row.id
  form.platform = row.platform
  form.name = row.name
  form.cookies = row.cookies || ''
  form.remark = row.remark
  dialogMode.value = 'edit'
  dialogVisible.value = true
}

function onDelete(row: PlatformAccount) {
  confirm.require({
    message: `确认删除「${row.name || platformLabel(row.platform) + '账号'}」吗？`,
    header: '删除账号',
    icon: 'pi pi-exclamation-triangle',
    acceptLabel: '确定',
    rejectLabel: '取消',
    acceptClass: 'p-button-danger',
    accept: async () => {
      try {
        await invoke('delete_platform_account', { id: row.id })
        fetchList()
      } catch (e) {
        console.error('删除失败', e)
      }
    },
  })
}

async function onLogin(row: PlatformAccount) {
  loggingInId.value = row.id
  pendingRemark = ''
  errorMsg.value = ''
  try {
    await invoke<string>('open_download_login', {
      platform: row.platform,
      cookies: row.cookies || undefined,
    })
  } catch (e: any) {
    loggingInId.value = null
    const msg = typeof e === 'string' ? e : e?.message || '打开登录窗口失败'
    if (msg.includes('Chrome')) {
      errorMsg.value = msg
    } else {
      console.error('打开登录窗口失败', e)
    }
  }
}

async function submitForm() {
  formSubmitting.value = true
  errorMsg.value = ''
  try {
    if (dialogMode.value === 'create') {
      pendingRemark = form.remark.trim()
      dialogVisible.value = false
      await invoke<string>('open_download_login', {
        platform: form.platform,
      })
    } else {
      await invoke('update_platform_account', {
        id: form.id,
        updates: { remark: form.remark.trim() },
      })
      dialogVisible.value = false
      fetchList()
    }
  } catch (e: any) {
    const msg = typeof e === 'string' ? e : e?.message || '操作失败'
    if (msg.includes('Chrome')) {
      errorMsg.value = msg
    } else {
      console.error('操作失败', e)
    }
  } finally {
    formSubmitting.value = false
  }
}

function copyCookies(text: string) {
  window.navigator.clipboard.writeText(text)
}

onMounted(async () => {
  await fetchList()
  checkAllStatus()
  statusInterval = setInterval(checkAllStatus, 5 * 60 * 1000)

  unlistenLogin = await listen<{
    cookies: string; platform: string; name: string; avatar: string; label: string
  }>('download-login-success', async (event) => {
    const { cookies: partialCookies, platform, name, avatar, label: webviewLabel } = event.payload

    if (!partialCookies || !name) {
      loggingInId.value = null
      try { await invoke('close_download_webview', { label: webviewLabel }) } catch { /* */ }
      return
    }

    try {
      let finalCookies = partialCookies
      const url = platformUrl(platform)

      const cookieMap = new Map<string, string>()
      for (const seg of partialCookies.split('; ')) {
        const eq = seg.indexOf('=')
        if (eq > 0) cookieMap.set(seg.slice(0, eq).trim(), seg.slice(eq + 1).trim())
      }

      if (url) {
        const captureUrls = platform === 'kuaishou'
          ? ['https://live.kuaishou.com', 'https://www.kuaishou.com']
          : [url]

        for (const captureUrl of captureUrls) {
          for (let attempt = 1; attempt <= 3; attempt++) {
            try {
              const fullCookies = await invoke<string>('capture_download_cookies', {
                label: webviewLabel,
                url: captureUrl,
              })
              if (fullCookies) {
                for (const seg of fullCookies.split('; ')) {
                  const eq = seg.indexOf('=')
                  if (eq > 0) {
                    const k = seg.slice(0, eq).trim()
                    const v = seg.slice(eq + 1).trim()
                    if (k && v) cookieMap.set(k, v)
                  }
                }
                break
              }
            } catch {
              if (attempt < 3) await new Promise(r => setTimeout(r, 300))
            }
          }
        }

        finalCookies = Array.from(cookieMap.entries()).map(([k, v]) => `${k}=${v}`).join('; ')
      }

      const accountName = name || platformLabel(platform) + '账号'

      if (loggingInId.value) {
        await invoke('update_platform_account', {
          id: loggingInId.value,
          updates: {
            cookies: finalCookies,
            status: 'active',
            ...(name ? { name } : {}),
            ...(avatar ? { avatar } : {}),
          },
        })
      } else {
        await invoke('upsert_download_account', {
          platform,
          name: accountName,
          cookies: finalCookies,
          avatar: avatar || undefined,
          remark: pendingRemark || undefined,
        })
      }

      loggingInId.value = null
      pendingRemark = ''
      fetchList()
    } catch (e) {
      console.error('保存登录信息失败', e)
      loggingInId.value = null
    } finally {
      try { await invoke('close_download_webview', { label: webviewLabel }) } catch { /* */ }
    }
  })
})

onUnmounted(() => {
  unlistenLogin?.()
  if (statusInterval) clearInterval(statusInterval)
})
</script>

<template>
  <div class="am-page">
    <ConfirmDialog />

    <!-- 顶部 -->
    <section class="am-header-section">
      <div class="am-header-row">
        <div class="am-header-left">
          <span class="am-title">
            <i class="pi pi-users"></i>
            账号登记
          </span>
          <span class="am-subtitle">管理各平台登录账号，用于视频解析下载</span>
        </div>
        <div class="am-header-right">
          <Button label="新增账号" icon="pi pi-plus" size="small" @click="onAdd" />
        </div>
      </div>
    </section>

    <!-- Chrome 未安装提示 -->
    <div v-if="errorMsg" class="am-chrome-alert">
      <i class="pi pi-exclamation-circle"></i>
      <span>{{ errorMsg }}</span>
      <Button label="关闭" size="small" text severity="danger" @click="errorMsg = ''" />
    </div>

    <!-- 表格 -->
    <section class="am-table-section">
      <DataTable
        :value="accounts"
        :loading="loading"
        dataKey="id"
        scrollable
        scrollHeight="flex"
        :virtualScrollerOptions="{ itemSize: 56 }"
        class="am-table am-table-grid"
        size="small"
        stripedRows
      >
        <template #empty>
          <div class="am-empty">
            <i class="pi pi-inbox"></i>
            <p>暂无账号，点击「新增账号」添加</p>
          </div>
        </template>

        <Column header="#" headerStyle="width: 3.5rem">
          <template #body="{ index }">{{ index + 1 }}</template>
        </Column>
        <Column field="platform" header="平台" headerStyle="width: 5rem">
          <template #body="{ data }">
            <Tag :value="platformLabel(data.platform)" :class="'am-platform-tag am-platform--' + data.platform" />
          </template>
        </Column>
        <Column header="头像" headerStyle="width: 4rem">
          <template #body="{ data }">
            <img v-if="data.avatar" :src="data.avatar" class="am-avatar" alt="" referrerpolicy="no-referrer" />
            <div v-else class="am-avatar-placeholder"><i class="pi pi-user"></i></div>
          </template>
        </Column>
        <Column field="name" header="账号名称" style="min-width: 140px">
          <template #body="{ data }">
            <span class="am-name">{{ data.name || '未知' }}</span>
          </template>
        </Column>
        <Column field="status" header="在线状态" headerStyle="width: 6rem">
          <template #body="{ data }">
            <Tag
              v-if="loggingInId === data.id"
              value="登录中..."
              severity="info"
              class="am-status-tag"
            />
            <Tag
              v-else
              :value="statusText(data.status)"
              :severity="statusSeverity(data.status)"
              class="am-status-tag"
            />
          </template>
        </Column>
        <Column field="cookies" header="Cookies" style="min-width: 140px">
          <template #body="{ data }">
            <span class="am-cookies-text">{{ truncateCookies(data.cookies) }}</span>
          </template>
        </Column>
        <Column field="remark" header="备注" style="min-width: 100px" />
        <Column header="操作" headerStyle="width: 10rem" frozen alignFrozen="right">
          <template #body="{ data }">
            <div class="am-actions">
              <Button
                icon="pi pi-sign-in"
                size="small"
                text
                rounded
                severity="success"
                title="登录"
                :loading="loggingInId === data.id"
                @click="onLogin(data)"
              />
              <Button
                icon="pi pi-pencil"
                size="small"
                text
                rounded
                severity="info"
                title="编辑"
                @click="onEdit(data)"
              />
              <Button
                icon="pi pi-trash"
                size="small"
                text
                rounded
                severity="danger"
                title="删除"
                @click="onDelete(data)"
              />
            </div>
          </template>
        </Column>
      </DataTable>
    </section>

    <!-- 底部 -->
    <div class="am-footer">
      <span class="am-footer-info">共 {{ accounts.length }} 个账号</span>
    </div>

    <!-- 弹窗 -->
    <Dialog
      v-model:visible="dialogVisible"
      :header="dialogMode === 'create' ? '新增账号' : '编辑账号'"
      modal
      :closable="!formSubmitting"
      :style="{ width: '440px' }"
    >
      <div class="am-form">
        <div class="am-form-item">
          <label>平台</label>
          <Select
            v-model="form.platform"
            :options="PLATFORMS"
            optionLabel="label"
            optionValue="value"
            :disabled="dialogMode === 'edit'"
            class="am-form-select"
          />
        </div>

        <template v-if="dialogMode === 'edit'">
          <div class="am-form-item">
            <label>账号名称</label>
            <InputText :modelValue="form.name" disabled class="am-form-input" />
          </div>
          <div class="am-form-item">
            <label>Cookies</label>
            <div class="am-cookies-readonly">
              <span class="am-cookies-preview">{{ truncateCookies(form.cookies) }}</span>
              <Button icon="pi pi-copy" size="small" text rounded title="复制" @click="copyCookies(form.cookies)" />
            </div>
          </div>
        </template>

        <div class="am-form-item">
          <label>备注</label>
          <Textarea v-model="form.remark" placeholder="可选" :autoResize="true" rows="2" class="am-form-input" />
        </div>
      </div>

      <template #footer>
        <Button label="取消" severity="secondary" text :disabled="formSubmitting" @click="dialogVisible = false" />
        <Button
          :label="dialogMode === 'create' ? '登录并获取 Cookies' : '保存'"
          :icon="dialogMode === 'create' ? 'pi pi-sign-in' : 'pi pi-check'"
          :loading="formSubmitting"
          @click="submitForm"
        />
      </template>
    </Dialog>
  </div>
</template>

<style scoped>
.am-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  gap: 12px;
  overflow: hidden;
}

.am-header-section {
  flex-shrink: 0;
  background: #fff;
  border-radius: 14px;
  padding: 16px 18px;
  box-shadow: 0 2px 12px rgba(15, 23, 42, 0.06);
}

.am-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.am-header-left {
  display: flex;
  align-items: baseline;
  gap: 10px;
}

.am-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: #0f172a;
  display: flex;
  align-items: center;
  gap: 6px;
}

.am-title i { color: var(--app-primary, #22c55e); }

.am-subtitle {
  font-size: 0.78rem;
  color: #94a3b8;
}

.am-table-section {
  flex: 1;
  min-height: 0;
  background: #fff;
  border-radius: 14px;
  box-shadow: 0 2px 12px rgba(15, 23, 42, 0.06);
  overflow: hidden;
  display: flex;
}

.am-table { flex: 1; }
:deep(.p-datatable-wrapper) { flex: 1; }

.am-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 48px 0;
  color: #94a3b8;
}
.am-empty i { font-size: 2.5rem; margin-bottom: 10px; }
.am-empty p { font-size: 0.85rem; }

.am-platform-tag { font-size: 0.72rem; font-weight: 600; padding: 2px 8px; }
.am-platform--douyin { background: #0f0f0f !important; color: #fff !important; }
.am-platform--kuaishou { background: #ff4906 !important; color: #fff !important; }
.am-platform--bilibili { background: #fb7299 !important; color: #fff !important; }

.am-table-grid :deep(.p-datatable-thead > tr > th) { border: 1px solid #e2e8f0 !important; }
.am-table-grid :deep(.p-datatable-tbody > tr > td) { border: 1px solid #e2e8f0 !important; }

.am-avatar { width: 32px; height: 32px; border-radius: 50%; object-fit: cover; }
.am-avatar-placeholder {
  width: 32px; height: 32px; border-radius: 50%;
  background: #f1f5f9; display: flex; align-items: center; justify-content: center;
  color: #94a3b8; font-size: 0.85rem;
}

.am-name { font-size: 0.83rem; font-weight: 500; }
.am-status-tag { font-size: 0.7rem; }
.am-cookies-text { font-size: 0.78rem; color: #94a3b8; font-family: 'Consolas', monospace; }
.am-actions { display: flex; gap: 2px; }

.am-footer {
  flex-shrink: 0;
  padding: 6px 14px;
  background: #fff;
  border-radius: 14px;
  box-shadow: 0 2px 12px rgba(15, 23, 42, 0.06);
}
.am-footer-info { font-size: 0.8rem; color: #94a3b8; }

.am-chrome-alert {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background: #fef2f2;
  border: 1px solid #fecaca;
  border-radius: 10px;
  color: #991b1b;
  font-size: 0.82rem;
  line-height: 1.4;
}
.am-chrome-alert i { color: #dc2626; font-size: 1rem; flex-shrink: 0; }
.am-chrome-alert span { flex: 1; white-space: pre-line; }

.am-form { display: flex; flex-direction: column; gap: 16px; padding: 4px 0; }
.am-form-item { display: flex; flex-direction: column; gap: 6px; }
.am-form-item label { font-size: 0.82rem; font-weight: 500; color: #334155; }
.am-form-select, .am-form-input { width: 100%; }

.am-cookies-readonly {
  display: flex; align-items: center; gap: 6px;
  padding: 8px 10px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px;
}
.am-cookies-preview {
  flex: 1; font-size: 0.78rem; color: #64748b; font-family: 'Consolas', monospace;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
</style>
