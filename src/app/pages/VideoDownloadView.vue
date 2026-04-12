<script setup lang="ts">
import { ref, reactive, computed, onActivated, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import Textarea from 'primevue/textarea'
import Button from 'primevue/button'
import DataTable from 'primevue/datatable'
import Column from 'primevue/column'
import ProgressBar from 'primevue/progressbar'
import TabView from 'primevue/tabview'
import TabPanel from 'primevue/tabpanel'
import Dialog from 'primevue/dialog'
import InputText from 'primevue/inputtext'
import Select from 'primevue/select'
import Tag from 'primevue/tag'

// ── 数据模型 ──────────────────────────────────────────────────────

interface ParsedVideoInfo {
  platform: string
  avatar: string
  author_name: string
  author_id: string
  author_uid: string
  author_desc: string
  home_url: string
  video_id: string
  video_name: string
  hash: string
  video_url: string
  video_url_fallbacks: string[]
  cover_url: string
  cover_url_fallbacks: string[]
  collection_name: string
  topics: string
  publish_time: string
  publish_timestamp: number
  video_width: number
  video_height: number
  duration: number
  video_ext: string
  video_resolution: string
  video_bitrate: string
  video_size: string
  video_codec: string
  cover_width: number
  cover_height: number
  cover_resolution: string
  likes: number
  plays: number
  shares: number
  comments: number
  favorites: number
}

interface VideoItem extends ParsedVideoInfo {
  [key: string]: unknown
  id: number
  _tab: string
  _status: 'parsed' | 'queued' | 'downloading' | 'completed' | 'failed'
  _progress: number
  _error_msg: string
  _file_path: string
  _file_size: number
  _speed: string
  _dl_tasks: BatchDownloadTask[]
}

interface PlatformAccount {
  id: number
  platform: string
  name: string
  cookies: string
  avatar: string
  status: string
  remark: string
}

interface AccountCookie {
  id: number
  name: string
  cookies: string
}

interface BatchDownloadTask {
  url: string
  save_path: string
  task_id: string
  fallback_urls: string[]
}

// ── 状态 ──────────────────────────────────────────────────────────

const linkText = ref('')
const isParsing = ref(false)
const parseCancelled = ref(false)
const activeTab = ref(0)
const hintMessage = ref('')
const hintIsError = ref(false)
let nextId = 1

const allVideos = ref<VideoItem[]>([])
const selectedParsed = ref<VideoItem[]>([])

const parseProgress = reactive({
  current: 0,
  total: 0,
  success: 0,
  failed: 0,
  message: '',
})

const filterDialogVisible = ref(false)
const filterOptions = reactive({
  min_duration: '',
  max_duration: '',
  min_likes: '',
  max_likes: '',
  date_from: '',
  date_to: '',
})

function applyFilter() {
  const minDur = filterOptions.min_duration ? Number(filterOptions.min_duration) : 0
  const maxDur = filterOptions.max_duration ? Number(filterOptions.max_duration) : Infinity
  const minLikes = filterOptions.min_likes ? Number(filterOptions.min_likes) : 0
  const maxLikes = filterOptions.max_likes ? Number(filterOptions.max_likes) : Infinity
  const dateFrom = filterOptions.date_from || ''
  const dateTo = filterOptions.date_to || ''

  const matched: VideoItem[] = []
  for (const v of parsedVideos.value) {
    if (v.duration < minDur || v.duration > maxDur) continue
    if (v.likes < minLikes || v.likes > maxLikes) continue
    if (dateFrom && v.publish_time < dateFrom) continue
    if (dateTo && v.publish_time > dateTo + '\uffff') continue
    matched.push(v)
  }
  selectedParsed.value = matched
  filterDialogVisible.value = false
  setHint(`已筛选 ${matched.length} 条视频`)
}

function resetFilter() {
  filterOptions.min_duration = ''
  filterOptions.max_duration = ''
  filterOptions.min_likes = ''
  filterOptions.max_likes = ''
  filterOptions.date_from = ''
  filterOptions.date_to = ''
  selectedParsed.value = []
  filterDialogVisible.value = false
  setHint('已清除筛选')
}

const dlSubmitting = ref(0)
const dlOptions = reactive({
  output_dir: '',
  video: true,
  cover: false,
  concurrent: 5,
  add_seq: false,
  remove_topics: false,
  remove_at: false,
})
const dlProgress = reactive({ completed: 0, total: 0, bytes: 0 })
const dlSpeed = ref('')
const lastFailedTasks = ref<BatchDownloadTask[]>()
let dlSpeedTimer: ReturnType<typeof setInterval> | null = null
let dlLastBytes = 0
let dlLastTime = Date.now()

function startSpeedTracker() {
  if (dlSpeedTimer) return
  dlLastBytes = dlProgress.bytes
  dlLastTime = Date.now()
  dlSpeedTimer = setInterval(() => {
    const now = Date.now()
    const elapsed = (now - dlLastTime) / 1000
    if (elapsed > 0) {
      const speed = (dlProgress.bytes - dlLastBytes) / elapsed
      dlSpeed.value = speed > 1048576 ? `${(speed / 1048576).toFixed(1)} MB/s`
        : speed > 1024 ? `${(speed / 1024).toFixed(0)} KB/s` : '0 MB/s'
    }
    dlLastBytes = dlProgress.bytes
    dlLastTime = now
    if (dlSubmitting.value === 0 && dlSpeedTimer) { clearInterval(dlSpeedTimer); dlSpeedTimer = null; dlSpeed.value = '' }
  }, 1000)
}

const parsedVideos = computed(() => allVideos.value.filter(v => v._status === 'parsed'))
const downloadingVideos = computed(() => allVideos.value.filter(v => v._status === 'queued' || v._status === 'downloading'))
const failedVideos = computed(() => allVideos.value.filter(v => v._status === 'failed'))
const completedVideos = computed(() => allVideos.value.filter(v => v._status === 'completed'))

const tabCounts = computed(() => ({
  parsed: parsedVideos.value.length,
  downloading: downloadingVideos.value.length,
  failed: failedVideos.value.length,
  completed: completedVideos.value.length,
}))

// ── 辅助函数 ──────────────────────────────────────────────────────

function setHint(msg: string, isError = false) { hintMessage.value = msg; hintIsError.value = isError }

function formatDuration(seconds: number): string {
  if (!seconds) return '--'
  const m = Math.floor(seconds / 60)
  const s = Math.round(seconds % 60)
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
}

function formatFileSize(bytes: number): string {
  if (!bytes) return '--'
  if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(1)}GB`
  if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(1)}MB`
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)}KB`
  return `${Math.round(bytes)}B`
}

function formatCount(n: number): string {
  if (!n) return '0'
  if (n >= 10000) return `${(n / 10000).toFixed(1)}万`
  return String(n)
}

function parseCnNumber(s: string): number {
  if (!s) return 0
  const n = parseFloat(s)
  if (isNaN(n)) return 0
  if (s.includes('亿')) return Math.round(n * 100000000)
  if (s.includes('万')) return Math.round(n * 10000)
  return n
}

function sanitizeFilename(name: string): string {
  return name.replace(/[/\\:*?"<>|]/g, '_').replace(/\s+/g, ' ').trim() || '未命名'
}

// ── 平台检测 ──────────────────────────────────────────────────────

function detectPlatform(url: string): string {
  if (url.includes('douyin.com') || url.includes('v.douyin.com')) return 'douyin'
  if (url.includes('kuaishou.com')) return 'kuaishou'
  if (url.includes('bilibili.com') || url.includes('b23.tv')) return 'bilibili'
  if (url.includes('miguvideo.com')) return 'migu'
  if (url.includes('tv.cctv.com') || url.includes('cctv.com/video')) return 'cctv'
  if (url.includes('yangshipin.cn')) return 'yangshipin'
  if (url.includes('xiaohongshu.com') || url.includes('xhslink.com')) return 'xiaohongshu'
  return ''
}

function extractUrl(text: string): string {
  const m = text.match(/https?:\/\/[^\s]+/)
  return m ? m[0] : text.trim()
}

const platformLabelMap: Record<string, string> = { douyin: 'DY', kuaishou: 'KS', bilibili: 'BLB', migu: 'MG', cctv: 'CCTV', yangshipin: 'YSP', xiaohongshu: '小红薯' }
const platformClassMap: Record<string, string> = { DY: 'vd-plat--dy', KS: 'vd-plat--ks', BLB: 'vd-plat--blb', MG: 'vd-plat--mg', CCTV: 'vd-plat--cctv', YSP: 'vd-plat--ysp', '小红薯': 'vd-plat--xhs' }

const AUTHOR_COLORS = [
  '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6',
  '#06b6d4', '#ec4899', '#14b8a6', '#f97316', '#6366f1',
  '#84cc16', '#e11d48', '#0ea5e9', '#a855f7', '#22c55e',
]
const authorColorCache = new Map<string, string>()
function getAuthorColor(name: string): string {
  if (!name) return '#64748b'
  if (authorColorCache.has(name)) return authorColorCache.get(name)!
  let hash = 0
  for (let i = 0; i < name.length; i++) hash = ((hash << 5) - hash + name.charCodeAt(i)) | 0
  const color = AUTHOR_COLORS[Math.abs(hash) % AUTHOR_COLORS.length]
  authorColorCache.set(name, color)
  return color
}

const tooltip = reactive({ visible: false, text: '', x: 0, y: 0 })
function showTooltip(e: MouseEvent, text: string) {
  if (!text) return
  tooltip.text = text; tooltip.x = e.clientX + 8; tooltip.y = e.clientY - 36; tooltip.visible = true
}
function hideTooltip() { tooltip.visible = false }

function detectUrlType(resolvedUrl: string): 'video' | 'homepage' | 'compilation' {
  if (resolvedUrl.includes('miguvideo.com')) {
    if (resolvedUrl.includes('/p/user/') || resolvedUrl.includes('authorId=')) return 'homepage'
    return 'video'
  }
  if (resolvedUrl.includes('cctv.com')) {
    if (resolvedUrl.includes('/lm/') || resolvedUrl.includes('/lanmu/')) return 'homepage'
    return 'video'
  }
  if (resolvedUrl.includes('yangshipin.cn')) return 'video'
  if (resolvedUrl.includes('xiaohongshu.com')) {
    if (resolvedUrl.includes('/user/profile/')) return 'homepage'
    return 'video'
  }
  if (resolvedUrl.includes('kuaishou.com') || resolvedUrl.includes('live.kuaishou.com')) {
    if (resolvedUrl.includes('/profile/')) return 'homepage'
    return 'video'
  }
  if (resolvedUrl.includes('bilibili.com')) {
    if (resolvedUrl.includes('/space/') || resolvedUrl.includes('space.bilibili.com')) return 'homepage'
    return 'video'
  }
  if (resolvedUrl.includes('showSubTab=compilation') || resolvedUrl.includes('/collection/')) return 'compilation'
  if (resolvedUrl.includes('/user/')) {
    const modalMatch = resolvedUrl.match(/[?&]modal_id=(\d+)/)
    if (modalMatch) return 'video'
    return 'homepage'
  }
  return 'video'
}

function extractSecUid(resolvedUrl: string): string {
  const m = resolvedUrl.match(/\/user\/([^/?#]+)/)
  if (m) return m[1]
  throw new Error(`无法从URL提取sec_uid: ${resolvedUrl}`)
}

function extractVideoId(platform: string, resolvedUrl: string): string {
  if (platform === 'douyin') {
    const modal = resolvedUrl.match(/[?&]modal_id=(\d+)/)
    if (modal) return modal[1]
    const coll = resolvedUrl.match(/\/collection\/(\d+)/)
    if (coll) return coll[1]
    const video = resolvedUrl.match(/\/video\/(\d+)/)
    if (video) return video[1]
    const note = resolvedUrl.match(/\/note\/(\d+)/)
    if (note) return note[1]
  }
  if (platform === 'kuaishou') {
    const m = resolvedUrl.match(/\/short-video\/([^/?#]+)/)
    if (m) return m[1]
    try {
      const photoId = new URL(resolvedUrl).searchParams.get('photoId')
      if (photoId) return photoId
    } catch { /* ignore */ }
  }
  if (platform === 'migu') {
    const m = resolvedUrl.match(/\/p\/(?:vertical|detail)\/(\d+)/)
    if (m) return m[1]
  }
  throw new Error(`无法从URL提取视频ID: ${resolvedUrl}`)
}

function extractBilibiliBvid(resolvedUrl: string): string {
  const m = resolvedUrl.match(/\/video\/(BV[a-zA-Z0-9]+)/)
  if (m) return m[1]
  throw new Error(`无法从URL提取B站BV号: ${resolvedUrl}`)
}

function extractBilibiliMid(resolvedUrl: string): string {
  const m = resolvedUrl.match(/\/space\/(\d+)/)
  if (m) return m[1]
  const m2 = resolvedUrl.match(/space\.bilibili\.com\/(\d+)/)
  if (m2) return m2[1]
  throw new Error(`无法从URL提取B站用户ID: ${resolvedUrl}`)
}

// ── Cookie 池（保留用于兼容，CDP 模式下仅检测是否有登录账号） ──

async function getPlatformCookiePool(platform: string): Promise<AccountCookie[]> {
  const accounts = await invoke<PlatformAccount[]>('list_download_accounts')
  const pool = accounts
    .filter(a => a.platform === platform && a.status === 'active' && a.cookies)
    .map(a => ({ id: a.id, name: a.name, cookies: a.cookies! }))
  if (pool.length === 0) {
    throw new Error(`没有在线的${platformLabelMap[platform] || platform}账号，请先在「账号登记」中登录`)
  }
  return pool
}

async function ensureCdpChrome(): Promise<void> {
  await invoke<boolean>('cdp_ensure_chrome')
}

// ── 解析函数 ──────────────────────────────────────────────────────

function parseDouyinDetail(d: Record<string, unknown>): ParsedVideoInfo {
  const author = (d.author || {}) as Record<string, unknown>
  const video = (d.video || {}) as Record<string, unknown>
  const stats = (d.statistics || {}) as Record<string, unknown>
  const images = (d.images || []) as Record<string, unknown>[]
  const isImagePost = images.length > 0

  const secUid = String(author.sec_uid || '')
  const avatarThumb = author.avatar_thumb as Record<string, unknown> | undefined
  const avatarUrls = (avatarThumb?.url_list || []) as string[]

  const playAddr = video.play_addr as Record<string, unknown> | undefined
  const playUrls = (playAddr?.url_list || []) as string[]
  const fileHash = String(playAddr?.file_hash || '')
  const dataSize = Number(playAddr?.data_size || 0)

  const durationSec = Number(d.duration || video.duration || 0) / 1000
  const calcBitrate = durationSec > 0 && dataSize > 0 ? Math.round(dataSize * 8 / durationSec) : 0

  const originCover = video.origin_cover as Record<string, unknown> | undefined
  const cover = video.cover as Record<string, unknown> | undefined
  const dynamicCover = video.dynamic_cover as Record<string, unknown> | undefined
  // 封面优先级：origin_cover → cover → dynamic_cover
  const allCoverUrls: string[] = []
  for (const src of [originCover, cover, dynamicCover]) {
    for (const u of ((src?.url_list || []) as string[])) {
      if (u && !allCoverUrls.includes(u)) allCoverUrls.push(u)
    }
  }
  const cw = Number(originCover?.width || cover?.width || 0)
  const ch = Number(originCover?.height || cover?.height || 0)

  const createTime = Number(d.create_time || 0)
  const textExtra = (d.text_extra || []) as Record<string, unknown>[]
  const topics = textExtra.map(t => t.hashtag_name as string).filter(Boolean).map(t => `#${t}`).join(' ')

  const publishTime = createTime > 0
    ? new Date(createTime * 1000).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai', hour12: false }).replace(/\//g, '-')
    : ''

  let videoUrl = ''
  let vw = 0, vh = 0, videoExt = '', videoCodec = '', videoResolution = ''

  if (isImagePost) {
    const imgUrls = images.map(img => {
      const urls = ((img.url_list || []) as string[])
      return urls.length > 0 ? urls[0] : ''
    }).filter(Boolean)
    videoUrl = imgUrls.join('\n')
    const firstImg = images[0]
    vw = Number(firstImg?.width || 0)
    vh = Number(firstImg?.height || 0)
    videoExt = 'jpeg'
    videoCodec = `图片x${images.length}`
    videoResolution = vw && vh ? `${vw}x${vh}` : ''
  } else {
    videoUrl = playUrls.length > 0 ? playUrls[0] : ''
    vw = Number(playAddr?.width || video.width || 0)
    vh = Number(playAddr?.height || video.height || 0)
    const rawFormat = String(video.format || '')
    videoExt = (rawFormat === 'dash' || !rawFormat) ? 'mp4' : rawFormat
    videoCodec = rawFormat === 'dash' ? 'H265' : 'H264'
    videoResolution = vw && vh ? `${Math.min(vw, vh)}p` : ''
  }

  // 收集视频备用 URL：play_addr 剩余 + download_addr
  const videoFallbacks: string[] = []
  if (!isImagePost) {
    for (const u of playUrls.slice(1)) { if (u && !videoFallbacks.includes(u)) videoFallbacks.push(u) }
    const dlAddr = video.download_addr as Record<string, unknown> | undefined
    for (const u of ((dlAddr?.url_list || []) as string[])) { if (u && u !== videoUrl && !videoFallbacks.includes(u)) videoFallbacks.push(u) }
  }

  return {
    platform: 'DY', avatar: avatarUrls[0] || '',
    author_name: String(author.nickname || ''), author_id: String(author.unique_id || author.uid || ''),
    author_uid: String(author.uid || ''), author_desc: String(author.signature || ''),
    home_url: secUid ? `https://www.douyin.com/user/${secUid}` : '',
    video_id: String(d.aweme_id || ''), video_name: String(d.desc || ''),
    hash: fileHash, video_url: videoUrl, video_url_fallbacks: videoFallbacks,
    cover_url: allCoverUrls[0] || '', cover_url_fallbacks: allCoverUrls.slice(1),
    collection_name: String((d.mix_info as Record<string, unknown>)?.mix_name || ''),
    topics, publish_time: publishTime, publish_timestamp: createTime,
    video_width: vw, video_height: vh, duration: durationSec,
    video_ext: videoExt, video_resolution: videoResolution,
    video_bitrate: calcBitrate > 0 ? `${Math.round(calcBitrate / 1000)}kbps` : '',
    video_size: dataSize > 0 ? formatFileSize(dataSize) : '',
    video_codec: videoCodec, cover_width: cw, cover_height: ch,
    cover_resolution: cw && ch ? `${Math.min(cw, ch)}p` : '',
    likes: Number(stats.digg_count || 0), plays: Number(stats.play_count || 0),
    shares: Number(stats.share_count || 0), comments: Number(stats.comment_count || 0),
    favorites: Number(stats.collect_count || 0),
  }
}

function parseKuaishouDetail(apolloState: Record<string, unknown>, photoId: string): ParsedVideoInfo {
  const dc = (apolloState as Record<string, Record<string, unknown>>).defaultClient
  let author: Record<string, unknown> = {}
  let photo: Record<string, unknown> = {}

  for (const key of Object.keys(dc)) {
    if (key.startsWith('VisionVideoDetailAuthor:')) author = dc[key] as Record<string, unknown>
    if (key === `VisionVideoDetailPhoto:${photoId}`) photo = dc[key] as Record<string, unknown>
  }

  const profileData = dc.__kuaishou_profile__ as Record<string, unknown> | undefined
  const profile = profileData?.profile as Record<string, unknown> | undefined
  const userDefineId = String(profileData?.userDefineId || '')
  const authorId = String(author.id || '')
  const ksId = userDefineId || String(profile?.user_id || authorId)
  const authorDesc = String(profile?.user_text || '').replace(/\n/g, ' ')

  const videoResource = photo.videoResource as { type?: string; json?: Record<string, unknown> } | undefined
  const h264 = videoResource?.json?.h264 as Record<string, unknown> | undefined
  const adaptationSets = h264?.adaptationSet as Record<string, unknown>[] | undefined
  const representations = adaptationSets?.[0]?.representation as Record<string, unknown>[] | undefined
  const rep = representations?.[0]

  const vw = Number(rep?.width || 0), vh = Number(rep?.height || 0)
  const fileSize = Number(rep?.fileSize || 0), avgBitrate = Number(rep?.avgBitrate || 0)
  const qualityType = String(rep?.qualityType || '')
  const durationMs = Number(photo.duration || 0)
  const timestamp = Number(photo.timestamp || 0)
  const publishTime = timestamp > 0 ? new Date(timestamp).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai', hour12: false }).replace(/\//g, '-') : ''
  const caption = String(photo.caption || '')
  const tags = caption.match(/#[^\s#]+/g)

  return {
    platform: 'KS', avatar: String(profile?.headurl || author.headerUrl || ''),
    author_name: String(profile?.user_name || author.name || ''), author_id: ksId,
    author_uid: authorId, author_desc: authorDesc || '无',
    home_url: authorId ? `https://www.kuaishou.com/profile/${authorId}` : '无',
    video_id: String(photo.id || photoId), video_name: caption || '无',
    hash: '无', video_url: String(photo.photoUrl || rep?.url || '无'), video_url_fallbacks: [],
    cover_url: String(photo.coverUrl || '无'), cover_url_fallbacks: [], collection_name: '',
    topics: tags && tags.length > 0 ? tags.join(' ') : '无',
    publish_time: publishTime || '无', publish_timestamp: timestamp,
    video_width: vw, video_height: vh, duration: durationMs / 1000,
    video_ext: 'mp4', video_resolution: qualityType || (vw && vh ? `${Math.min(vw, vh)}p` : '无'),
    video_bitrate: avgBitrate > 0 ? `${avgBitrate}kbps` : '无',
    video_size: fileSize > 0 ? formatFileSize(fileSize) : '无',
    video_codec: h264 ? 'H264' : '无',
    cover_width: Number(dc.__cover_width__ || 0), cover_height: Number(dc.__cover_height__ || 0),
    cover_resolution: (() => { const cw2 = Number(dc.__cover_width__ || 0); const ch2 = Number(dc.__cover_height__ || 0); return cw2 && ch2 ? `${Math.min(cw2, ch2)}p` : '无' })(),
    likes: Number(photo.realLikeCount || 0) || parseCnNumber(String(photo.likeCount || '0')),
    plays: parseCnNumber(String(photo.viewCount || '0')),
    shares: 0, comments: 0, favorites: 0,
  }
}

function parseKuaishouFeedItem(item: Record<string, unknown>): ParsedVideoInfo {
  const author = (item.author || {}) as Record<string, unknown>
  const photo = (item.photo || {}) as Record<string, unknown>
  const authorId = String(author.id || '')

  const caption = String(photo.caption || '')
  const durationMs = Number(photo.duration || 0)
  const timestamp = Number(photo.timestamp || 0)
  const vw = Number(photo.width || 0)
  const vh = Number(photo.height || 0)

  const publishTime = timestamp > 0
    ? new Date(timestamp).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai', hour12: false }).replace(/\//g, '-')
    : '无'
  const tags = caption.match(/#[^\s#]+/g)

  const photoUrls = (photo.photoUrls || []) as { url?: string }[]
  const videoUrl = photoUrls[0]?.url || ''
  const fallbackUrls = photoUrls.slice(1).map(u => String(u.url || '')).filter(u => u)

  const manifest = photo.manifest as Record<string, unknown> | undefined
  const adaptSets = (manifest?.adaptationSet || []) as Record<string, unknown>[]
  const reps = (adaptSets[0]?.representation || []) as Record<string, unknown>[]
  const rep = reps[0] as Record<string, unknown> | undefined
  const avgBitrate = Number(rep?.avgBitrate || 0)
  const fileSize = Number(rep?.fileSize || 0)
  const qualityType = String(rep?.qualityType || '')
  const frameRate = Number(rep?.frameRate || 0)

  return {
    platform: 'KS', avatar: String(author.headerUrl || ''),
    author_name: String(author.name || ''), author_id: authorId,
    author_uid: authorId, author_desc: '无',
    home_url: authorId ? `https://www.kuaishou.com/profile/${authorId}` : '无',
    video_id: String(photo.id || ''), video_name: caption || '无', hash: '无',
    video_url: videoUrl || '无', video_url_fallbacks: fallbackUrls,
    cover_url: String(photo.coverUrl || '无'), cover_url_fallbacks: [], collection_name: '',
    topics: tags && tags.length > 0 ? tags.join(' ') : '无',
    publish_time: publishTime, publish_timestamp: timestamp,
    video_width: vw, video_height: vh, duration: durationMs / 1000,
    video_ext: 'mp4',
    video_resolution: qualityType || (vw && vh ? `${Math.min(vw, vh)}p` : '无'),
    video_bitrate: avgBitrate > 0 ? `${avgBitrate}kbps` : '无',
    video_size: fileSize > 0 ? formatFileSize(fileSize) : '无',
    video_codec: rep ? 'H264' : '无',
    cover_width: 0, cover_height: 0, cover_resolution: vw && vh ? `${Math.min(vw, vh)}p` : '无',
    likes: Number(photo.likeCount || 0),
    plays: Number(photo.viewCount || 0),
    shares: 0, comments: 0, favorites: Number(photo.collectCount || 0),
  }
}

function parseBilibiliDetail(d: Record<string, unknown>): ParsedVideoInfo {
  const owner = (d.owner || {}) as Record<string, unknown>
  const stat = (d.stat || {}) as Record<string, unknown>
  const mid = String(owner.mid || '')
  const duration = Number(d.duration || 0)
  const pubdate = Number(d.pubdate || 0)
  const vw = Number(d.width || 0), vh = Number(d.height || 0)
  const videoSize = Number(d.video_size || 0)
  const videoBitrate = Number(d.video_bitrate || 0)
  const publishTime = pubdate > 0 ? new Date(pubdate * 1000).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai', hour12: false }).replace(/\//g, '-') : ''
  const desc = String(d.desc || '')
  const tags = desc.match(/#[^\s#]+/g)
  return {
    platform: 'BLB', avatar: String(owner.face || ''),
    author_name: String(owner.name || ''), author_id: mid, author_uid: mid,
    author_desc: String(owner.sign || ''),
    home_url: mid ? `https://space.bilibili.com/${mid}` : '',
    video_id: String(d.bvid || ''), video_name: String(d.title || ''), hash: '',
    video_url: String(d.video_url || ''),
    video_url_fallbacks: ((d.video_url_fallbacks || []) as string[]).filter(u => u),
    cover_url: String(d.pic || ''), cover_url_fallbacks: [], collection_name: '',
    topics: tags && tags.length > 0 ? tags.join(' ') : '',
    publish_time: publishTime, publish_timestamp: pubdate,
    video_width: vw, video_height: vh, duration,
    video_ext: 'mp4', video_resolution: vw && vh ? `${Math.min(vw, vh)}p` : '',
    video_bitrate: videoBitrate > 0 ? `${Math.round(videoBitrate / 1000)}kbps` : '',
    video_size: videoSize > 0 ? formatFileSize(videoSize) : '',
    video_codec: String(d.video_codec || ''),
    cover_width: 0, cover_height: 0, cover_resolution: '',
    likes: Number(stat.like || 0), plays: Number(stat.view || 0),
    shares: Number(stat.share || 0), comments: Number(stat.reply || 0),
    favorites: Number(stat.favorite || 0),
  }
}

function parseBilibiliFeedItem(item: Record<string, unknown>): ParsedVideoInfo {
  const card = (item.__card__ || {}) as Record<string, unknown>
  const mid = String(item.mid || card.mid || '')
  const durationStr = String(item.length || '0:0')
  const parts = durationStr.split(':')
  const duration = parts.length === 2 ? Number(parts[0]) * 60 + Number(parts[1]) : 0
  const created = Number(item.created || 0)
  const publishTime = created > 0 ? new Date(created * 1000).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai', hour12: false }).replace(/\//g, '-') : ''
  const title = String(item.title || '')
  const tags = title.match(/#[^\s#]+/g)
  const vw = Number(item.__width__ || 0), vh = Number(item.__height__ || 0)
  const vSize = Number(item.__video_size__ || 0), vBitrate = Number(item.__video_bitrate__ || 0)
  return {
    platform: 'BLB', avatar: String(card.face || ''),
    author_name: String(item.author || card.name || ''), author_id: mid, author_uid: mid,
    author_desc: String(card.sign || ''),
    home_url: mid ? `https://space.bilibili.com/${mid}` : '',
    video_id: String(item.bvid || ''), video_name: title, hash: '',
    video_url: String(item.__video_url__ || ''),
    video_url_fallbacks: ((item.__video_url_fallbacks__ || []) as string[]).filter(u => u),
    cover_url: String(item.pic || ''), cover_url_fallbacks: [], collection_name: '',
    topics: tags && tags.length > 0 ? tags.join(' ') : '',
    publish_time: publishTime, publish_timestamp: created,
    video_width: vw, video_height: vh, duration,
    video_ext: 'mp4', video_resolution: vw && vh ? `${Math.min(vw, vh)}p` : '',
    video_bitrate: vBitrate > 0 ? `${Math.round(vBitrate / 1000)}kbps` : '',
    video_size: vSize > 0 ? formatFileSize(vSize) : '',
    video_codec: String(item.__video_codec__ || ''),
    cover_width: 0, cover_height: 0, cover_resolution: '',
    likes: Number(item.__like__ || item.like || 0), plays: Number(item.__view__ || item.play || 0),
    shares: Number(item.__share__ || 0), comments: Number(item.__reply__ || item.comment || 0),
    favorites: Number(item.__favorite__ || item.favorites || 0),
  }
}

function parseMiguDetail(d: Record<string, unknown>): ParsedVideoInfo {
  const play = (d.play || {}) as Record<string, unknown>
  const content = (d.content || {}) as Record<string, unknown>
  const resolution = (content.resolution || {}) as Record<string, unknown>
  const pics = (play.h5pics || content.pics || {}) as Record<string, unknown>
  const durationStr = String(play.duration || '0:00')
  const dParts = durationStr.split(':')
  const duration = dParts.length === 2 ? Number(dParts[0]) * 60 + Number(dParts[1]) : 0
  const vw = Number(resolution.mediaWidth || 0)
  const vh = Number(resolution.mediaHeight || 0)
  const m3u8Url = String(d.m3u8_url || '')
  const publishTime = String(content.publishTime || play.publishTime || '')
  const authorId = String(content.author || '')

  return {
    platform: 'MG',
    avatar: '',
    author_name: authorId,
    author_id: authorId,
    author_uid: authorId,
    author_desc: '无',
    home_url: '无',
    video_id: String(play.pID || d.contentId || ''),
    video_name: String(play.name || play.detail || content.name || ''),
    hash: '',
    video_url: m3u8Url,
    video_url_fallbacks: [],
    cover_url: String(pics.highResolutionV || pics.highResolutionH || pics.lowResolutionH || ''),
    cover_url_fallbacks: [],
    collection_name: '',
    topics: '无',
    publish_time: publishTime || '无',
    publish_timestamp: 0,
    video_width: vw,
    video_height: vh,
    duration,
    video_ext: 'mp4',
    video_resolution: vw && vh ? `${Math.min(vw, vh)}p` : '无',
    video_bitrate: '无',
    video_size: '无',
    video_codec: 'H264',
    cover_width: 0,
    cover_height: 0,
    cover_resolution: '无',
    likes: Number(d.likeCount || 0),
    plays: Number(d.playCount || 0),
    shares: 0,
    comments: Number(d.commentCount || 0),
    favorites: 0,
  }
}

function parseMiguHomepageItem(item: Record<string, unknown>): ParsedVideoInfo {
  const pics = (item.pics || {}) as Record<string, unknown>
  const durationStr = String(item.duration || '0:00')
  const dParts = durationStr.split(':')
  const duration = dParts.length === 2 ? Number(dParts[0]) * 60 + Number(dParts[1]) : 0
  const publishTime = Number(item.publishTime || 0)
  const publishStr = publishTime > 0
    ? new Date(publishTime).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai', hour12: false }).replace(/\//g, '-')
    : '无'
  const direction = String(item.direction || '')
  const vw = direction === 'vertical' ? 1080 : direction === 'horizontal' ? 1920 : 0
  const vh = direction === 'vertical' ? 1920 : direction === 'horizontal' ? 1080 : 0
  const m3u8Url = String(item.m3u8_url || '')

  return {
    platform: 'MG',
    avatar: String(item.author_avatar || ''),
    author_name: String(item.author_name || ''),
    author_id: String(item.author_id || item.gkeUserid || ''),
    author_uid: String(item.gkeUserid || item.author_id || ''),
    author_desc: '无',
    home_url: '无',
    video_id: String(item.contentId || item.pID || ''),
    video_name: String(item.name || item.description || ''),
    hash: '',
    video_url: m3u8Url,
    video_url_fallbacks: [],
    cover_url: String(pics.highResolutionV || pics.highResolutionH || pics.lowResolutionH || ''),
    cover_url_fallbacks: [],
    collection_name: '',
    topics: '无',
    publish_time: publishStr,
    publish_timestamp: publishTime,
    video_width: vw,
    video_height: vh,
    duration,
    video_ext: 'mp4',
    video_resolution: vw && vh ? `${Math.min(vw, vh)}p` : '无',
    video_bitrate: '无',
    video_size: '无',
    video_codec: 'H264',
    cover_width: 0, cover_height: 0, cover_resolution: '无',
    likes: 0, plays: 0, shares: 0, comments: 0, favorites: 0,
  }
}

// ── 数据管理 ──────────────────────────────────────────────────────

function parseCctvDetail(d: Record<string, unknown>): ParsedVideoInfo {
  const durationSecs = Number(d.duration || 0)
  const vw = Number(d.video_width || 0)
  const vh = Number(d.video_height || 0)
  const m3u8Url = String(d.m3u8_url || '')
  const column = String(d.column || '')

  return {
    platform: 'CCTV',
    avatar: '',
    author_name: column || 'CCTV',
    author_id: column,
    author_uid: column,
    author_desc: '无',
    home_url: '无',
    video_id: String(d.guid || ''),
    video_name: String(d.title || ''),
    hash: '',
    video_url: m3u8Url,
    video_url_fallbacks: [],
    cover_url: String(d.cover_url || ''),
    cover_url_fallbacks: [],
    collection_name: '',
    topics: '无',
    publish_time: String(d.publish_time || '无'),
    publish_timestamp: 0,
    video_width: vw,
    video_height: vh,
    duration: durationSecs,
    video_ext: 'mp4',
    video_resolution: vw && vh ? `${Math.min(vw, vh)}p` : '无',
    video_bitrate: '无',
    video_size: '无',
    video_codec: 'H264',
    cover_width: 0, cover_height: 0, cover_resolution: '无',
    likes: 0, plays: 0, shares: 0, comments: 0, favorites: 0,
  }
}

function parseCctvColumnItem(item: Record<string, unknown>): ParsedVideoInfo {
  const lengthStr = String(item.length || '0:00')
  const parts = lengthStr.split(':')
  const duration = parts.length === 3
    ? Number(parts[0]) * 3600 + Number(parts[1]) * 60 + Number(parts[2])
    : parts.length === 2 ? Number(parts[0]) * 60 + Number(parts[1]) : 0
  const timeStr = String(item.time || '')
  const columnName = String(item.column_name || 'CCTV')

  return {
    platform: 'CCTV',
    avatar: '',
    author_name: columnName,
    author_id: columnName,
    author_uid: columnName,
    author_desc: '无',
    home_url: '无',
    video_id: String(item.guid || ''),
    video_name: String(item.title || ''),
    hash: '',
    video_url: String(item.m3u8_url || ''),
    video_url_fallbacks: [],
    cover_url: String(item.image || ''),
    cover_url_fallbacks: [],
    collection_name: '',
    topics: '无',
    publish_time: timeStr || '无',
    publish_timestamp: 0,
    video_width: 0,
    video_height: 0,
    duration,
    video_ext: 'mp4',
    video_resolution: '无',
    video_bitrate: '无',
    video_size: '无',
    video_codec: 'H264',
    cover_width: 0, cover_height: 0, cover_resolution: '无',
    likes: 0, plays: 0, shares: 0, comments: 0, favorites: 0,
  }
}

function parseYangshipinDetail(d: Record<string, unknown>): ParsedVideoInfo {
  const durationSecs = Number(d.duration || 0)
  const title = String(d.title || '')

  return {
    platform: 'YSP',
    avatar: '',
    author_name: '央视频',
    author_id: 'yangshipin',
    author_uid: 'yangshipin',
    author_desc: '无',
    home_url: 'https://yangshipin.cn',
    video_id: String(d.vid || ''),
    video_name: title,
    hash: '',
    video_url: String(d.mp4_url || ''),
    video_url_fallbacks: [],
    cover_url: '',
    cover_url_fallbacks: [],
    collection_name: '',
    topics: '无',
    publish_time: '无',
    publish_timestamp: 0,
    video_width: 0,
    video_height: 0,
    duration: durationSecs,
    video_ext: 'mp4',
    video_resolution: '无',
    video_bitrate: '无',
    video_size: '无',
    video_codec: 'H264',
    cover_width: 0, cover_height: 0, cover_resolution: '无',
    likes: 0, plays: 0, shares: 0, comments: 0, favorites: 0,
  }
}

function parseXiaohongshuDetail(d: Record<string, unknown>): ParsedVideoInfo {
  const user = (d.user || {}) as Record<string, unknown>
  const interact = (d.interactInfo || {}) as Record<string, unknown>
  const durationMs = Number(d.duration || 0)
  const timestamp = Number(d.time || 0)
  const vw = Number(d.video_width || 0)
  const vh = Number(d.video_height || 0)
  const videoSize = Number(d.video_size || 0)
  const noteType = String(d.type || 'video')
  const title = String(d.title || '')
  const desc = String(d.desc || '')
  const publishTime = timestamp > 0
    ? new Date(timestamp).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai', hour12: false }).replace(/\//g, '-')
    : '无'

  const images = (d.images || []) as string[]
  const isImage = noteType !== 'video' || (!d.video_url && images.length > 0)

  return {
    platform: '小红薯',
    avatar: String(user.avatar || ''),
    author_name: String(user.nickname || ''),
    author_id: String(user.userId || ''),
    author_uid: String(user.userId || ''),
    author_desc: '无',
    home_url: user.userId ? `https://www.xiaohongshu.com/user/profile/${user.userId}` : '无',
    video_id: String(d.noteId || ''),
    video_name: title || desc || '无',
    hash: '',
    video_url: isImage ? images.join('\n') : String(d.video_url || ''),
    video_url_fallbacks: [],
    cover_url: images.length > 0 ? images[0] : '',
    cover_url_fallbacks: [],
    collection_name: '',
    topics: desc || '无',
    publish_time: publishTime,
    publish_timestamp: timestamp,
    video_width: vw,
    video_height: vh,
    duration: durationMs / 1000,
    video_ext: isImage ? '图片' : 'mp4',
    video_resolution: vw && vh ? `${Math.min(vw, vh)}p` : '无',
    video_bitrate: '无',
    video_size: videoSize > 0 ? formatFileSize(videoSize) : '无',
    video_codec: isImage ? '图片' : 'H264',
    cover_width: 0, cover_height: 0, cover_resolution: '无',
    likes: parseCnNumber(String(interact.likedCount || '0')),
    plays: 0,
    shares: parseCnNumber(String(interact.shareCount || '0')),
    comments: parseCnNumber(String(interact.commentCount || '0')),
    favorites: parseCnNumber(String(interact.collectedCount || '0')),
  }
}

function parseXiaohongshuHomepageItem(item: Record<string, unknown>): ParsedVideoInfo {
  const noteId = String(item.noteId || item.note_id || '')
  const title = String(item.title || item.display_title || '')
  const isVideo = item.isVideo === true || String(item.type || '') === 'video'
  const cover = typeof item.cover === 'string' ? item.cover : String((item.cover as Record<string, unknown>)?.url_default || (item.cover as Record<string, unknown>)?.url || '')
  const interactInfo = (item.interact_info || {}) as Record<string, unknown>
  const likes = String(item.likes || interactInfo.liked_count || '0')
  const user = (item.user || {}) as Record<string, unknown>
  const vw = Number(item.width || (item.cover as Record<string, unknown>)?.width || 0)
  const vh = Number(item.height || (item.cover as Record<string, unknown>)?.height || 0)

  return {
    platform: '小红薯',
    avatar: String(item.__author_avatar__ || user.avatar || ''),
    author_name: String(item.__author_name__ || user.nickname || user.nick_name || ''),
    author_id: String(item.__user_id__ || user.user_id || ''),
    author_uid: String(item.__user_id__ || user.user_id || ''),
    author_desc: '无',
    home_url: item.__user_id__ ? `https://www.xiaohongshu.com/user/profile/${item.__user_id__}` : '无',
    video_id: noteId,
    video_name: title || '无',
    hash: '',
    video_url: '',
    video_url_fallbacks: [],
    cover_url: cover,
    cover_url_fallbacks: [],
    collection_name: '',
    topics: '无',
    publish_time: '无',
    publish_timestamp: 0,
    video_width: 0,
    video_height: 0,
    duration: 0,
    video_ext: isVideo ? 'mp4' : '图片',
    video_resolution: vw && vh ? `${Math.min(vw, vh)}p` : '无',
    video_bitrate: '无',
    video_size: '无',
    video_codec: isVideo ? 'H264' : '图片',
    cover_width: 0, cover_height: 0, cover_resolution: '无',
    likes: parseCnNumber(likes),
    plays: 0, shares: 0, comments: 0, favorites: 0,
  }
}

function upsertItem(info: ParsedVideoInfo, tab: string): boolean {
  const existing = allVideos.value.find(item => item.video_id === info.video_id)
  if (existing) {
    Object.assign(existing, info, { _tab: tab })
    return false
  }
  const newItem: VideoItem = {
    ...info,
    id: nextId++,
    _tab: tab,
    _status: 'parsed',
    _progress: 0,
    _error_msg: '',
    _file_path: '',
    _file_size: 0,
    _speed: '',
    _dl_tasks: [],
  }
  allVideos.value.push(newItem)
  selectedParsed.value = [...selectedParsed.value, newItem]
  return true
}

// ── 主解析入口（CDP 模式） ─────────────────────────────────────────

async function startParse() {
  const raw = linkText.value.trim()
  if (!raw) { setHint('请输入至少一个链接'); return }
  const lines = raw.split('\n').map(l => l.trim()).filter(l => l.length > 0)
  if (lines.length === 0) { setHint('请输入至少一个链接'); return }

  // 确认需要的平台有已登录的账号
  const neededPlatforms = new Set<string>()
  for (const line of lines) {
    const p = detectPlatform(extractUrl(line))
    if (p) neededPlatforms.add(p)
  }

  const missing: string[] = []
  for (const p of neededPlatforms) {
    if (p === 'cctv' || p === 'yangshipin') continue
    try { await getPlatformCookiePool(p) }
    catch { missing.push(platformLabelMap[p] || p) }
  }
  if (missing.length > 0) { setHint(`${missing.join('、')}没有在线账号，请先在「账号登记」中登录`); return }

  // Chrome CDP 按需启动（仅在需要 CDP 的操作时才启动）
  let cdpReady = false
  async function lazyEnsureCdp() {
    if (cdpReady) return
    await ensureCdpChrome()
    cdpReady = true
  }

  isParsing.value = true
  parseCancelled.value = false

  // 读取解析内容过滤设置
  let parseContentFilter = 'video_and_image'
  try {
    const all = await invoke<Record<string, string>>('get_all_settings')
    if (all.parse_content) parseContentFilter = all.parse_content
  } catch { /* ignore */ }
  console.log('[解析过滤] parse_content 设置值:', parseContentFilter)

  // 监听 CDP 解析进度和数据事件
  let cdpItemCount = 0
  const unlistenChunk = await listen<{ platform: string; type: string; items: Record<string, unknown>[] }>('cdp-parse-chunk', (event) => {
    const { platform: p, type: t, items } = event.payload
    for (const item of items) {
      // 抖音内容类型过滤：aweme_type=68 为图文，其余为视频
      if (p === 'douyin') {
        const awemeType = Number(item.aweme_type ?? 0)
        const isImagePost = awemeType === 68
        console.log(`[解析过滤] aweme_type=${awemeType}, isImage=${isImagePost}, filter=${parseContentFilter}`)
        if (parseContentFilter === 'video' && isImagePost) continue
        if (parseContentFilter === 'image' && !isImagePost) continue
      }

      let info: ParsedVideoInfo | null = null
      if (p === 'douyin') {
        info = parseDouyinDetail(item)
      } else if (p === 'kuaishou' && t === 'homepage') {
        info = parseKuaishouFeedItem(item)
      } else if (p === 'bilibili' && t === 'homepage') {
        info = parseBilibiliFeedItem(item)
      } else if (p === 'bilibili') {
        info = parseBilibiliDetail(item)
      } else if (p === 'migu' && t === 'homepage') {
        info = parseMiguHomepageItem(item)
      } else if (p === 'cctv' && t === 'homepage') {
        info = parseCctvColumnItem(item)
      } else if (p === 'xiaohongshu' && t === 'homepage') {
        info = parseXiaohongshuHomepageItem(item)
      }
      if (info) {
        if (currentCollectionName) info.collection_name = currentCollectionName
        upsertItem(info, '')
        cdpItemCount++
      }
    }
  })

  const unlistenProgress = await listen<{ message: string }>('cdp-parse-progress', (event) => {
    parseProgress.message = event.payload.message
    setHint(event.payload.message)
  })

  let successCount = 0, failCount = 0, lastErrorMsg = ''
  let currentCollectionName = ''
  parseProgress.current = 0; parseProgress.total = lines.length
  parseProgress.success = 0; parseProgress.failed = 0; parseProgress.message = '准备解析...'

  for (let i = 0; i < lines.length; i++) {
    if (parseCancelled.value) { parseProgress.message = '已取消'; break }
    parseProgress.current = i + 1

    const url = extractUrl(lines[i])
    if (!url) continue
    const platform = detectPlatform(url)
    if (!platform) { parseProgress.message = `第${i + 1}条：无法识别平台`; parseProgress.failed++; failCount++; continue }

    try {
      parseProgress.message = `正在解析第${i + 1}/${lines.length}条...`
      setHint(`正在解析第${i + 1}/${lines.length}条...`)

      // 抖音 PC 端 /user/ 页面带 modal_id 参数的链接，直接提取视频ID走单视频解析
      // 避免 resolve_video_url 重定向时丢失 modal_id 导致误判为主页
      if (platform === 'douyin') {
        const modalIdMatch = url.match(/[?&]modal_id=(\d+)/)
        if (modalIdMatch) {
          const videoId = modalIdMatch[1]
          setHint(`第${i + 1}/${lines.length}条 正在解析抖音视频...`)
          const rawData = await invoke<string>('api_parse_douyin_video', { videoId })
          if (!rawData) throw new Error('API未返回视频数据')
          const detail = JSON.parse(rawData) as Record<string, unknown>
          const mixId = (detail?.mix_info as Record<string, unknown>)?.mix_id as string | undefined
          if (mixId) {
            const mixName = String((detail?.mix_info as Record<string, unknown>)?.mix_name || '合集')
            parseProgress.message = `第${i + 1}/${lines.length}条 检测到合集「${mixName}」，正在加载全部视频...`
            cdpItemCount = 0
            const dyPool3 = await getPlatformCookiePool('douyin')
            await invoke<string>('api_parse_douyin_collection', { mixId, cookies: dyPool3[0].cookies })
            if (cdpItemCount === 0) { upsertItem(parseDouyinDetail(detail), ''); successCount++ }
            else { successCount += cdpItemCount }
          } else {
            upsertItem(parseDouyinDetail(detail), ''); successCount++
          }
          parseProgress.success = successCount
          continue
        }
      }

      const isCompilationUrl = platform === 'douyin' && url.includes('showSubTab=compilation')
      const isCompilationText = platform === 'douyin' && raw.includes('合集')

      const resolvedUrl = await invoke<string>('resolve_video_url', { url, platform })
      const urlType = detectUrlType(resolvedUrl)

      if (platform === 'cctv') {
        if (urlType === 'homepage') {
          parseProgress.message = `第${i + 1}/${lines.length}条 正在加载央视栏目...`
          setHint(`第${i + 1}/${lines.length}条 正在加载央视栏目...`)
          cdpItemCount = 0
          await invoke<string>('api_parse_cctv_column', { pageUrl: resolvedUrl })
          if (cdpItemCount === 0) throw new Error('该栏目未找到视频数据')
          successCount += cdpItemCount
        } else {
          setHint(`第${i + 1}/${lines.length}条 正在解析央视视频...`)
          const rawData = await invoke<string>('api_parse_cctv_video', { pageUrl: resolvedUrl })
          if (!rawData) throw new Error('央视返回空数据')
          const detail = JSON.parse(rawData) as Record<string, unknown>
          const info = parseCctvDetail(detail)
          if (!info.video_url) throw new Error('未能获取到视频播放地址')
          upsertItem(info, ''); successCount++
        }
      } else if (platform === 'yangshipin') {
        const vidMatch = resolvedUrl.match(/vid=([a-zA-Z0-9]+)/)
        const yspVid = vidMatch?.[1]
        if (!yspVid) throw new Error('无法从URL提取央视频视频ID')
        setHint(`第${i + 1}/${lines.length}条 正在解析央视频视频...`)
        await lazyEnsureCdp()
        const rawData = await invoke<string>('api_parse_yangshipin_video', { vid: yspVid })
        if (!rawData) throw new Error('央视频返回空数据')
        const detail = JSON.parse(rawData) as Record<string, unknown>
        const info = parseYangshipinDetail(detail)
        if (!info.video_url) throw new Error('未能获取到视频播放地址')
        upsertItem(info, ''); successCount++
      } else if (urlType === 'homepage' && platform === 'xiaohongshu') {
        const userIdMatch = resolvedUrl.match(/\/user\/profile\/([a-f0-9]+)/)
        const xhsUserId = userIdMatch?.[1]
        if (!xhsUserId) throw new Error('无法从URL提取小红薯用户ID')
        parseProgress.message = `第${i + 1}/${lines.length}条 正在加载小红薯主页...`
        setHint(`第${i + 1}/${lines.length}条 正在加载小红薯主页...`)
        cdpItemCount = 0
        const xhsPool = await getPlatformCookiePool('xiaohongshu')
        await lazyEnsureCdp()
        await invoke<string>('api_parse_xiaohongshu_homepage', { userId: xhsUserId, cookies: xhsPool[0].cookies })
        if (cdpItemCount === 0) throw new Error('该用户主页未找到作品数据')
        successCount += cdpItemCount
      } else if (platform === 'xiaohongshu') {
        setHint(`第${i + 1}/${lines.length}条 正在解析小红薯...`)
        const xhsPool = await getPlatformCookiePool('xiaohongshu')
        const rawData = await invoke<string>('api_parse_xiaohongshu_video', { noteUrl: resolvedUrl, cookies: xhsPool[0].cookies })
        if (!rawData) throw new Error('小红薯返回空数据')
        const detail = JSON.parse(rawData) as Record<string, unknown>
        const info = parseXiaohongshuDetail(detail)
        upsertItem(info, ''); successCount++
      } else if ((urlType === 'compilation' || isCompilationUrl || (isCompilationText && urlType === 'video')) && platform === 'douyin') {
        const isCollectionUrl = resolvedUrl.includes('/collection/')
        const videoId = extractVideoId('douyin', isCompilationUrl ? url : resolvedUrl)
        parseProgress.message = `第${i + 1}/${lines.length}条 正在解析合集...`

        if (isCollectionUrl) {
          const mixIdMatch = resolvedUrl.match(/\/collection\/(\d+)/)
          const mixId = mixIdMatch ? mixIdMatch[1] : videoId
          cdpItemCount = 0
          const dyPool1 = await getPlatformCookiePool('douyin')
          await invoke<string>('api_parse_douyin_collection', { mixId, cookies: dyPool1[0].cookies })
          if (cdpItemCount === 0) throw new Error('合集未找到视频数据')
          successCount += cdpItemCount
        } else {
          // PC 合集 URL：通过 sec_uid + video_id 查找 mix_id（纯 API）
          const secUid = extractSecUid(resolvedUrl)
          const dyPoolFind = await getPlatformCookiePool('douyin')
          parseProgress.message = `第${i + 1}/${lines.length}条 正在查找合集...`
          const findResult = await invoke<string>('api_find_douyin_mix_id', { secUid, videoId, cookies: dyPoolFind[0].cookies })
          const mixInfo = JSON.parse(findResult) as { mix_id: string; mix_name: string }
          cdpItemCount = 0
          currentCollectionName = mixInfo.mix_name || ''
          await invoke<string>('api_parse_douyin_collection', { mixId: mixInfo.mix_id, cookies: dyPoolFind[0].cookies })
          successCount += cdpItemCount > 0 ? cdpItemCount : 1
          currentCollectionName = ''
        }
      } else if (urlType === 'homepage' && platform === 'douyin') {
        const secUid = extractSecUid(resolvedUrl)
        parseProgress.message = `第${i + 1}/${lines.length}条 正在加载主页...`
        setHint(`第${i + 1}/${lines.length}条 正在加载主页...`)
        cdpItemCount = 0
        const dyPool = await getPlatformCookiePool('douyin')
        await invoke<string>('api_parse_douyin_homepage', { secUid, cookies: dyPool[0].cookies })
        if (cdpItemCount === 0) throw new Error('该用户主页未找到视频数据')
        successCount += cdpItemCount
      } else if (urlType === 'homepage' && platform === 'kuaishou') {
        const m = resolvedUrl.match(/\/profile\/([^/?#]+)/)
        const userId = m?.[1]
        if (!userId) throw new Error('无法从 URL 中提取快手用户ID')
        if (resolvedUrl.includes('live.kuaishou.com') && !userId.startsWith('3x')) {
          throw new Error('暂不支持快手手机主页链接，请使用电脑版主页链接（www.kuaishou.com/profile/...）')
        }
        parseProgress.message = `第${i + 1}/${lines.length}条 正在加载主页...`
        setHint(`第${i + 1}/${lines.length}条 正在加载快手主页...`)
        cdpItemCount = 0
        const ksPool = await getPlatformCookiePool('kuaishou')
        await invoke<string>('api_parse_kuaishou_homepage', { userId, cookies: ksPool[0].cookies })
        if (cdpItemCount === 0) throw new Error('该用户主页未找到视频数据')
        successCount += cdpItemCount
      } else if (platform === 'kuaishou') {
        const photoId = extractVideoId('kuaishou', resolvedUrl)
        setHint(`第${i + 1}/${lines.length}条 正在解析快手视频...`)
        const ksPool = await getPlatformCookiePool('kuaishou')
        const rawData = await invoke<string>('api_parse_kuaishou_video', { photoId, cookies: ksPool[0].cookies })
        if (!rawData) throw new Error('API未返回视频数据')
        const apolloState = JSON.parse(rawData) as Record<string, unknown>
        if ((apolloState as Record<string, string>).error) throw new Error((apolloState as Record<string, string>).error)
        const info = parseKuaishouDetail(apolloState, photoId)
        if (!info.video_url || info.video_url === '无') throw new Error('未能获取到视频地址')
        upsertItem(info, ''); successCount++
      } else if (urlType === 'homepage' && platform === 'bilibili') {
        const biliMid = extractBilibiliMid(resolvedUrl)
        parseProgress.message = `第${i + 1}/${lines.length}条 正在加载主页...`
        setHint(`第${i + 1}/${lines.length}条 正在加载B站主页...`)
        cdpItemCount = 0
        const biliPool = await getPlatformCookiePool('bilibili')
        await invoke<string>('fetch_bilibili_homepage', { app: undefined, mid: biliMid, cookies: biliPool[0].cookies })
        if (cdpItemCount === 0) throw new Error('该用户主页未找到视频数据')
        successCount += cdpItemCount
      } else if (urlType === 'homepage' && platform === 'migu') {
        const authorIdMatch = resolvedUrl.match(/authorId=(\d+)/)
        const authorId = authorIdMatch?.[1]
        if (!authorId) throw new Error('无法从 URL 中提取咪咕作者ID')
        parseProgress.message = `第${i + 1}/${lines.length}条 正在加载咪咕主页...`
        setHint(`第${i + 1}/${lines.length}条 正在加载咪咕主页...`)
        cdpItemCount = 0
        const miguPool = await getPlatformCookiePool('migu')
        await invoke<string>('api_parse_migu_homepage', { authorId, cookies: miguPool[0].cookies })
        if (cdpItemCount === 0) throw new Error('该用户主页未找到视频数据')
        successCount += cdpItemCount
      } else if (platform === 'bilibili') {
        const bvid = extractBilibiliBvid(resolvedUrl)
        setHint(`第${i + 1}/${lines.length}条 正在解析B站视频...`)
        const biliPool = await getPlatformCookiePool('bilibili')
        const rawData = await invoke<string>('fetch_bilibili_video', { bvid, cookies: biliPool[0].cookies })
        if (!rawData) throw new Error('B站返回空数据')
        const detail = JSON.parse(rawData) as Record<string, unknown>
        upsertItem(parseBilibiliDetail(detail), ''); successCount++
      } else if (platform === 'migu') {
        const contentId = extractVideoId('migu', resolvedUrl)
        setHint(`第${i + 1}/${lines.length}条 正在解析咪咕视频...`)
        const miguPool = await getPlatformCookiePool('migu')
        await lazyEnsureCdp()
        const rawData = await invoke<string>('api_parse_migu_video', { contentId, cookies: miguPool[0].cookies })
        if (!rawData) throw new Error('咪咕返回空数据')
        const detail = JSON.parse(rawData) as Record<string, unknown>
        const info = parseMiguDetail(detail)
        if (!info.video_url) throw new Error('未能获取到视频播放地址')
        upsertItem(info, ''); successCount++
      } else if (platform === 'douyin') {
        const videoId = extractVideoId(platform, resolvedUrl)
        setHint(`第${i + 1}/${lines.length}条 正在解析抖音视频...`)
        const rawData = await invoke<string>('api_parse_douyin_video', { videoId })
        if (!rawData) throw new Error('API未返回视频数据')
        const detail = JSON.parse(rawData) as Record<string, unknown>

        const mixId = (detail?.mix_info as Record<string, unknown>)?.mix_id as string | undefined
        if (mixId) {
          const mixName = String((detail?.mix_info as Record<string, unknown>)?.mix_name || '合集')
          parseProgress.message = `第${i + 1}/${lines.length}条 检测到合集「${mixName}」，正在加载全部视频...`
          cdpItemCount = 0
          const dyPool3 = await getPlatformCookiePool('douyin')
          await invoke<string>('api_parse_douyin_collection', { mixId, cookies: dyPool3[0].cookies })
          if (cdpItemCount === 0) { upsertItem(parseDouyinDetail(detail), ''); successCount++ }
          else { successCount += cdpItemCount }
        } else {
          upsertItem(parseDouyinDetail(detail), ''); successCount++
        }
      }
      parseProgress.success = successCount
    } catch (e: unknown) {
      const msg = typeof e === 'string' ? e : (e as { message?: string })?.message || '解析失败'
      lastErrorMsg = `第${i + 1}条：${msg}`
      parseProgress.message = `第${i + 1}条解析失败：${msg}`
      setHint(`第${i + 1}条解析失败：${msg}`, true)
      failCount++
      parseProgress.failed = failCount
      await new Promise(r => setTimeout(r, 500))
    }
  }

  isParsing.value = false
  parseProgress.current = 0; parseProgress.total = 0
  unlistenChunk()
  unlistenProgress()
  const cancelMsg = parseCancelled.value ? '（已取消）' : ''
  if (failCount > 0 && lastErrorMsg) {
    setHint(`解析完成${cancelMsg}：成功 ${successCount}，失败 ${failCount}。${lastErrorMsg}`, true)
  } else {
    setHint(`解析完成${cancelMsg}：成功 ${successCount}，失败 ${failCount}`)
  }
  if (successCount > 0) linkText.value = ''
  parseCancelled.value = false
  activeTab.value = 0
}

function cancelParse() { parseCancelled.value = true }

// ── 下载逻辑 ──────────────────────────────────────────────────────

async function loadDlDefaults() {
  try {
    const all = await invoke<Record<string, string>>('get_all_settings')
    if (all.download_content === 'video') { dlOptions.video = true; dlOptions.cover = false }
    else if (all.download_content === 'cover') { dlOptions.video = false; dlOptions.cover = true }
    else { dlOptions.video = true; dlOptions.cover = true }
    if (all.concurrent) dlOptions.concurrent = parseInt(all.concurrent) || 5
    if (all.add_seq !== undefined) dlOptions.add_seq = all.add_seq === 'true'
    if (all.remove_topics !== undefined) dlOptions.remove_topics = all.remove_topics === 'true'
    if (all.remove_at !== undefined) dlOptions.remove_at = all.remove_at === 'true'

    if (all.output_dir) {
      dlOptions.output_dir = all.output_dir
    } else if (!dlOptions.output_dir) {
      dlOptions.output_dir = await invoke<string>('get_download_dir')
    }
  } catch { /* ignore */ }
}


async function startBatchDownload() {
  const selected = selectedParsed.value.length > 0 ? selectedParsed.value : parsedVideos.value
  if (selected.length === 0) { setHint('没有可下载的视频'); return }

  await loadDlDefaults()
  if (!dlOptions.output_dir) { setHint('请先在「下载设置」中配置导出目录'); return }
  if (!dlOptions.video && !dlOptions.cover) { setHint('请先在「下载设置」中选择下载内容'); return }

  const groups = new Map<string, typeof selected>()
  for (const item of selected) {
    const key = item.author_name || '_未知作者'
    if (!groups.has(key)) groups.set(key, [])
    groups.get(key)!.push(item)
  }

  const tasks: BatchDownloadTask[] = []
  const taskIdToItemId = new Map<string, number>()
  const sep = '\\'

  for (const [author, items] of groups) {
    const baseAuthorDir = dlOptions.output_dir + sep + sanitizeFilename(author)
    for (let idx = 0; idx < items.length; idx++) {
      const item = items[idx]
      const authorDir = item.collection_name
        ? baseAuthorDir + sep + sanitizeFilename(item.collection_name)
        : baseAuthorDir
      let rawName = (item.video_name && item.video_name !== '无') ? item.video_name : author
      if (dlOptions.remove_topics) rawName = rawName.replace(/#[^\s#@]+/g, '')
      if (dlOptions.remove_at) rawName = rawName.replace(/@[^\s#@]+/g, '')
      let baseName = sanitizeFilename(rawName.replace(/\s+/g, ' ').trim())
      if (baseName.length > 80) baseName = baseName.substring(0, 80)
      const seqPrefix = dlOptions.add_seq ? `${idx + 1}.` : ''

      const isImagePost = item.video_codec && item.video_codec.startsWith('图片')
      if (isImagePost && item.video_url && item.video_url !== '无') {
        const imageUrls = item.video_url.split('\n').filter(Boolean)
        const folderPath = `${authorDir}${sep}${seqPrefix}${baseName}`
        for (let imgIdx = 0; imgIdx < imageUrls.length; imgIdx++) {
          const imgUrl = imageUrls[imgIdx]
          const imgExt = imgUrl.includes('.png') ? 'png' : 'jpg'
          const tid = `img_${item.id}_${imgIdx}`
          tasks.push({ url: imgUrl, save_path: `${folderPath}${sep}${imgIdx + 1}.${baseName}.${imgExt}`, task_id: tid, fallback_urls: [] })
          taskIdToItemId.set(tid, item.id)
        }
      } else {
        if (dlOptions.video) {
          let videoUrl = item.video_url && item.video_url !== '无' ? item.video_url : ''
          if (!videoUrl && item.platform === 'MG' && item.video_id) {
            videoUrl = `migu://resolve/${item.video_id}`
          }
          if (!videoUrl && item.platform === '小红薯' && item.video_id && item.video_codec !== '图片') {
            videoUrl = `xhs://resolve/${item.video_id}`
          }
          if (videoUrl) {
            const ext = item.video_ext && item.video_ext !== '无' ? item.video_ext : 'mp4'
            const tid = `v_${item.id}`
            tasks.push({ url: videoUrl, save_path: `${authorDir}${sep}${seqPrefix}${baseName}.${ext}`, task_id: tid, fallback_urls: item.video_url_fallbacks || [] })
            taskIdToItemId.set(tid, item.id)
          }
        }
        if (dlOptions.cover && item.cover_url && item.cover_url !== '无') {
          const coverExt = 'jpg'
          const tid = `c_${item.id}`
          tasks.push({ url: item.cover_url, save_path: `${authorDir}${sep}${seqPrefix}${baseName}.${coverExt}`, task_id: tid, fallback_urls: item.cover_url_fallbacks || [] })
          taskIdToItemId.set(tid, item.id)
        }
      }
    }
  }

  if (tasks.length === 0) { setHint('没有可下载的链接'); return }

  // 保存下载任务到每个 item，重试时复用
  for (const item of selected) {
    item._dl_tasks = tasks.filter(t => taskIdToItemId.get(t.task_id) === item.id)
  }

  // 将选中的项移入下载队列（累加到已有进度上）
  for (const item of selected) { item._status = 'queued'; item._progress = 0; item._speed = '' }
  selectedParsed.value = []
  dlProgress.total += tasks.length
  dlSubmitting.value++
  activeTab.value = 1
  startSpeedTracker()

  interface DlEvent { task_id: string; status: string; completed: number; total: number; error?: string; bytes: number }
  const doneTaskIds = new Set<string>()
  const failedIds = new Set<string>()

  const unlistenFileProgress = await listen<{ task_id: string; downloaded: number; total: number }>('download-file-progress', (event) => {
    const { task_id, downloaded, total: fileTotal } = event.payload
    const itemId = taskIdToItemId.get(task_id)
    if (!itemId) return
    const item = allVideos.value.find(v => v.id === itemId)
    if (!item) return
    item._status = 'downloading'
    item._progress = fileTotal > 0 ? Math.round(downloaded / fileTotal * 100) : 0
    item._file_size = fileTotal
  })

  const unlisten = await listen<DlEvent>('batch-download-progress', (event) => {
    const { task_id, status, bytes } = event.payload

    if (!taskIdToItemId.has(task_id)) return
    if (doneTaskIds.has(task_id)) return
    doneTaskIds.add(task_id)

    dlProgress.completed++
    dlProgress.bytes = bytes

    const itemId = taskIdToItemId.get(task_id)!
    const item = allVideos.value.find(v => v.id === itemId)
    if (!item) return

    if (status === 'error') {
      failedIds.add(task_id)
    }

    const allTasksForItem = [...taskIdToItemId.entries()].filter(([, id]) => id === itemId).map(([tid]) => tid)
    const allDone = allTasksForItem.every(tid => doneTaskIds.has(tid))
    if (allDone) {
      const hasFail = allTasksForItem.some(tid => failedIds.has(tid))
      item._status = hasFail ? 'failed' : 'completed'
      if (hasFail) item._error_msg = event.payload.error || '下载失败'
      else item._progress = 100
    } else {
      item._status = 'downloading'
    }
  })

  try {
    const rawResult = await invoke<string>('batch_download_videos', { tasks, concurrent: dlOptions.concurrent })
    const result = JSON.parse(rawResult) as { total: number; success: number; failed: number; skipped: number }
    const msgParts = [`本批下载完成: 成功 ${result.success}`]
    if (result.skipped > 0) msgParts.push(`跳过 ${result.skipped}`)
    if (result.failed > 0) msgParts.push(`失败 ${result.failed}`)
    setHint(msgParts.join('，'))

    // 兜底：确保本批次所有项都有最终状态
    for (const [, itemId] of taskIdToItemId) {
      const item = allVideos.value.find(v => v.id === itemId)
      if (item && (item._status === 'queued' || item._status === 'downloading')) {
        const hasFail = failedIds.has(`v_${item.id}`) || failedIds.has(`c_${item.id}`)
        item._status = hasFail ? 'failed' : 'completed'
        if (hasFail) item._error_msg = '下载失败'
      }
    }
  } catch (e: unknown) {
    setHint(`下载失败: ${(e as { message?: string })?.message || e}`)
    for (const [, itemId] of taskIdToItemId) {
      const item = allVideos.value.find(v => v.id === itemId)
      if (item && (item._status === 'queued' || item._status === 'downloading')) {
        item._status = 'failed'; item._error_msg = '下载中断'
      }
    }
  } finally {
    unlisten()
    unlistenFileProgress()
    dlSubmitting.value--
  }
}

// ── UI 操作 ───────────────────────────────────────────────────────

async function retryFailedItems(items: VideoItem[]) {
  const tasks: BatchDownloadTask[] = []
  const taskIdToItemId = new Map<string, number>()

  for (const item of items) {
    if (item._dl_tasks && item._dl_tasks.length > 0) {
      for (const t of item._dl_tasks) {
        tasks.push(t)
        taskIdToItemId.set(t.task_id, item.id)
      }
    }
  }

  if (tasks.length === 0) { setHint('没有可重试的任务'); return }

  await loadDlDefaults()
  for (const item of items) { item._status = 'queued'; item._progress = 0; item._error_msg = '' }
  dlProgress.total += tasks.length
  dlSubmitting.value++
  activeTab.value = 1
  startSpeedTracker()

  interface DlEvent { task_id: string; status: string; completed: number; total: number; error?: string; bytes: number }
  const doneTaskIds = new Set<string>()
  const failedIds = new Set<string>()

  const unlistenFileProgress = await listen<{ task_id: string; downloaded: number; total: number }>('download-file-progress', (event) => {
    const { task_id, downloaded, total: fileTotal } = event.payload
    const itemId = taskIdToItemId.get(task_id)
    if (!itemId) return
    const item = allVideos.value.find(v => v.id === itemId)
    if (!item) return
    item._status = 'downloading'
    item._progress = fileTotal > 0 ? Math.round(downloaded / fileTotal * 100) : 0
    item._file_size = fileTotal
  })

  const unlisten = await listen<DlEvent>('batch-download-progress', (event) => {
    const { task_id, status, bytes } = event.payload
    if (!taskIdToItemId.has(task_id)) return
    if (doneTaskIds.has(task_id)) return
    doneTaskIds.add(task_id)
    dlProgress.completed++
    dlProgress.bytes = bytes

    const itemId = taskIdToItemId.get(task_id)!
    const item = allVideos.value.find(v => v.id === itemId)
    if (!item) return
    if (status === 'error') failedIds.add(task_id)
    const allTasks = item._dl_tasks.map(t => t.task_id)
    const allDone = allTasks.every(tid => doneTaskIds.has(tid))
    if (allDone) {
      const hasFail = allTasks.some(tid => failedIds.has(tid))
      item._status = hasFail ? 'failed' : 'completed'
      if (hasFail) item._error_msg = event.payload.error || '下载失败'
      else item._progress = 100
    } else {
      item._status = 'downloading'
    }
  })

  try {
    await invoke<string>('batch_download_videos', { tasks, concurrent: dlOptions.concurrent })
  } catch (e: unknown) {
    for (const item of items) {
      if (item._status === 'queued' || item._status === 'downloading') { item._status = 'failed'; item._error_msg = '下载中断' }
    }
  } finally { unlisten(); unlistenFileProgress(); dlSubmitting.value-- }
}

function retryFailed(item: VideoItem) { retryFailedItems([item]) }
function retryAllFailed() { retryFailedItems([...failedVideos.value]) }
function removeFailed(item: VideoItem) {
  const idx = allVideos.value.findIndex(v => v.id === item.id)
  if (idx !== -1) allVideos.value.splice(idx, 1)
}

const clearDialogVisible = ref(false)

function clearHistory() {
  clearDialogVisible.value = true
}

function doClearHistory() {
  allVideos.value = []; selectedParsed.value = []
  clearDialogVisible.value = false
}

// KeepAlive 重新激活时，触发 resize 让虚拟滚动器重新计算可视区域
onActivated(() => {
  nextTick(() => {
    window.dispatchEvent(new Event('resize'))
  })
})
</script>

<template>
  <Dialog v-model:visible="clearDialogVisible" header="清空历史记录" :modal="true" :style="{ width: '420px' }">
    <div style="display: flex; align-items: center; gap: 12px;">
      <i class="pi pi-exclamation-triangle" style="font-size: 1.5rem; color: var(--p-yellow-500);" />
      <span>确定要清空所有历史记录吗？此操作不可撤销。</span>
    </div>
    <template #footer>
      <Button label="取消" severity="secondary" @click="clearDialogVisible = false" />
      <Button label="确定清空" severity="danger" @click="doClearHistory" />
    </template>
  </Dialog>
  <div class="vd-page">

    <!-- 链接输入区 -->
    <section class="vd-input-section">
      <div class="vd-input-header">
        <span class="vd-input-title">
          <i class="pi pi-link"></i>
          粘贴视频链接
        </span>
        <span class="vd-input-hint">一行一个，支持多平台分享链接和主页链接</span>
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
          <Button
            v-if="isParsing"
            label="取消"
            icon="pi pi-times"
            severity="secondary"
            size="small"
            @click="cancelParse"
          />
          <Transition name="fade">
            <div v-if="isParsing" class="vd-progress-info">
              <ProgressBar
                :value="parseProgress.total ? Math.round(parseProgress.current / parseProgress.total * 100) : 0"
                :showValue="false"
                class="vd-progress-bar"
              />
              <span class="vd-progress-text">{{ parseProgress.message || `已解析 ${parseProgress.current} / ${parseProgress.total} 条` }}</span>
            </div>
          </Transition>
        </div>
        <span v-if="hintMessage && !isParsing" class="vd-hint-text" :class="{ 'vd-hint-error': hintIsError }">{{ hintMessage }}</span>
      </div>
    </section>

    <!-- 标签页区域 -->
    <section class="vd-tabs-section">
      <TabView v-model:activeIndex="activeTab" class="vd-tabview">
        <!-- 解析中 -->
        <TabPanel value="0">
          <template #header>
            <span class="vd-tab-header vd-tab--parse">
              <i class="pi pi-list"></i>
              解析中
              <span v-if="tabCounts.parsed > 0" class="vd-tab-badge vd-badge-orange">{{ tabCounts.parsed }}</span>
            </span>
          </template>

          <div class="vd-tab-toolbar">
            <div class="vd-toolbar-left">
              <span v-if="selectedParsed.length > 0" class="vd-selected-info">已选 {{ selectedParsed.length }} 项</span>
            </div>
            <div class="vd-toolbar-right">
              <Button label="筛选" icon="pi pi-filter" size="small" class="vd-btn-orange" :disabled="parsedVideos.length === 0" @click="filterDialogVisible = true" />
              <Button label="下载选中" icon="pi pi-download" size="small" :disabled="selectedParsed.length === 0 || isParsing" @click="startBatchDownload" />
            </div>
          </div>

          <DataTable v-model:selection="selectedParsed" :value="parsedVideos" dataKey="id" scrollable scrollHeight="flex" :virtualScrollerOptions="{ itemSize: 52 }" class="vd-table vd-table-grid" size="small" stripedRows>
            <Column selectionMode="multiple" headerStyle="width: 3rem" />
            <Column header="#" headerStyle="width: 3.5rem"><template #body="{ index }">{{ index + 1 }}</template></Column>
            <Column field="platform" header="平台" headerStyle="width: 4.5rem">
              <template #body="{ data }"><Tag :value="data.platform" :class="['vd-plat-tag', platformClassMap[data.platform] || '']" /></template>
            </Column>
            <Column field="author_name" header="作者" headerStyle="width: 7rem">
              <template #body="{ data }"><span class="vd-author-tag" :style="{ background: getAuthorColor(data.author_name) + '18', color: getAuthorColor(data.author_name), borderColor: getAuthorColor(data.author_name) + '40' }">{{ data.author_name || '未知' }}</span></template>
            </Column>
            <Column field="video_name" header="标题" style="min-width: 120px; max-width: 320px">
              <template #body="{ data }">
                <div class="vd-cell-title" @mouseenter="showTooltip($event, data.video_name)" @mouseleave="hideTooltip">
                  <img v-if="data.cover_url && data.cover_url !== '无'" :src="data.cover_url" class="vd-cover" alt="" referrerpolicy="no-referrer" />
                  <div v-else class="vd-cover-placeholder"><i class="pi pi-video"></i></div>
                  <span class="vd-title-text">{{ data.video_name || '未知标题' }}</span>
                </div>
              </template>
            </Column>
            <Column field="duration" header="时长" headerStyle="width: 5rem"><template #body="{ data }">{{ formatDuration(data.duration) }}</template></Column>
            <Column field="publish_time" header="日期" headerStyle="width: 9rem" />
            <Column field="likes" header="点赞" headerStyle="width: 5.5rem"><template #body="{ data }">{{ formatCount(data.likes) }}</template></Column>
            <!-- <Column field="video_id" header="视频ID" headerStyle="width: 10rem"><template #body="{ data }"><span style="font-size: 0.75rem; color: #999; user-select: all">{{ data.video_id }}</span></template></Column> -->
          </DataTable>
        </TabPanel>

        <!-- 下载中 -->
        <TabPanel value="1">
          <template #header>
            <span class="vd-tab-header vd-tab--download">
              <i class="pi pi-cloud-download"></i>
              下载中
              <span v-if="tabCounts.downloading > 0" class="vd-tab-badge vd-badge-blue">{{ tabCounts.downloading }}</span>
            </span>
          </template>

          <div v-if="dlSubmitting > 0" class="vd-dl-overall-progress">
            <ProgressBar :value="dlProgress.total ? Math.round(dlProgress.completed / dlProgress.total * 100) : 0" :showValue="false" class="vd-progress-bar" />
            <span class="vd-progress-text">
              已完成 {{ dlProgress.completed }} / {{ dlProgress.total }} 个文件，剩余 {{ dlProgress.total - dlProgress.completed }} 个
              <span v-if="dlSpeed" class="vd-speed-badge"> · {{ dlSpeed }}</span>
            </span>
          </div>

          <DataTable :value="downloadingVideos" dataKey="id" scrollable scrollHeight="flex" :virtualScrollerOptions="{ itemSize: 52 }" class="vd-table vd-table-grid" size="small" stripedRows>
            <Column header="#" headerStyle="width: 3.5rem"><template #body="{ index }">{{ index + 1 }}</template></Column>
            <Column field="platform" header="平台" headerStyle="width: 4.5rem">
              <template #body="{ data }"><Tag :value="data.platform" :class="['vd-plat-tag', platformClassMap[data.platform] || '']" /></template>
            </Column>
            <Column field="video_name" header="标题" style="min-width: 120px; max-width: 320px">
              <template #body="{ data }">
                <div class="vd-cell-title" @mouseenter="showTooltip($event, data.video_name)" @mouseleave="hideTooltip">
                  <img v-if="data.cover_url && data.cover_url !== '无'" :src="data.cover_url" class="vd-cover" alt="" referrerpolicy="no-referrer" />
                  <div v-else class="vd-cover-placeholder"><i class="pi pi-video"></i></div>
                  <span class="vd-title-text">{{ data.video_name || '未知标题' }}</span>
                </div>
              </template>
            </Column>
            <Column field="author_name" header="作者" headerStyle="width: 7rem">
              <template #body="{ data }"><span class="vd-author-tag" :style="{ background: getAuthorColor(data.author_name) + '18', color: getAuthorColor(data.author_name), borderColor: getAuthorColor(data.author_name) + '40' }">{{ data.author_name || '未知' }}</span></template>
            </Column>
            <Column header="进度" headerStyle="width: 10rem">
              <template #body="{ data }">
                <span v-if="data._status === 'queued'" style="color: #999; font-size: 12px">排队中</span>
                <div v-else class="vd-file-progress">
                  <div class="vd-file-progress-bar">
                    <div class="vd-file-progress-fill" :style="{ width: (data._progress || 0) + '%' }"></div>
                  </div>
                  <span class="vd-file-progress-text">{{ data._progress || 0 }}%</span>
                </div>
              </template>
            </Column>
          </DataTable>
        </TabPanel>

        <!-- 下载失败 -->
        <TabPanel value="2">
          <template #header>
            <span class="vd-tab-header vd-tab--failed">
              <i class="pi pi-exclamation-circle"></i>
              下载失败
              <span v-if="tabCounts.failed > 0" class="vd-tab-badge vd-badge-red">{{ tabCounts.failed }}</span>
            </span>
          </template>

          <div class="vd-tab-toolbar">
            <div class="vd-toolbar-left"></div>
            <div class="vd-toolbar-right">
              <Button label="全部重试" icon="pi pi-refresh" size="small" severity="warn" :disabled="failedVideos.length === 0" @click="retryAllFailed" />
            </div>
          </div>

          <DataTable :value="failedVideos" dataKey="id" scrollable scrollHeight="flex" :virtualScrollerOptions="{ itemSize: 52 }" class="vd-table vd-table-grid" size="small" stripedRows>
            <Column header="#" headerStyle="width: 3.5rem"><template #body="{ index }">{{ index + 1 }}</template></Column>
            <Column field="video_name" header="标题" style="min-width: 120px; max-width: 320px">
              <template #body="{ data }">
                <div class="vd-cell-title" @mouseenter="showTooltip($event, data.video_name)" @mouseleave="hideTooltip">
                  <img v-if="data.cover_url && data.cover_url !== '无'" :src="data.cover_url" class="vd-cover" alt="" referrerpolicy="no-referrer" />
                  <div v-else class="vd-cover-placeholder"><i class="pi pi-video"></i></div>
                  <span class="vd-title-text">{{ data.video_name || '未知标题' }}</span>
                </div>
              </template>
            </Column>
            <Column field="author_name" header="作者" headerStyle="width: 7rem">
              <template #body="{ data }"><span class="vd-author-tag" :style="{ background: getAuthorColor(data.author_name) + '18', color: getAuthorColor(data.author_name), borderColor: getAuthorColor(data.author_name) + '40' }">{{ data.author_name || '未知' }}</span></template>
            </Column>
            <Column field="_error_msg" header="失败原因" style="min-width: 120px; max-width: 260px">
              <template #body="{ data }"><span class="vd-error-text" style="word-break: break-all; white-space: normal;">{{ data._error_msg || '未知错误' }}</span></template>
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
            <span class="vd-tab-header vd-tab--done">
              <i class="pi pi-check-circle"></i>
              已完成
              <span v-if="tabCounts.completed > 0" class="vd-tab-badge vd-badge-green">{{ tabCounts.completed }}</span>
            </span>
          </template>

          <DataTable :value="completedVideos" dataKey="id" scrollable scrollHeight="flex" :virtualScrollerOptions="{ itemSize: 52 }" class="vd-table vd-table-grid" size="small" stripedRows>
            <Column header="#" headerStyle="width: 3.5rem"><template #body="{ index }">{{ index + 1 }}</template></Column>
            <Column field="platform" header="平台" headerStyle="width: 4.5rem">
              <template #body="{ data }"><Tag :value="data.platform" :class="['vd-plat-tag', platformClassMap[data.platform] || '']" /></template>
            </Column>
            <Column field="video_name" header="标题" style="min-width: 120px; max-width: 320px">
              <template #body="{ data }">
                <div class="vd-cell-title" @mouseenter="showTooltip($event, data.video_name)" @mouseleave="hideTooltip">
                  <img v-if="data.cover_url && data.cover_url !== '无'" :src="data.cover_url" class="vd-cover" alt="" referrerpolicy="no-referrer" />
                  <div v-else class="vd-cover-placeholder"><i class="pi pi-video"></i></div>
                  <span class="vd-title-text">{{ data.video_name || '未知标题' }}</span>
                </div>
              </template>
            </Column>
            <Column field="author_name" header="作者" headerStyle="width: 7rem">
              <template #body="{ data }"><span class="vd-author-tag" :style="{ background: getAuthorColor(data.author_name) + '18', color: getAuthorColor(data.author_name), borderColor: getAuthorColor(data.author_name) + '40' }">{{ data.author_name || '未知' }}</span></template>
            </Column>
            <Column header="文件大小" headerStyle="width: 6rem">
              <template #body="{ data }">{{ data.video_size || data.video_codec || '--' }}</template>
            </Column>
            <Column field="publish_time" header="发布时间" headerStyle="width: 10rem" />
          </DataTable>
        </TabPanel>
      </TabView>

      <!-- 底部操作栏 -->
      <div class="vd-bottom-bar">
        <div class="vd-bottom-left">
          <span class="vd-total-info">共 {{ activeTab === 0 ? tabCounts.parsed : activeTab === 1 ? tabCounts.downloading : activeTab === 2 ? tabCounts.failed : tabCounts.completed }} 条记录</span>
        </div>
        <div class="vd-bottom-right">
          <Button label="清空历史记录" icon="pi pi-trash" size="small" severity="danger" text :disabled="allVideos.length === 0" @click="clearHistory" />
        </div>
      </div>
    </section>

    <!-- 黑色 tooltip -->
    <Teleport to="body">
      <div v-if="tooltip.visible" class="vd-tooltip" :style="{ left: tooltip.x + 'px', top: tooltip.y + 'px' }" @mouseenter="tooltip.visible = true" @mouseleave="hideTooltip">
        {{ tooltip.text }}
      </div>
    </Teleport>

    <!-- 筛选对话框 -->
    <Dialog v-model:visible="filterDialogVisible" header="筛选视频" :modal="true" :style="{ width: '420px' }">
      <div class="vd-filter-form">
        <div class="vd-filter-row">
          <label>时长（秒）</label>
          <div class="vd-filter-range">
            <InputText v-model="filterOptions.min_duration" placeholder="最小" size="small" style="width: 100px" />
            <span>~</span>
            <InputText v-model="filterOptions.max_duration" placeholder="最大" size="small" style="width: 100px" />
          </div>
        </div>
        <div class="vd-filter-row">
          <label>点赞数</label>
          <div class="vd-filter-range">
            <InputText v-model="filterOptions.min_likes" placeholder="最小" size="small" style="width: 100px" />
            <span>~</span>
            <InputText v-model="filterOptions.max_likes" placeholder="最大" size="small" style="width: 100px" />
          </div>
        </div>
        <div class="vd-filter-row">
          <label>发布日期</label>
          <div class="vd-filter-range">
            <InputText v-model="filterOptions.date_from" placeholder="起始 如 2025-01-01" size="small" style="width: 130px" />
            <span>~</span>
            <InputText v-model="filterOptions.date_to" placeholder="结束 如 2026-12-31" size="small" style="width: 130px" />
          </div>
        </div>
      </div>
      <template #footer>
        <Button label="清除筛选" severity="secondary" size="small" @click="resetFilter" />
        <Button label="筛选并选中" icon="pi pi-check" size="small" class="vd-btn-orange" @click="applyFilter" />
      </template>
    </Dialog>

    <!-- 下载对话框 -->
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

.vd-input-title i { color: var(--app-primary, #22c55e); }
.vd-input-hint { font-size: 0.78rem; color: #94a3b8; }

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

.vd-action-left { display: flex; align-items: center; gap: 10px; flex: 1; }
.vd-parse-btn { flex-shrink: 0; }
.vd-progress-info { display: flex; align-items: center; gap: 10px; flex: 1; max-width: 400px; }
.vd-progress-bar { flex: 1; height: 8px; }
.vd-progress-text { font-size: 0.78rem; color: #64748b; white-space: nowrap; }
.vd-hint-text { font-size: 0.78rem; color: #94a3b8; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 600px; }
.vd-hint-error { color: #ef4444; font-weight: 600; }

/* ═══ 标签页 ═══ */
.vd-tabs-section {
  flex: 1; min-height: 0; display: flex; flex-direction: column;
  background: #fff; border-radius: 14px; box-shadow: 0 2px 12px rgba(15, 23, 42, 0.06); overflow: hidden;
}
.vd-tabview { flex: 1; display: flex; flex-direction: column; min-height: 0; }
:deep(.p-tabview-panels) { flex: 1; min-height: 0; padding: 0; }
:deep(.p-tabview-panel) { height: 100%; display: flex; flex-direction: column; padding: 0; }
:deep(.p-tabview-nav) { border-bottom: 1px solid #e2e8f0; padding: 0 12px; }
:deep(.p-tabview-nav li .p-tabview-nav-link) { padding: 10px 16px; font-size: 0.85rem; }

.vd-tab-header { display: flex; align-items: center; gap: 6px; }
.vd-tab-badge { display: inline-flex; align-items: center; justify-content: center; min-width: 20px; height: 18px; padding: 0 5px; border-radius: 9px; font-size: 0.7rem; font-weight: 600; background: #e2e8f0; color: #475569; }
.vd-badge-orange { background: #fff7ed; color: #ea580c; }
.vd-badge-blue { background: #dbeafe; color: #2563eb; }
.vd-badge-red { background: #fee2e2; color: #dc2626; }
.vd-badge-green { background: #dcfce7; color: #16a34a; }

.vd-tab--parse i { color: #ea580c; }
.vd-tab--download i { color: #2563eb; }
.vd-tab--failed i { color: #dc2626; }
.vd-tab--done i { color: #16a34a; }

:deep(.p-tabview-nav li.p-tabview-selected .vd-tab--parse) { color: #ea580c; }
:deep(.p-tabview-nav li.p-tabview-selected .vd-tab--download) { color: #2563eb; }
:deep(.p-tabview-nav li.p-tabview-selected .vd-tab--failed) { color: #dc2626; }
:deep(.p-tabview-nav li.p-tabview-selected .vd-tab--done) { color: #16a34a; }

:deep(.p-tabview-nav li.p-highlight:has(.vd-tab--parse) .p-tabview-nav-link) { border-color: #ea580c; }
:deep(.p-tabview-nav li.p-highlight:has(.vd-tab--download) .p-tabview-nav-link) { border-color: #2563eb; }
:deep(.p-tabview-nav li.p-highlight:has(.vd-tab--failed) .p-tabview-nav-link) { border-color: #dc2626; }
:deep(.p-tabview-nav li.p-highlight:has(.vd-tab--done) .p-tabview-nav-link) { border-color: #16a34a; }

/* ═══ 工具栏 ═══ */
.vd-tab-toolbar { display: flex; align-items: center; justify-content: space-between; padding: 8px 14px; border-bottom: 1px solid #f1f5f9; flex-shrink: 0; }
.vd-toolbar-right { display: flex; gap: 8px; }
.vd-selected-info { font-size: 0.8rem; color: #64748b; }

/* ═══ 表格 ═══ */
.vd-table { flex: 1; min-height: 0; }
:deep(.p-datatable-wrapper) { flex: 1; }
.vd-cell-title { display: flex; align-items: center; gap: 8px; }
.vd-cover { width: 40px; height: 40px; border-radius: 6px; object-fit: cover; flex-shrink: 0; }
.vd-cover-placeholder { width: 40px; height: 40px; border-radius: 6px; background: #f1f5f9; display: flex; align-items: center; justify-content: center; flex-shrink: 0; color: #94a3b8; }
.vd-title-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.83rem; }
.vd-error-text { font-size: 0.8rem; color: #dc2626; }
.vd-actions { display: flex; gap: 2px; }

/* ═══ 下载进度 ═══ */
.vd-dl-overall-progress { display: flex; align-items: center; gap: 10px; padding: 8px 14px; border-bottom: 1px solid #f1f5f9; flex-shrink: 0; }

/* ═══ 底部操作栏 ═══ */
.vd-bottom-bar { display: flex; align-items: center; justify-content: space-between; padding: 8px 14px; border-top: 1px solid #f1f5f9; flex-shrink: 0; }
.vd-total-info { font-size: 0.8rem; color: #94a3b8; }

/* ═══ 橙色按钮 ═══ */
.vd-btn-orange { background: #f97316 !important; border-color: #f97316 !important; color: #fff !important; }
.vd-btn-orange:hover { background: #ea580c !important; border-color: #ea580c !important; }
.vd-btn-orange:disabled { background: #fdba74 !important; border-color: #fdba74 !important; }

/* ═══ 筛选对话框 ═══ */
.vd-filter-form { display: flex; flex-direction: column; gap: 14px; }
.vd-filter-row { display: flex; align-items: center; gap: 12px; }
.vd-filter-row > label { width: 65px; font-size: 0.85rem; color: #475569; flex-shrink: 0; }
.vd-filter-range { display: flex; align-items: center; gap: 8px; }
.vd-filter-range span { color: #94a3b8; }

/* ═══ 下载对话框 ═══ */
.vd-dl-form { display: flex; flex-direction: column; gap: 14px; }
.vd-dl-row { display: flex; align-items: center; gap: 12px; }
.vd-dl-row label:first-child { width: 70px; font-size: 0.85rem; color: #475569; flex-shrink: 0; }

/* ═══ 平台标签 ═══ */
.vd-plat-tag { font-size: 0.7rem; font-weight: 600; padding: 2px 8px; border-radius: 4px; }
.vd-plat--dy { background: #0f0f0f !important; color: #fff !important; }
.vd-plat--ks { background: #ff4906 !important; color: #fff !important; }
.vd-plat--blb { background: #fb7299 !important; color: #fff !important; }
.vd-plat--mg { background: #e62e2e !important; color: #fff !important; }
.vd-plat--cctv { background: #c41230 !important; color: #fff !important; }
.vd-plat--ysp { background: #ff6600 !important; color: #fff !important; }
.vd-plat--xhs { background: #ff2442 !important; color: #fff !important; }

/* ═══ 作者标签 ═══ */
.vd-author-tag {
  display: inline-block;
  padding: 1px 8px;
  border-radius: 10px;
  font-size: 0.78rem;
  font-weight: 500;
  border: 1px solid;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 120px;
}

/* ═══ 下载速度渐变 ═══ */
.vd-speed-badge {
  font-weight: 700;
  background: linear-gradient(90deg, #6366f1, #a855f7, #ec4899, #f43f5e);
  background-size: 200% 100%;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  animation: vd-speed-gradient 3s ease infinite;
}
@keyframes vd-speed-gradient {
  0% { background-position: 0% 50%; }
  50% { background-position: 100% 50%; }
  100% { background-position: 0% 50%; }
}

/* ═══ 单文件下载进度 ═══ */
.vd-file-progress { display: flex; align-items: center; gap: 6px; }
.vd-file-progress-bar { flex: 1; height: 6px; background: #e2e8f0; border-radius: 3px; overflow: hidden; }
.vd-file-progress-fill { height: 100%; background: var(--p-primary-color); border-radius: 3px; transition: width 0.3s ease; }
.vd-file-progress-text { font-size: 12px; color: var(--p-primary-color); font-weight: 600; min-width: 36px; text-align: right; }

/* ═══ 表格线 ═══ */
.vd-table-grid :deep(.p-datatable-thead > tr > th) { border: 1px solid #e2e8f0 !important; }
.vd-table-grid :deep(.p-datatable-tbody > tr > td) { border: 1px solid #e2e8f0 !important; }

/* ═══ 过渡动画 ═══ */
.fade-enter-active, .fade-leave-active { transition: opacity 0.25s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>

<!-- tooltip 需要非 scoped 样式 -->
<style>
.vd-tooltip {
  position: fixed;
  z-index: 99999;
  max-width: 420px;
  padding: 8px 12px;
  background: rgba(15, 23, 42, 0.92);
  color: #fff;
  font-size: 0.8rem;
  line-height: 1.4;
  border-radius: 6px;
  pointer-events: auto;
  user-select: text;
  word-break: break-all;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
}
</style>
