<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch, nextTick } from 'vue'
import type { BrandConfig } from '../brand'
import {
  getBrand,
  setBrand,
  setActiveBrandId,
  getTemplate,
  resolveImageUrl,
  serverBrandToConfig,
} from '../brand'
import { useUserStore } from '../stores/user'
import {
  fetchBrandList, createBrand, updateBrand, deleteBrandApi, uploadImage,
  templateIdToLabel,
} from '../api/brand'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

const userStore = useUserStore()
const isDark = getTemplate() === 'dark'

const brands = ref<BrandConfig[]>([])
const editingBrand = ref<BrandConfig | null>(null)
const showEditor = ref(false)
const loading = ref(false)
const toastMsg = ref('')
const logoInputRef = ref<HTMLInputElement | null>(null)
const contactInputRef = ref<HTMLInputElement | null>(null)

const pendingLogoFile = ref<File | null>(null)
const pendingContactFiles = ref<File[]>([])
const pendingLogoPreview = ref('')
const pendingContactPreviews = ref<string[]>([])

const TEMPLATES = [
  { id: 'green', label: '经典（绿色）' },
  { id: 'orange', label: '华云（橙色）' },
  { id: 'dark', label: '科技（暗黑）' },
]

function toast(msg: string, duration = 2500) {
  toastMsg.value = msg
  setTimeout(() => toastMsg.value = '', duration)
}

const isEditing = computed(() => !!editingBrand.value && brands.value.some(b => b.id === editingBrand.value!.id))

/* ─── 加载品牌列表 ─── */

async function loadBrands() {
  loading.value = true
  try {
    const res = await fetchBrandList(userStore.token)
    brands.value = (res.items || []).map(serverBrandToConfig)
  } catch (err: unknown) {
    toast(err instanceof Error ? err.message : '加载品牌列表失败', 3000)
    brands.value = [getBrand()]
  } finally {
    loading.value = false
  }
}

/* ─── 切换激活品牌 ─── */

function activateBrand(b: BrandConfig) {
  if (getBrand().id === b.id) return
  const needReload = getBrand().template !== b.template
  setBrand(b)
  setActiveBrandId(b.id)
  toast('已切换到「' + b.brand_name + '」')
  if (needReload) {
    setTimeout(() => window.location.reload(), 500)
  } else {
    setTimeout(() => window.location.reload(), 500)
  }
}

/* ─── Logo 显示 ─── */

function getLogoSrc(b: BrandConfig): string {
  if (!b.logo) return '/app-icon.png'
  return resolveImageUrl(b.logo)
}

function getTemplateName(tpl: string): string {
  return TEMPLATES.find(t => t.id === tpl)?.label || tpl
}

function rewardPayload(b: BrandConfig) {
  return {
    reg_reward_type: b.reg_reward_type,
    reg_reward_value: b.reg_reward_value,
    invite_invitee_reward_type: b.invite_invitee_reward_type,
    invite_invitee_reward_value: b.invite_invitee_reward_value,
    invite_inviter_reward_type: b.invite_inviter_reward_type,
    invite_inviter_reward_value: b.invite_inviter_reward_value,
    checkin_reward_type: b.checkin_reward_type,
    checkin_reward_value: b.checkin_reward_value,
  }
}

function togglePayMethod(method: string) {
  if (!editingBrand.value) return
  const list = editingBrand.value.pay_methods
  const idx = list.indexOf(method)
  if (idx >= 0) list.splice(idx, 1)
  else list.push(method)
}

/* ─── 新增 / 编辑 ─── */

function addBrand() {
  editingBrand.value = {
    id: '',
    brand_name: '',
    product_name: '',
    template: 'green',
    logo: '',
    about: `© ${new Date().getFullYear()} `,
    website: '',
    tutorial_url: '',
    contact_images: [],
    data_version: 0,
    reg_reward_type: 'membership',
    reg_reward_value: 0,
    invite_invitee_reward_type: 'membership',
    invite_invitee_reward_value: 0,
    invite_inviter_reward_type: 'membership',
    invite_inviter_reward_value: 0,
    checkin_reward_type: 'membership',
    checkin_reward_value: 0,
    pay_channel: 'none',
    pay_methods: [],
  }
  resetPending()
  showEditor.value = true
}

function editBrand(b: BrandConfig) {
  editingBrand.value = { ...b, contact_images: [...(b.contact_images || [])], pay_methods: [...(b.pay_methods || [])] }
  resetPending()
  showEditor.value = true
}

function resetPending() {
  pendingLogoFile.value = null
  pendingContactFiles.value = []
  if (pendingLogoPreview.value) URL.revokeObjectURL(pendingLogoPreview.value)
  pendingLogoPreview.value = ''
  pendingContactPreviews.value.forEach(u => URL.revokeObjectURL(u))
  pendingContactPreviews.value = []
}

/* ─── 删除 ─── */

async function handleDelete(brandId: string) {
  if (brands.value.length <= 1) {
    toast('至少保留一个品牌')
    return
  }
  loading.value = true
  try {
    await deleteBrandApi(userStore.token, brandId)
    brands.value = brands.value.filter(b => b.id !== brandId)
    if (getBrand().id === brandId && brands.value.length > 0) {
      setBrand(brands.value[0])
      setActiveBrandId(brands.value[0].id)
      window.location.reload()
    }
    toast('删除成功')
  } catch (err: unknown) {
    toast(err instanceof Error ? err.message : '删除失败', 3000)
  } finally {
    loading.value = false
  }
}

/* ─── 保存（创建 / 更新） ─── */

async function saveCurrent() {
  if (!editingBrand.value) return
  const b = editingBrand.value
  if (!b.brand_name.trim()) {
    toast('请填写品牌名称')
    return
  }

  loading.value = true
  try {
    let brandId = b.id

    if (!isEditing.value) {
      const res = await createBrand(userStore.token, {
        name: b.brand_name,
        product_name: b.product_name,
        template: templateIdToLabel(b.template),
        about_info: b.about,
        website_url: b.website,
        tutorial_url: b.tutorial_url,
        pay_channel: b.pay_channel,
        pay_methods: b.pay_methods,
        ...rewardPayload(b),
      })
      brandId = res.brand_id
    }

    if (pendingLogoFile.value) {
      const res = await uploadImage(userStore.token, pendingLogoFile.value, 'logo', { brandId })
      b.logo = res.url
      pendingLogoFile.value = null
    }

    for (const file of pendingContactFiles.value) {
      const res = await uploadImage(userStore.token, file, 'contact', { brandId })
      b.contact_images.push(res.url)
    }
    pendingContactFiles.value = []
    pendingContactPreviews.value.forEach(u => URL.revokeObjectURL(u))
    pendingContactPreviews.value = []

    if (isEditing.value) {
      await updateBrand(userStore.token, brandId, {
        name: b.brand_name,
        product_name: b.product_name,
        template: templateIdToLabel(b.template),
        about_info: b.about,
        logo: b.logo,
        website_url: b.website,
        tutorial_url: b.tutorial_url,
        contact_images: b.contact_images,
        pay_channel: b.pay_channel,
        pay_methods: b.pay_methods,
        ...rewardPayload(b),
      })
    } else if (b.logo || b.contact_images.length) {
      await updateBrand(userStore.token, brandId, {
        logo: b.logo,
        contact_images: b.contact_images,
        pay_channel: b.pay_channel,
        pay_methods: b.pay_methods,
        ...rewardPayload(b),
      })
    }

    b.id = brandId
    setBrand(b)
    setActiveBrandId(brandId)

    const idx = brands.value.findIndex(x => x.id === brandId)
    if (idx >= 0) {
      brands.value[idx] = { ...b }
    } else {
      brands.value.push({ ...b })
    }

    showEditor.value = false

    toast('保存成功')
    setTimeout(() => window.location.reload(), 800)
  } catch (err: unknown) {
    toast(err instanceof Error ? err.message : '保存失败', 3000)
  } finally {
    loading.value = false
  }
}

/* ─── Logo 上传 ─── */

function onLogoSelected(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file || !editingBrand.value) return
  const validTypes = ['image/jpeg', 'image/png', 'image/webp', 'image/x-icon', 'image/vnd.microsoft.icon']
  if (!validTypes.includes(file.type)) {
    toast('仅支持 jpg/png/webp/ico 格式')
    return
  }
  if (file.size > 5 * 1024 * 1024) {
    toast('Logo 大小不能超过 5MB')
    return
  }
  pendingLogoFile.value = file
  if (pendingLogoPreview.value) URL.revokeObjectURL(pendingLogoPreview.value)
  pendingLogoPreview.value = URL.createObjectURL(file)
}

function removeLogo() {
  if (!editingBrand.value) return
  editingBrand.value.logo = ''
  pendingLogoFile.value = null
  if (pendingLogoPreview.value) URL.revokeObjectURL(pendingLogoPreview.value)
  pendingLogoPreview.value = ''
}

function currentLogoSrc(): string {
  if (pendingLogoPreview.value) return pendingLogoPreview.value
  if (editingBrand.value?.logo) return resolveImageUrl(editingBrand.value.logo)
  return ''
}

/* ─── 联系方式上传 ─── */

function onContactSelected(e: Event) {
  const input = e.target as HTMLInputElement
  const fileArray = Array.from(input.files || [])
  input.value = ''
  if (!fileArray.length || !editingBrand.value) return

  const validTypes = ['image/jpeg', 'image/png', 'image/webp', 'image/gif']
  const existingCount = editingBrand.value.contact_images.length + pendingContactFiles.value.length
  for (const file of fileArray) {
    if (existingCount + pendingContactFiles.value.length >= 5) {
      toast('最多上传 5 张联系方式图片')
      break
    }
    if (!validTypes.includes(file.type)) {
      toast(`不支持的格式：${file.name}，仅支持 jpg/png/webp/gif`, 3000)
      continue
    }
    if (file.size > 10 * 1024 * 1024) {
      toast(`图片太大：${file.name}，最大 10MB`, 3000)
      continue
    }
    pendingContactFiles.value.push(file)
    pendingContactPreviews.value.push(URL.createObjectURL(file))
  }
}

function removeExistingContact(idx: number) {
  editingBrand.value?.contact_images.splice(idx, 1)
}

function removePendingContact(idx: number) {
  URL.revokeObjectURL(pendingContactPreviews.value[idx])
  pendingContactFiles.value.splice(idx, 1)
  pendingContactPreviews.value.splice(idx, 1)
}

const totalContactCount = computed(() =>
  (editingBrand.value?.contact_images.length || 0) + pendingContactFiles.value.length,
)

/* ─── 打包构建 ─── */

const showBuild = ref(false)
const buildStatus = ref<'idle' | 'building' | 'success' | 'error'>('idle')
const buildLogs = ref('')
const buildOutputPath = ref('')
const buildLogRef = ref<HTMLPreElement | null>(null)
let unlistenLog: UnlistenFn | null = null
let unlistenStatus: UnlistenFn | null = null
let unlistenComplete: UnlistenFn | null = null

watch(buildLogs, () => {
  nextTick(() => {
    if (buildLogRef.value) {
      buildLogRef.value.scrollTop = buildLogRef.value.scrollHeight
    }
  })
})

async function setupBuildListeners() {
  unlistenLog = await listen<string>('build-log', (event) => {
    buildLogs.value += event.payload + '\n'
  })

  unlistenStatus = await listen<string>('build-status', () => {
    buildStatus.value = 'building'
  })

  unlistenComplete = await listen<{ success: boolean; output_path?: string; error?: string }>('build-complete', (event) => {
    if (event.payload.success) {
      buildStatus.value = 'success'
      buildOutputPath.value = event.payload.output_path || ''
      buildLogs.value += '\n✅ 构建成功!\n'
      if (event.payload.output_path) {
        buildLogs.value += `输出路径: ${event.payload.output_path}\n`
      }
    } else {
      buildStatus.value = 'error'
      buildLogs.value += `\n❌ 构建失败: ${event.payload.error}\n`
    }
  })
}

function cleanupBuildListeners() {
  unlistenLog?.()
  unlistenStatus?.()
  unlistenComplete?.()
}

async function startBuild(b: BrandConfig) {
  if (!b.id) {
    toast('请先保存品牌设置')
    return
  }

  function resizeToIcon(imgSrc: string): Promise<string> {
    const SIZE = 256
    return new Promise((resolve, reject) => {
      const img = new Image()
      img.crossOrigin = 'anonymous'
      img.onload = () => {
        const canvas = document.createElement('canvas')
        canvas.width = SIZE
        canvas.height = SIZE
        const ctx = canvas.getContext('2d')!
        ctx.imageSmoothingQuality = 'high'
        ctx.drawImage(img, 0, 0, SIZE, SIZE)
        resolve(canvas.toDataURL('image/png'))
      }
      img.onerror = () => reject(new Error('Image load failed'))
      img.src = imgSrc
    })
  }

  async function downloadImageAsBase64(url: string): Promise<string> {
    // 方式1: Tauri HTTP 插件下载 → Canvas 缩放到 256×256
    try {
      const { fetch: tauriFetch } = await import('@tauri-apps/plugin-http')
      const resp = await tauriFetch(url)
      if (resp.ok) {
        const blob = await resp.blob()
        const blobUrl = URL.createObjectURL(blob)
        try {
          return await resizeToIcon(blobUrl)
        } finally {
          URL.revokeObjectURL(blobUrl)
        }
      }
      buildLogs.value += `  Tauri fetch 状态: ${resp.status}\n`
    } catch (err) {
      buildLogs.value += `  Tauri fetch 失败: ${err instanceof Error ? err.message : err}\n`
    }

    // 方式2: 直接用 URL 通过 Canvas 加载并缩放
    try {
      return await resizeToIcon(url)
    } catch (err) {
      buildLogs.value += `  Canvas 转换失败: ${err instanceof Error ? err.message : err}\n`
    }

    return ''
  }

  if (getBrand().id !== b.id) {
    setBrand(b)
    setActiveBrandId(b.id)
  }

  const displayName = b.product_name || b.brand_name
  buildLogs.value = `[${new Date().toLocaleTimeString()}] 开始打包「${displayName}」…\n`
  buildOutputPath.value = ''
  buildStatus.value = 'building'
  showBuild.value = true

  try {
    const { getBrandLogo, resolveImageUrl } = await import('../brand')
    let logoData = getBrandLogo()

    if (!logoData.startsWith('data:')) {
      const logoUrl = b.logo ? resolveImageUrl(b.logo) : ''
      if (logoUrl && logoUrl.startsWith('http')) {
        buildLogs.value += `正在下载品牌图标: ${logoUrl}\n`
        logoData = await downloadImageAsBase64(logoUrl)
        if (!logoData.startsWith('data:')) {
          buildLogs.value += `⚠ 图标下载失败，将使用默认图标\n`
        }
      }
    }

    await invoke('start_brand_build', {
      brandName: b.brand_name,
      productName: b.product_name || b.brand_name,
      logoData: logoData.startsWith('data:') ? logoData : '',
    })
  } catch (e: unknown) {
    buildStatus.value = 'error'
    buildLogs.value += `\n❌ 错误: ${e}\n`
  }
}

function closeBuild() {
  if (buildStatus.value === 'building') return
  showBuild.value = false
}

onMounted(async () => {
  await loadBrands()
  await setupBuildListeners()
  const running = await invoke<boolean>('is_build_running')
  if (running) {
    buildStatus.value = 'building'
    showBuild.value = true
  }
})

onUnmounted(() => {
  cleanupBuildListeners()
})
</script>

<template>
  <div class="bm-page" :class="{ 'bm-dark': isDark }">
    <div class="bm-header">
      <h1>品牌配置管理</h1>
      <span class="bm-badge">仅开发环境可见</span>
      <div class="bm-header-actions">
        <button class="bm-btn bm-btn--primary" @click="addBrand" :disabled="loading">
          <i class="pi pi-plus"></i> 新增品牌
        </button>
      </div>
    </div>

    <Transition name="toast">
      <div v-if="toastMsg" class="bm-toast">{{ toastMsg }}</div>
    </Transition>

    <div v-if="loading && !brands.length" class="bm-loading">加载中…</div>

    <div class="bm-grid">
      <div
        v-for="b in brands" :key="b.id"
        class="bm-card"
        :class="{ 'bm-card--active': getBrand().id === b.id }"
      >
        <div class="bm-card-top">
          <img :src="getLogoSrc(b)" class="bm-card-logo" alt="" />
          <div class="bm-card-info">
            <strong>{{ b.brand_name || '未命名' }}</strong>
            <span class="bm-card-product">{{ b.product_name }}</span>
          </div>
          <span class="bm-card-tpl">{{ getTemplateName(b.template) }}</span>
        </div>
        <div class="bm-card-meta">
          <span v-if="b.website"><i class="pi pi-globe"></i> {{ b.website }}</span>
          <span v-if="b.about"><i class="pi pi-info-circle"></i> {{ b.about }}</span>
        </div>
        <div class="bm-card-actions">
          <button v-if="getBrand().id !== b.id" class="bm-btn bm-btn--sm bm-btn--primary" @click="activateBrand(b)" :disabled="loading"><i class="pi pi-check"></i> 应用</button>
          <button class="bm-btn bm-btn--sm" @click="editBrand(b)" :disabled="loading"><i class="pi pi-pencil"></i> 编辑</button>
          <button class="bm-btn bm-btn--sm bm-btn--outline" @click="startBuild(b)" :disabled="loading || buildStatus === 'building'"><i class="pi pi-box"></i> 打包</button>
          <button class="bm-btn bm-btn--sm bm-btn--danger" @click="handleDelete(b.id)" :disabled="loading"><i class="pi pi-trash"></i> 删除</button>
        </div>
        <div v-if="getBrand().id === b.id" class="bm-card-active-tag">当前</div>
      </div>
    </div>

    <!-- Editor Modal -->
    <Transition name="modal">
      <div v-if="showEditor && editingBrand" class="bm-overlay" @click.self="showEditor = false">
        <div class="bm-editor">
          <div class="bm-editor-header">
            <h3>{{ isEditing ? '编辑品牌' : '新增品牌' }}</h3>
            <button class="bm-editor-close" @click="showEditor = false"><i class="pi pi-times"></i></button>
          </div>

          <div class="bm-editor-body">
            <div v-if="editingBrand.id" class="bm-field">
              <label>品牌 ID</label>
              <div class="bm-readonly">{{ editingBrand.id }}</div>
            </div>

            <div class="bm-row">
              <div class="bm-field">
                <label>品牌名称 <em>*</em></label>
                <input v-model="editingBrand.brand_name" placeholder="如 青拾" />
              </div>
              <div class="bm-field">
                <label>产品名</label>
                <input v-model="editingBrand.product_name" placeholder="如 视频下载" />
              </div>
            </div>

            <div class="bm-row">
              <div class="bm-field">
                <label>模板</label>
                <select v-model="editingBrand.template">
                  <option v-for="t in TEMPLATES" :key="t.id" :value="t.id">{{ t.label }}</option>
                </select>
              </div>
              <div class="bm-field">
                <label>关于信息</label>
                <input v-model="editingBrand.about" placeholder="© 2024 品牌名" />
              </div>
            </div>

            <div class="bm-field">
              <label>Logo</label>
              <div class="bm-logo-area">
                <div v-if="currentLogoSrc()" class="bm-logo-preview">
                  <img :src="currentLogoSrc()" alt="" />
                  <button class="bm-logo-del" @click="removeLogo" title="删除"><i class="pi pi-times"></i></button>
                </div>
                <button class="bm-upload-btn" @click="logoInputRef?.click()">
                  <i class="pi pi-image"></i>
                  <span>{{ currentLogoSrc() ? '更换' : '上传 Logo' }}</span>
                </button>
                <input ref="logoInputRef" type="file" accept="image/jpeg,image/png,image/webp,image/x-icon" style="display:none" @change="onLogoSelected" />
              </div>
            </div>

            <div class="bm-row">
              <div class="bm-field">
                <label>官网地址</label>
                <input v-model="editingBrand.website" placeholder="https://example.com" />
              </div>
              <div class="bm-field">
                <label>教程地址</label>
                <input v-model="editingBrand.tutorial_url" placeholder="https://docs.example.com" />
              </div>
            </div>

            <div class="bm-reward-section">
              <div class="bm-reward-title">支付控制</div>
              <div class="bm-field" style="margin-bottom:8px">
                <label>支付通道</label>
                <div class="bm-pay-methods">
                  <label class="bm-pay-check">
                    <input type="radio" v-model="editingBrand.pay_channel" value="none" />
                    <span>不开通</span>
                  </label>
                  <label class="bm-pay-check">
                    <input type="radio" v-model="editingBrand.pay_channel" value="enterprise" />
                    <span>企业支付（自有商户）</span>
                  </label>
                  <label class="bm-pay-check">
                    <input type="radio" v-model="editingBrand.pay_channel" value="hupijiao" />
                    <span>虎皮椒（个人）</span>
                  </label>
                </div>
              </div>
              <div v-if="editingBrand.pay_channel !== 'none'" class="bm-field">
                <label>开通方式</label>
                <div class="bm-pay-methods">
                  <label class="bm-pay-check">
                    <input type="checkbox" :checked="editingBrand.pay_methods.includes('wechat')" @change="togglePayMethod('wechat')" />
                    <span>微信支付</span>
                  </label>
                  <label class="bm-pay-check">
                    <input type="checkbox" :checked="editingBrand.pay_methods.includes('alipay')" @change="togglePayMethod('alipay')" />
                    <span>支付宝支付</span>
                  </label>
                </div>
              </div>
            </div>

            <div class="bm-reward-section">
              <div class="bm-reward-title">奖励控制</div>
              <p class="bm-reward-hint">
                会员奖励单位为秒（86400 秒 = 1 天），积分奖励填写数值。设为 0 表示不发放该项奖励。
              </p>
              <div class="bm-reward-row">
                <span class="bm-reward-label">注册奖励</span>
                <select v-model="editingBrand.reg_reward_type" class="bm-reward-select">
                  <option value="membership">会员</option>
                  <option value="points">积分</option>
                </select>
                <div class="bm-reward-value-cell">
                  <input v-model.number="editingBrand.reg_reward_value" type="number" min="0" step="1" class="bm-reward-num" />
                  <span class="bm-reward-unit">{{ editingBrand.reg_reward_type === 'membership' ? '秒' : '积分' }}</span>
                </div>
              </div>
              <div class="bm-reward-row">
                <span class="bm-reward-label">邀请奖励（受邀者）</span>
                <select v-model="editingBrand.invite_invitee_reward_type" class="bm-reward-select">
                  <option value="membership">会员</option>
                  <option value="points">积分</option>
                </select>
                <div class="bm-reward-value-cell">
                  <input v-model.number="editingBrand.invite_invitee_reward_value" type="number" min="0" step="1" class="bm-reward-num" />
                  <span class="bm-reward-unit">{{ editingBrand.invite_invitee_reward_type === 'membership' ? '秒' : '积分' }}</span>
                </div>
              </div>
              <div class="bm-reward-row">
                <span class="bm-reward-label">邀请奖励（邀请者）</span>
                <select v-model="editingBrand.invite_inviter_reward_type" class="bm-reward-select">
                  <option value="membership">会员</option>
                  <option value="points">积分</option>
                </select>
                <div class="bm-reward-value-cell">
                  <input v-model.number="editingBrand.invite_inviter_reward_value" type="number" min="0" step="1" class="bm-reward-num" />
                  <span class="bm-reward-unit">{{ editingBrand.invite_inviter_reward_type === 'membership' ? '秒' : '积分' }}</span>
                </div>
              </div>
              <div class="bm-reward-row">
                <span class="bm-reward-label">签到奖励</span>
                <select v-model="editingBrand.checkin_reward_type" class="bm-reward-select">
                  <option value="membership">会员</option>
                  <option value="points">积分</option>
                </select>
                <div class="bm-reward-value-cell">
                  <input v-model.number="editingBrand.checkin_reward_value" type="number" min="0" step="1" class="bm-reward-num" />
                  <span class="bm-reward-unit">{{ editingBrand.checkin_reward_type === 'membership' ? '秒' : '积分' }}</span>
                </div>
              </div>
            </div>

            <div class="bm-field">
              <label>联系方式（二维码等，最多 5 张）</label>
              <div class="bm-contacts">
                <!-- 已上传到服务器的图片 -->
                <div v-for="(img, idx) in editingBrand.contact_images" :key="'s-' + idx" class="bm-contact-item">
                  <img :src="resolveImageUrl(img)" alt="" />
                  <button class="bm-contact-del" @click="removeExistingContact(idx)"><i class="pi pi-times"></i></button>
                </div>
                <!-- 待上传的新图片 -->
                <div v-for="(preview, idx) in pendingContactPreviews" :key="'p-' + idx" class="bm-contact-item bm-contact-pending">
                  <img :src="preview" alt="" />
                  <button class="bm-contact-del" @click="removePendingContact(idx)"><i class="pi pi-times"></i></button>
                </div>
                <button v-if="totalContactCount < 5" class="bm-contact-add" @click="contactInputRef?.click()">
                  <i class="pi pi-plus"></i>
                </button>
                <input ref="contactInputRef" type="file" accept="image/jpeg,image/png,image/webp,image/gif" multiple style="display:none" @change="onContactSelected" />
              </div>
            </div>
          </div>

          <div class="bm-editor-footer">
            <button class="bm-btn bm-btn--outline" @click="showEditor = false" :disabled="loading">取消</button>
            <button class="bm-btn bm-btn--primary" @click="saveCurrent" :disabled="loading">
              {{ loading ? '保存中…' : '保存并应用' }}
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <!-- Build Dialog -->
    <Transition name="modal">
      <div v-if="showBuild" class="bm-overlay" @click.self="closeBuild">
        <div class="bm-build-dialog">
          <div class="bm-build-header">
            <h3>打包品牌包</h3>
            <span v-if="buildStatus === 'building'" class="bm-build-badge bm-build-badge--building">
              <i class="pi pi-spin pi-spinner"></i> 构建中…
            </span>
            <span v-else-if="buildStatus === 'success'" class="bm-build-badge bm-build-badge--success">
              <i class="pi pi-check-circle"></i> 构建成功
            </span>
            <span v-else-if="buildStatus === 'error'" class="bm-build-badge bm-build-badge--error">
              <i class="pi pi-times-circle"></i> 构建失败
            </span>
            <button class="bm-editor-close" @click="closeBuild" :disabled="buildStatus === 'building'">
              <i class="pi pi-times"></i>
            </button>
          </div>

          <div v-if="buildStatus === 'building'" class="bm-build-progress">
            <div class="bm-progress-track"><div class="bm-progress-bar"></div></div>
          </div>

          <div v-if="buildOutputPath" class="bm-build-output">
            <i class="pi pi-folder-open"></i>
            <span>{{ buildOutputPath }}</span>
          </div>

          <div class="bm-build-log-wrap">
            <div class="bm-build-log-header">
              <span>构建日志</span>
              <button class="bm-btn bm-btn--sm" @click="buildLogs = ''">清空</button>
            </div>
            <pre ref="buildLogRef" class="bm-build-log">{{ buildLogs }}</pre>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.bm-page {
  padding: 20px;
  max-width: 100%;
  font-family: system-ui, -apple-system, sans-serif;
}

.bm-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 18px;
  flex-wrap: wrap;
}

.bm-header h1 { margin: 0; font-size: 1.2rem; color: #0f172a; }
.bm-badge { font-size: 0.65rem; padding: 2px 8px; background: #fef3c7; color: #92400e; border-radius: 4px; font-weight: 600; }
.bm-header-actions { margin-left: auto; display: flex; gap: 8px; }

.bm-loading { text-align: center; padding: 40px 0; color: #94a3b8; font-size: 0.9rem; }

/* Buttons */
.bm-btn {
  display: inline-flex; align-items: center; gap: 5px;
  padding: 7px 14px; border: 1.5px solid #e2e8f0; border-radius: 8px;
  background: #fff; color: #334155; font-size: 0.82rem; font-weight: 500;
  cursor: pointer; transition: all 0.15s; font-family: inherit;
}
.bm-btn:hover { border-color: #94a3b8; }
.bm-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.bm-btn--primary { background: var(--app-primary); color: #fff; border-color: var(--app-primary); }
.bm-btn--primary:hover { background: var(--app-primary-hover); border-color: var(--app-primary-hover); }
.bm-btn--danger { color: #ef4444; border-color: #fca5a5; }
.bm-btn--danger:hover { background: #fef2f2; border-color: #ef4444; }
.bm-btn--outline { border-color: #cbd5e1; }
.bm-btn--outline:hover { border-color: #64748b; background: #f8fafc; }
.bm-btn--sm { padding: 4px 10px; font-size: 0.75rem; border-radius: 6px; }
.bm-btn i { font-size: 0.82rem; }

/* Toast */
.bm-toast {
  position: fixed; top: 20px; left: 50%; transform: translateX(-50%);
  padding: 10px 20px; background: #0f172a; color: #fff; border-radius: 8px;
  font-size: 0.82rem; z-index: 2000; box-shadow: 0 4px 16px rgba(0,0,0,0.2);
}
.toast-enter-active, .toast-leave-active { transition: all 0.25s ease; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translateX(-50%) translateY(-10px); }

/* Card Grid */
.bm-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 14px;
}

.bm-card {
  background: #fff;
  border: 1.5px solid #e2e8f0;
  border-radius: 12px;
  padding: 14px;
  position: relative;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.bm-card:hover { border-color: #cbd5e1; box-shadow: 0 2px 8px rgba(0,0,0,0.04); }

.bm-card--active {
  border-color: var(--app-primary);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--app-primary) 12%, transparent);
}

.bm-card-active-tag {
  position: absolute; top: 8px; right: 8px;
  font-size: 0.6rem; padding: 1px 6px; background: var(--app-primary);
  color: #fff; border-radius: 4px; font-weight: 600;
}

.bm-card-top {
  display: flex; align-items: center; gap: 10px; margin-bottom: 8px;
}

.bm-card-logo {
  width: 36px; height: 36px; border-radius: 8px;
  object-fit: contain; background: #f8fafc; border: 1px solid #f1f5f9;
  flex-shrink: 0;
}

.bm-card-info {
  flex: 1; min-width: 0;
  display: flex; flex-direction: column; gap: 1px;
}
.bm-card-info strong { font-size: 0.88rem; color: #0f172a; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.bm-card-product { font-size: 0.72rem; color: #94a3b8; }

.bm-card-tpl {
  font-size: 0.62rem; padding: 2px 6px; background: #f1f5f9;
  border-radius: 4px; color: #64748b; white-space: nowrap; flex-shrink: 0;
}

.bm-card-meta {
  display: flex; flex-direction: column; gap: 2px;
  font-size: 0.72rem; color: #94a3b8; margin-bottom: 10px;
  min-height: 18px;
}
.bm-card-meta span { display: flex; align-items: center; gap: 4px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.bm-card-meta i { font-size: 0.7rem; }

.bm-card-actions { display: flex; gap: 6px; }

/* Editor Modal */
.bm-overlay {
  position: fixed; inset: 0; background: rgba(0,0,0,0.45);
  display: flex; align-items: center; justify-content: center; z-index: 1000;
}

.bm-editor {
  background: #fff; border-radius: 16px; width: 520px;
  max-height: 85vh; display: flex; flex-direction: column;
  box-shadow: 0 20px 60px rgba(0,0,0,0.15);
}

.bm-editor-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 18px 22px 14px; border-bottom: 1px solid #f1f5f9;
}
.bm-editor-header h3 { margin: 0; font-size: 1rem; }
.bm-editor-close {
  width: 28px; height: 28px; border: none; background: #f1f5f9;
  border-radius: 6px; cursor: pointer; display: flex; align-items: center; justify-content: center;
  color: #64748b; transition: all 0.15s;
}
.bm-editor-close:hover { background: #e2e8f0; color: #0f172a; }

.bm-editor-body {
  flex: 1; overflow-y: auto; padding: 18px 22px;
  display: flex; flex-direction: column; gap: 14px;
}

.bm-editor-footer {
  padding: 14px 22px; border-top: 1px solid #f1f5f9;
  display: flex; justify-content: flex-end; gap: 8px;
}

.bm-field { display: flex; flex-direction: column; gap: 4px; flex: 1; }
.bm-field label { font-size: 0.78rem; font-weight: 600; color: #475569; }
.bm-field label em { color: #ef4444; font-style: normal; }
.bm-field input, .bm-field select {
  height: 34px; border: 1.5px solid #e2e8f0; border-radius: 8px;
  padding: 0 10px; font-size: 0.82rem; font-family: inherit;
  transition: border-color 0.15s; background: #fff;
}
.bm-field input:focus, .bm-field select:focus { outline: none; border-color: var(--app-primary); }
.bm-readonly { font-size: 0.78rem; color: #94a3b8; padding: 6px 0; font-family: monospace; }

.bm-row { display: flex; gap: 12px; }

.bm-reward-section {
  padding: 12px 14px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.bm-reward-title {
  font-size: 0.8rem;
  font-weight: 700;
  color: #334155;
}
.bm-pay-methods {
  display: flex;
  gap: 20px;
}
.bm-pay-check {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.78rem;
  color: #475569;
  cursor: pointer;
}
.bm-pay-check input { cursor: pointer; }
.bm-reward-hint {
  margin: 0;
  font-size: 0.72rem;
  line-height: 1.45;
  color: #64748b;
}
.bm-reward-row {
  display: grid;
  grid-template-columns: 1fr 100px 1fr;
  align-items: center;
  gap: 10px;
}
.bm-reward-label {
  font-size: 0.76rem;
  font-weight: 600;
  color: #475569;
}
.bm-reward-select {
  height: 34px;
  border: 1.5px solid #e2e8f0;
  border-radius: 8px;
  padding: 0 8px;
  font-size: 0.8rem;
  font-family: inherit;
  background: #fff;
}
.bm-reward-value-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.bm-reward-num {
  flex: 1;
  min-width: 0;
  height: 34px;
  border: 1.5px solid #e2e8f0;
  border-radius: 8px;
  padding: 0 10px;
  font-size: 0.82rem;
  font-family: inherit;
}
.bm-reward-num:focus { outline: none; border-color: var(--app-primary); }
.bm-reward-unit {
  font-size: 0.72rem;
  color: #94a3b8;
  white-space: nowrap;
  flex-shrink: 0;
}

/* Logo Upload */
.bm-logo-area { display: flex; align-items: center; gap: 12px; }

.bm-logo-preview {
  width: 56px; height: 56px; border-radius: 10px; border: 1.5px solid #e2e8f0;
  position: relative; overflow: hidden; flex-shrink: 0;
}
.bm-logo-preview img { width: 100%; height: 100%; object-fit: contain; }
.bm-logo-del {
  position: absolute; top: 2px; right: 2px;
  width: 18px; height: 18px; border: none; border-radius: 50%;
  background: rgba(0,0,0,0.5); color: #fff; font-size: 0.55rem;
  cursor: pointer; display: flex; align-items: center; justify-content: center;
}

.bm-upload-btn {
  display: flex; align-items: center; gap: 6px;
  padding: 8px 14px; border: 1.5px dashed #cbd5e1; border-radius: 8px;
  background: #f8fafc; color: #64748b; font-size: 0.78rem;
  cursor: pointer; transition: all 0.15s; font-family: inherit;
}
.bm-upload-btn:hover { border-color: var(--app-primary); color: var(--app-primary); background: var(--app-primary-light); }

/* Contact Images */
.bm-contacts {
  display: flex; flex-wrap: wrap; gap: 8px;
}

.bm-contact-item {
  width: 72px; height: 72px; border-radius: 8px; border: 1.5px solid #e2e8f0;
  position: relative; overflow: hidden;
}
.bm-contact-item img { width: 100%; height: 100%; object-fit: cover; }
.bm-contact-pending { border-style: dashed; border-color: #93c5fd; }
.bm-contact-del {
  position: absolute; top: 2px; right: 2px;
  width: 18px; height: 18px; border: none; border-radius: 50%;
  background: rgba(0,0,0,0.5); color: #fff; font-size: 0.55rem;
  cursor: pointer; display: flex; align-items: center; justify-content: center;
}

.bm-contact-add {
  width: 72px; height: 72px; border: 1.5px dashed #cbd5e1; border-radius: 8px;
  background: #f8fafc; color: #94a3b8; font-size: 1.2rem;
  cursor: pointer; display: flex; align-items: center; justify-content: center;
  transition: all 0.15s;
}
.bm-contact-add:hover { border-color: var(--app-primary); color: var(--app-primary); background: var(--app-primary-light); }

/* Modal Transition */
.modal-enter-active, .modal-leave-active { transition: opacity 0.2s ease; }
.modal-enter-from, .modal-leave-to { opacity: 0; }
.modal-enter-active .bm-editor { transition: transform 0.25s ease; }
.modal-enter-from .bm-editor { transform: scale(0.95); }

/* ═══ Dark Mode ═══ */
.bm-dark { color: #E2E8F0; }
.bm-dark .bm-header h1 { color: #F1F5F9; }
.bm-dark .bm-badge { background: #1E293B; color: #94A3B8; }
.bm-dark .bm-loading { color: #64748B; }
.bm-dark .bm-toast { background: #1E293B; }

.bm-dark .bm-btn { background: #1E293B; color: #CBD5E1; border-color: #334155; }
.bm-dark .bm-btn:hover { border-color: #475569; background: #283548; }
.bm-dark .bm-btn--outline { border-color: #334155; }
.bm-dark .bm-btn--outline:hover { border-color: #64748B; background: #1E293B; }
.bm-dark .bm-btn--danger { color: #f87171; border-color: #7f1d1d; }
.bm-dark .bm-btn--danger:hover { background: rgba(239, 68, 68, 0.1); border-color: #f87171; }

.bm-dark .bm-card { background: #1E293B; border-color: #334155; }
.bm-dark .bm-card:hover { border-color: #475569; box-shadow: 0 2px 8px rgba(0,0,0,0.2); }
.bm-dark .bm-card--active { border-color: var(--app-primary); box-shadow: 0 0 0 2px color-mix(in srgb, var(--app-primary) 20%, transparent); }
.bm-dark .bm-card-info strong { color: #F1F5F9; }
.bm-dark .bm-card-product { color: #64748B; }
.bm-dark .bm-card-tpl { background: #0F172A; color: #94A3B8; }
.bm-dark .bm-card-meta { color: #64748B; }

.bm-dark .bm-editor { background: #1E293B; }
.bm-dark .bm-editor-header { border-bottom-color: #334155; }
.bm-dark .bm-editor-header h3 { color: #F1F5F9; }
.bm-dark .bm-editor-close { background: #334155; color: #94A3B8; }
.bm-dark .bm-editor-close:hover { background: #475569; color: #F1F5F9; }
.bm-dark .bm-editor-footer { border-top-color: #334155; }

.bm-dark .bm-field label { color: #94A3B8; }
.bm-dark .bm-field input,
.bm-dark .bm-field select { background: #0F172A; border-color: #334155; color: #E2E8F0; }
.bm-dark .bm-field input:focus,
.bm-dark .bm-field select:focus { border-color: var(--app-primary); }
.bm-dark .bm-readonly { color: #64748B; }

.bm-dark .bm-reward-section { background: #0F172A; border-color: #334155; }
.bm-dark .bm-reward-title { color: #E2E8F0; }
.bm-dark .bm-reward-hint { color: #64748B; }
.bm-dark .bm-reward-label { color: #94A3B8; }
.bm-dark .bm-reward-select,
.bm-dark .bm-reward-num { background: #0F172A; border-color: #334155; color: #E2E8F0; }
.bm-dark .bm-reward-unit { color: #64748B; }

.bm-dark .bm-logo-preview { border-color: #334155; }
.bm-dark .bm-upload-btn { background: #0F172A; border-color: #334155; color: #94A3B8; }
.bm-dark .bm-upload-btn:hover { border-color: var(--app-primary); color: var(--app-primary); background: var(--app-primary-light); }

.bm-dark .bm-contact-item { border-color: #334155; }
.bm-dark .bm-contact-add { background: #0F172A; border-color: #334155; color: #64748B; }
.bm-dark .bm-contact-add:hover { border-color: var(--app-primary); color: var(--app-primary); background: var(--app-primary-light); }

/* ═══ Build Dialog ═══ */
.bm-build-dialog {
  background: #fff;
  border-radius: 16px;
  width: 680px;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0,0,0,0.18);
  overflow: hidden;
}

.bm-build-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 18px 22px 14px;
  border-bottom: 1px solid #f1f5f9;
}

.bm-build-header h3 { margin: 0; font-size: 1rem; flex-shrink: 0; }

.bm-build-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 0.75rem;
  font-weight: 600;
  padding: 3px 10px;
  border-radius: 20px;
}

.bm-build-badge--building { background: #EFF6FF; color: #2563EB; }
.bm-build-badge--success { background: #F0FDF4; color: #16A34A; }
.bm-build-badge--error { background: #FEF2F2; color: #DC2626; }

.bm-build-header .bm-editor-close { margin-left: auto; }

.bm-build-progress {
  padding: 0 22px;
  margin: 12px 0 0;
}

.bm-progress-track {
  width: 100%;
  height: 4px;
  background: #E2E8F0;
  border-radius: 4px;
  overflow: hidden;
}

.bm-progress-bar {
  width: 40%;
  height: 100%;
  background: linear-gradient(90deg, var(--app-primary), #60A5FA);
  border-radius: 4px;
  animation: bm-progress-slide 1.5s ease-in-out infinite;
}

@keyframes bm-progress-slide {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(350%); }
}

.bm-build-output {
  margin: 12px 22px 0;
  padding: 10px 14px;
  background: #F0FDF4;
  border: 1px solid #BBF7D0;
  border-radius: 8px;
  font-size: 0.78rem;
  color: #166534;
  display: flex;
  align-items: center;
  gap: 8px;
  word-break: break-all;
}

.bm-build-log-wrap {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 12px 22px 18px;
}

.bm-build-log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
  font-size: 0.78rem;
  font-weight: 600;
  color: #475569;
}

.bm-build-log {
  flex: 1;
  min-height: 300px;
  max-height: 50vh;
  overflow-y: auto;
  margin: 0;
  padding: 14px;
  background: #0F172A;
  color: #A5F3FC;
  border-radius: 8px;
  font-size: 0.72rem;
  line-height: 1.7;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  white-space: pre-wrap;
  word-break: break-all;
}

.bm-build-log::-webkit-scrollbar { width: 6px; }
.bm-build-log::-webkit-scrollbar-track { background: transparent; }
.bm-build-log::-webkit-scrollbar-thumb { background: #334155; border-radius: 3px; }

/* Dark mode build dialog */
.bm-dark .bm-build-dialog { background: #1E293B; }
.bm-dark .bm-build-header { border-bottom-color: #334155; }
.bm-dark .bm-build-header h3 { color: #F1F5F9; }
.bm-dark .bm-build-badge--building { background: rgba(37, 99, 235, 0.15); color: #60A5FA; }
.bm-dark .bm-build-badge--success { background: rgba(22, 163, 74, 0.15); color: #4ADE80; }
.bm-dark .bm-build-badge--error { background: rgba(220, 38, 38, 0.15); color: #F87171; }
.bm-dark .bm-progress-track { background: #334155; }
.bm-dark .bm-build-output { background: rgba(22, 163, 74, 0.1); border-color: #334155; color: #4ADE80; }
.bm-dark .bm-build-log-header { color: #94A3B8; }
.bm-dark .bm-build-log { background: #0F172A; border: 1px solid #334155; }
</style>
