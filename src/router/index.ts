import { createRouter, createWebHistory } from 'vue-router'
import { useUserStore } from '../stores/user'
import { logger } from '../utils/logger'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/login',
    },
    {
      path: '/login',
      name: 'Login',
      component: () => import('../views/auth/LoginView.vue'),
      meta: { title: '登录 - 青拾' },
    },
    {
      path: '/register',
      name: 'Register',
      component: () => import('../views/auth/RegisterView.vue'),
      meta: { title: '注册 - 青拾' },
    },
    {
      path: '/forgot-password',
      name: 'ForgotPassword',
      component: () => import('../views/auth/ForgotPasswordView.vue'),
      meta: { title: '找回密码 - 青拾' },
    },
    {
      path: '/main',
      component: () => import('../layouts/MainLayout.vue'),
      meta: { title: '主页 - 青拾', requiresAuth: true },
      children: [
        {
          path: '',
          redirect: '/main/dashboard',
        },
        {
          path: 'dashboard',
          name: 'Dashboard',
          component: () => import('../views/MainView.vue'),
          meta: { title: '仪表盘 - 青拾', requiresAuth: true },
        },
      ],
    },
  ],
})

router.beforeEach((to) => {
  const userStore = useUserStore()
  // #region agent log
  fetch('http://127.0.0.1:7486/ingest/637909e2-8eda-4b2c-af00-1f426bec300c',{method:'POST',headers:{'Content-Type':'application/json','X-Debug-Session-Id':'23ef92'},body:JSON.stringify({sessionId:'23ef92',location:'router/index.ts:beforeEach',message:'router guard',data:{to:to.fullPath,requiresAuth:!!to.meta.requiresAuth,isLoggedIn:userStore.isLoggedIn,windowTitle:document.title},timestamp:Date.now()})}).catch(()=>{});
  // #endregion
  logger.log('router', '路由进入 beforeEach', {
    to: to.fullPath,
    requiresAuth: !!to.meta.requiresAuth,
    isLoggedIn: userStore.isLoggedIn,
  })
  document.title = (to.meta.title as string) || '青拾'

  if (to.meta.requiresAuth && !userStore.isLoggedIn) {
    logger.warn('router', '未登录，重定向到登录页', { to: to.fullPath })
    return '/login'
  }

})

router.afterEach((to) => {
  logger.log('router', '路由跳转完成', {
    to: to.fullPath,
    title: document.title,
  })
})

export default router
