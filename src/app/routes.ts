import type { RouteRecordRaw } from 'vue-router'
import { getBrand } from '../brand'

const brand = getBrand()

const appRoutes: RouteRecordRaw[] = [
  {
    path: 'video-download',
    name: 'VideoDownload',
    component: () => import('./pages/VideoDownloadView.vue'),
    meta: {
      title: `视频下载 - ${brand.brand_name}`,
      requiresAuth: true,
      menuItem: { label: '视频下载', icon: 'pi pi-download', order: 1 },
    },
  },
]

export default appRoutes
