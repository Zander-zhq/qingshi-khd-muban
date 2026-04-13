import { createRouter, createWebHistory } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useUserStore } from '../stores/user'
import { logger } from '../utils/logger'
import { getBrand } from '../brand'
import appRoutes from '../app/routes'

const brand = getBrand()
const t = brand.template

const templateModules: Record<string, Record<string, () => Promise<any>>> = {
  green: {
    login: () => import('../templates/green/auth/LoginView.vue'),
    register: () => import('../templates/green/auth/RegisterView.vue'),
    forgotPassword: () => import('../templates/green/auth/ForgotPasswordView.vue'),
    recharge: () => import('../templates/green/auth/RechargeView.vue'),
    unbindDevice: () => import('../templates/green/auth/UnbindDeviceView.vue'),
    mainLayout: () => import('../templates/green/MainLayout.vue'),
    mainView: () => import('../templates/green/MainView.vue'),
  },
  orange: {
    login: () => import('../templates/orange/auth/LoginView.vue'),
    register: () => import('../templates/orange/auth/RegisterView.vue'),
    forgotPassword: () => import('../templates/orange/auth/ForgotPasswordView.vue'),
    recharge: () => import('../templates/orange/auth/RechargeView.vue'),
    unbindDevice: () => import('../templates/orange/auth/UnbindDeviceView.vue'),
    mainLayout: () => import('../templates/orange/MainLayout.vue'),
    mainView: () => import('../templates/orange/MainView.vue'),
  },
  dark: {
    login: () => import('../templates/dark/auth/LoginView.vue'),
    register: () => import('../templates/dark/auth/RegisterView.vue'),
    forgotPassword: () => import('../templates/dark/auth/ForgotPasswordView.vue'),
    recharge: () => import('../templates/dark/auth/RechargeView.vue'),
    unbindDevice: () => import('../templates/dark/auth/UnbindDeviceView.vue'),
    mainLayout: () => import('../templates/dark/MainLayout.vue'),
    mainView: () => import('../templates/dark/MainView.vue'),
  },
}

const m = templateModules[t] || templateModules.green

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/login' },
    { path: '/login', name: 'Login', component: m.login, meta: { title: `登录 - ${brand.brand_name}` } },
    { path: '/register', name: 'Register', component: m.register, meta: { title: `注册 - ${brand.brand_name}` } },
    { path: '/forgot-password', name: 'ForgotPassword', component: m.forgotPassword, meta: { title: `找回密码 - ${brand.brand_name}` } },
    { path: '/recharge', name: 'Recharge', component: m.recharge, meta: { title: `充值中心 - ${brand.brand_name}` } },
    { path: '/unbind-device', name: 'UnbindDevice', component: m.unbindDevice, meta: { title: `解绑设备 - ${brand.brand_name}` } },
    {
      path: '/main',
      component: m.mainLayout,
      meta: { title: `主页 - ${brand.brand_name}`, requiresAuth: true },
      children: [
        { path: '', redirect: '/main/dashboard' },
        { path: 'dashboard', name: 'Dashboard', component: m.mainView, meta: { title: `仪表盘 - ${brand.brand_name}`, requiresAuth: true } },
        ...appRoutes,
        ...(import.meta.env.DEV ? [
          { path: 'dev-brand', name: 'DevBrand', component: () => import('../dev/BrandManager.vue'), meta: { title: `品牌管理 - ${brand.brand_name}`, requiresAuth: true } },
          { path: 'dev-version', name: 'DevVersion', component: () => import('../dev/VersionManager.vue'), meta: { title: `版本管理 - ${brand.brand_name}`, requiresAuth: true } },
        ] : []),
      ],
    },
  ],
})

router.beforeEach((to) => {
  const userStore = useUserStore()
  logger.log('router', '路由进入 beforeEach', {
    to: to.fullPath,
    requiresAuth: !!to.meta.requiresAuth,
    isLoggedIn: userStore.isLoggedIn,
  })
  document.title = (to.meta.title as string) || brand.brand_name

  if (to.meta.requiresAuth && !userStore.isLoggedIn) {
    logger.warn('router', '未登录，重定向到登录页', { to: to.fullPath })
    return '/login'
  }
})

router.afterEach((to) => {
  const title = document.title
  getCurrentWindow().setTitle(title).catch(() => {})
  logger.log('router', '路由跳转完成', { to: to.fullPath, title })
})

export default router
