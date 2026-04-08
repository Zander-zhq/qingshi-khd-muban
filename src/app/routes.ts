/**
 * 产品业务路由 — 每个产品在此定义自己的业务页面路由
 *
 * 这些路由会被注入到 /main 下作为子路由。
 * 每条路由不需要写 /main 前缀，直接写子路径即可。
 *
 * 示例：
 *   {
 *     path: 'editor',
 *     name: 'Editor',
 *     component: () => import('./pages/EditorView.vue'),
 *     meta: { title: `编辑器 - ${brand.brand_name}`, requiresAuth: true },
 *   }
 *   访问地址为 /main/editor
 */
import type { RouteRecordRaw } from 'vue-router'

const appRoutes: RouteRecordRaw[] = []

export default appRoutes
