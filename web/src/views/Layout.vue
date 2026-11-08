<template>
  <div class="app-layout">
    <Transition name="overlay">
      <div v-if="drawerOpen" class="mobile-overlay" @click="drawerOpen = false"></div>
    </Transition>

    <!-- Sidebar -->
    <aside class="sidebar" :class="{ 'drawer-open': drawerOpen }">
      <div class="sidebar-logo">
        <div class="logo-icon">
          <img src="/apple-touch-icon.png" width="20" height="20" style="border-radius:4px;display:block;" alt="DockOps" />
        </div>
        <span class="logo-text">Dock<span>Ops</span></span>
        <button class="drawer-close" @click="drawerOpen = false">
          <X :size="18" />
        </button>
      </div>

      <nav class="sidebar-nav">
        <RouterLink v-for="item in navItems" :key="item.path"
          :to="item.path" custom v-slot="{ isActive, navigate }">
          <div class="nav-item" :class="{ active: isActive }" @click="() => { navigate(); drawerOpen = false }">
            <component :is="item.icon" :size="17" />
            <span>{{ item.label }}</span>
          </div>
        </RouterLink>
      </nav>

      <div class="sidebar-footer">
        <div class="nav-item" @click="handleLogout">
          <LogOut :size="17" />
          <span>{{ t.logout }}</span>
        </div>
      </div>
    </aside>

    <!-- Main -->
    <div class="main-content">
      <!-- Topbar -->
      <header class="topbar">
        <div class="topbar-left">
          <button class="hamburger-btn" @click="drawerOpen = true">
            <Menu :size="20" />
          </button>
          <div>
            <div class="topbar-title">{{ currentTitle }}</div>
          </div>
        </div>
        <div class="topbar-actions">
          <button v-if="isDashboard"
            class="btn btn-ghost btn-sm topbar-refresh"
            @click="globalRefresh" :disabled="refreshing" :title="t.refreshDashboard">
            <div v-if="refreshing" class="spinner" style="width:12px;height:12px;border-width:2px"></div>
            <RefreshCw v-else :size="13" />
          </button>
          <!-- Lang toggle -->
          <button class="lang-btn" @click="i18n.toggle()">{{ t.langLabel }}</button>
          <div class="docker-status" :class="dockerOk ? 'ok' : 'err'">
            <span class="status-dot"></span>
            <span class="docker-status-text">{{ dockerOk ? t.dockerConnected : t.dockerDisconnected }}</span>
          </div>
          <div class="topbar-user">
            <div class="user-avatar">
              <User :size="14" />
            </div>
            <span class="user-name">Admin</span>
          </div>
        </div>
      </header>

      <!-- Page content -->
      <div class="page-content">
        <RouterView v-slot="{ Component }">
          <Transition name="page" mode="out-in">
            <component :is="Component" />
          </Transition>
        </RouterView>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, provide, onMounted } from 'vue'
import { RouterLink, RouterView, useRouter, useRoute } from 'vue-router'
import { LayoutDashboard, Box, Image, Network, Settings, LogOut, User, RefreshCw, Menu, X } from 'lucide-vue-next'
import api from '@/api'
import { useI18nStore } from '@/stores/i18n'

const router = useRouter()
const route = useRoute()
const i18n = useI18nStore()
const t = computed(() => i18n.t)

const dockerOk = ref(true)
const refreshing = ref(false)
const drawerOpen = ref(false)

const isDashboard = computed(() => route.path === '/dashboard')

const dashboardRefreshData = ref(null)
provide('dashboardRefreshData', dashboardRefreshData)

async function globalRefresh() {
  if (route.path !== '/dashboard') return
  refreshing.value = true
  try {
    const res = await api.dashboardRefresh()
    dashboardRefreshData.value = res.data
  } catch {}
  finally { refreshing.value = false }
}

const navItems = computed(() => [
  { path: '/dashboard', label: t.value.dashboard, icon: LayoutDashboard },
  { path: '/containers', label: t.value.containers, icon: Box },
  { path: '/images', label: t.value.images, icon: Image },
  { path: '/network-storage', label: t.value.networkStorage, icon: Network },
  { path: '/settings', label: t.value.settings, icon: Settings },
])

const pageMeta = computed(() => ({
  '/dashboard': { title: t.value.dashboard, desc: t.value.dashboardDesc },
  '/containers': { title: t.value.containers, desc: t.value.containersDesc },
  '/images': { title: t.value.images, desc: t.value.imagesDesc },
  '/network-storage': { title: t.value.networkStorage, desc: t.value.networkStorageDesc },
  '/settings': { title: t.value.settings, desc: t.value.settingsDesc },
}))

const currentTitle = computed(() => pageMeta.value[route.path]?.title || 'DockOps')
const currentDesc = computed(() => pageMeta.value[route.path]?.desc || '')

function handleLogout() {
  localStorage.removeItem('token')
  router.push('/login')
}

onMounted(async () => {
  try {
    await api.dashboardInfo()
    dockerOk.value = true
  } catch { dockerOk.value = false }
})
</script>

<style scoped>
.mobile-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.4); z-index: 199; backdrop-filter: blur(2px); }
.overlay-enter-active, .overlay-leave-active { transition: opacity 0.25s ease; }
.overlay-enter-from, .overlay-leave-to { opacity: 0; }

.hamburger-btn {
  display: none; align-items: center; justify-content: center;
  width: 36px; height: 36px; border-radius: 8px;
  background: transparent; color: var(--text-secondary); border: 1px solid var(--border);
  flex-shrink: 0; transition: all var(--transition);
}
.hamburger-btn:hover { color: var(--accent); border-color: var(--accent); }

.drawer-close {
  display: none; margin-left: auto; align-items: center; justify-content: center;
  width: 28px; height: 28px; border-radius: 6px;
  background: transparent; color: var(--text-muted); border: none; flex-shrink: 0;
  transition: all var(--transition);
}
.drawer-close:hover { color: var(--text-primary); background: var(--bg-hover); }

.topbar-left { display: flex; align-items: center; gap: 12px; }
.topbar-left > div { display: flex; flex-direction: column; gap: 2px; }
.topbar-breadcrumb { font-size: 12px; color: var(--text-muted); }
.topbar-refresh { padding: 5px 8px; border-radius: 6px; }

.docker-status {
  display: flex; align-items: center; gap: 6px;
  padding: 5px 12px; border-radius: 99px; font-size: 13px; font-weight: 500;
}
.docker-status.ok { background: rgba(16,185,129,0.07); color: var(--green); border: 1px solid rgba(16,185,129,0.18); }
.docker-status.err { background: rgba(239,68,68,0.07); color: var(--red); border: 1px solid rgba(239,68,68,0.15); }
.status-dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; flex-shrink: 0; }
.docker-status.ok .status-dot { animation: pulse-dot 2s infinite; }

.topbar-user {
  display: flex; align-items: center; gap: 8px; padding: 5px 12px;
  border-radius: 99px; font-size: 13px; font-weight: 500;
  color: var(--text-secondary); background: var(--bg-card); border: 1px solid var(--border); cursor: pointer;
}
.user-avatar {
  width: 22px; height: 22px; border-radius: 50%;
  background: var(--accent); display: flex; align-items: center; justify-content: center; color: white;
}

.page-enter-active { transition: all 0.25s ease; }
.page-leave-active { transition: all 0.15s ease; }
.page-enter-from { opacity: 0; transform: translateY(6px); }
.page-leave-to   { opacity: 0; }

@media (max-width: 768px) {
  .sidebar { transform: translateX(-100%); transition: transform 0.28s cubic-bezier(0.4,0,0.2,1); z-index: 200; }
  .sidebar.drawer-open { transform: translateX(0); box-shadow: 4px 0 32px rgba(0,0,0,0.2); }
  .drawer-close { display: flex; }
  .main-content { margin-left: 0 !important; }
  .hamburger-btn { display: flex; }
  .topbar { padding: 0 16px !important; }
  .page-content { padding: 16px !important; }
  .docker-status-text { display: none; }
  .docker-status { padding: 6px 8px; border-radius: 50%; }
  .user-name { display: none; }
  .topbar-user { padding: 5px 7px; }
}
</style>
