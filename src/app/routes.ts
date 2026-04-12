import type { RouteRecordRaw } from 'vue-router'
import { getBrand } from '../brand'

const brand = getBrand()

const appRoutes: RouteRecordRaw[] = [
  {
    path: 'account-manager',
    name: 'AccountManager',
    component: () => import('./pages/AccountManagerView.vue'),
    meta: {
      title: `账号登记 - ${brand.brand_name}`,
      requiresAuth: true,
      menuItem: { label: '账号登记', icon: 'pi pi-users', order: 0 },
    },
  },
  {
    path: 'video-download',
    name: 'VideoDownload',
    component: () => import('./pages/VideoDownloadView.vue'),
    meta: {
      title: `视频解析下载 - ${brand.brand_name}`,
      requiresAuth: true,
      menuItem: { label: '视频解析下载', icon: 'pi pi-download', order: 1 },
    },
  },
  {
    path: 'settings',
    name: 'Settings',
    component: () => import('./pages/SettingsView.vue'),
    meta: {
      title: `下载设置 - ${brand.brand_name}`,
      requiresAuth: true,
      menuItem: { label: '下载设置', icon: 'pi pi-cog', order: 90 },
    },
  },
]

export default appRoutes
