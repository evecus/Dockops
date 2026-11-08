import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/login', component: () => import('@/views/Login.vue'), meta: { public: true } },
    { path: '/setup', component: () => import('@/views/Setup.vue'), meta: { public: true } },
    {
      path: '/',
      component: () => import('@/views/Layout.vue'),
      children: [
        { path: '', redirect: '/dashboard' },
        { path: 'dashboard', component: () => import('@/views/Dashboard.vue') },
        { path: 'containers', component: () => import('@/views/Containers.vue') },
        { path: 'images', component: () => import('@/views/Images.vue') },
        { path: 'network-storage', component: () => import('@/views/NetworkStorage.vue') },
        { path: 'settings', component: () => import('@/views/Settings.vue') },
      ]
    },
    { path: '/:pathMatch(.*)*', redirect: '/' }
  ]
})

router.beforeEach(async (to) => {
  const token = localStorage.getItem('token')
  if (to.meta.public) return true
  if (!token) return '/login'
  return true
})

export default router
