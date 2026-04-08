/**
 * 产品业务路由 — 每个产品在此定义自己的业务页面路由
 *
 * 这些路由会被注入到 /main 下作为子路由。
 * 每条路由不需要写 /main 前缀，直接写子路径即可。
 *
 * 示例：
 *   { path: 'editor', name: 'Editor', component: () => import('./pages/EditorView.vue') }
 *   访问地址为 /main/editor
 */
import type { RouteRecordRaw } from 'vue-router'
import { getBrand } from '../brand'

const brand = getBrand()

const appRoutes: RouteRecordRaw[] = [
  {
    path: 'downloads',
    name: 'Downloads',
    component: () => import('./pages/PlaceholderView.vue'),
    meta: { title: `下载列表 - ${brand.brand_name}`, requiresAuth: true },
  },
  {
    path: 'history',
    name: 'History',
    component: () => import('./pages/PlaceholderView.vue'),
    meta: { title: `下载历史 - ${brand.brand_name}`, requiresAuth: true },
  },
  {
    path: 'settings',
    name: 'Settings',
    component: () => import('./pages/PlaceholderView.vue'),
    meta: { title: `设置 - ${brand.brand_name}`, requiresAuth: true },
  },
]

export default appRoutes
