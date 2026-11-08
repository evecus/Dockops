<template>
  <div class="dashboard">
    <div class="grid-4 mb-6">
      <div class="stat-card" v-for="stat in statCards" :key="stat.label">
        <div class="stat-icon" :style="`background:${stat.bg}`">
          <component :is="stat.icon" :size="19" :style="`color:${stat.color}`" />
        </div>
        <div class="stat-value" :style="`color:${stat.color}`">{{ stat.value }}</div>
        <div class="stat-label">{{ stat.label }}</div>
      </div>
    </div>

    <div class="grid-2 mb-6">
      <div class="card">
        <div class="card-header">
          <div class="card-title"><Server :size="16" /> {{ t.hostInfo }}</div>
          <span class="badge badge-cyan">{{ info?.docker_version || '—' }}</span>
        </div>
        <div class="card-body">
          <div v-if="loading" class="empty-state"><div class="spinner"></div></div>
          <table v-else class="info-table">
            <tbody>
              <tr v-for="row in infoRows" :key="row.label">
                <td class="info-key">{{ row.label }}</td>
                <td class="info-val">{{ row.value }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div class="card">
        <div class="card-header">
          <div class="card-title"><Activity :size="16" /> {{ t.resourceUsage }}</div>
          <span v-if="cacheTime" class="cache-hint">{{ cacheTimeText }}</span>
        </div>
        <div class="card-body">
          <div class="resource-gauges">
            <div class="gauge-item">
              <div class="gauge-header">
                <span class="gauge-label">{{ t.cpuUsage }}</span>
                <span class="gauge-val">{{ stats?.total_cpu_percent?.toFixed(1) || '0.0' }}%</span>
              </div>
              <div class="progress-bar">
                <div class="progress-fill" :style="`width:${Math.min(stats?.total_cpu_percent || 0, 100)}%`"></div>
              </div>
            </div>
            <div class="gauge-item">
              <div class="gauge-header">
                <span class="gauge-label">{{ t.memUsage }}</span>
                <span class="gauge-val">{{ formatBytes(stats?.total_mem_usage) }} / {{ formatBytes(info?.total_memory) }}</span>
              </div>
              <div class="progress-bar">
                <div class="progress-fill" :style="`width:${memPercent}%`"
                  :class="memPercent > 80 ? 'fill-red' : memPercent > 60 ? 'fill-amber' : ''"></div>
              </div>
            </div>
          </div>
          <div class="container-breakdown">
            <div class="breakdown-item" v-for="b in breakdown" :key="b.label">
              <div class="breakdown-dot" :style="`background:${b.color}`"></div>
              <span class="breakdown-label">{{ b.label }}</span>
              <span class="breakdown-val">{{ b.value }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, inject, watch, onMounted } from 'vue'
import { Server, Activity, Cpu, MemoryStick, HardDrive, Container, Box } from 'lucide-vue-next'
import api from '@/api'
import { useI18nStore } from '@/stores/i18n'

const i18n = useI18nStore()
const t = computed(() => i18n.t)

const info = ref(null)
const stats = ref(null)
const loading = ref(true)
const cacheTime = ref(null)

const dashboardRefreshData = inject('dashboardRefreshData', ref(null))
watch(dashboardRefreshData, (val) => {
  if (!val) return
  info.value = val.info
  stats.value = val.stats
  cacheTime.value = Date.now()
})

const cacheTimeText = computed(() => {
  if (!cacheTime.value) return ''
  const diff = Math.floor((Date.now() - cacheTime.value) / 1000)
  return t.value.updatedAgo(diff)
})

const memPercent = computed(() => {
  if (!stats.value?.total_mem_usage || !info.value?.total_memory) return 0
  return Math.min(stats.value.total_mem_usage / info.value.total_memory * 100, 100)
})

const statCards = computed(() => [
  { label: t.value.runningContainers, value: info.value?.containers_running ?? '—', icon: Box, color: 'var(--green)', bg: 'rgba(16,185,129,0.1)' },
  { label: t.value.allContainers, value: info.value?.containers ?? '—', icon: Container, color: 'var(--accent)', bg: 'rgba(37,99,235,0.08)' },
  { label: t.value.localImages, value: info.value?.images ?? '—', icon: HardDrive, color: 'var(--purple)', bg: 'rgba(139,92,246,0.1)' },
  { label: t.value.cpuCores, value: info.value?.cpus ?? '—', icon: Cpu, color: 'var(--amber)', bg: 'rgba(245,158,11,0.1)' },
])

const infoRows = computed(() => [
  { label: t.value.dockerVersion, value: info.value?.docker_version || '—' },
  { label: t.value.os, value: info.value?.os || '—' },
  { label: t.value.arch, value: info.value?.arch || '—' },
  { label: t.value.kernelVersion, value: info.value?.kernel_version || '—' },
  { label: t.value.storageDriver, value: info.value?.storage_driver || '—' },
  { label: t.value.dockerRootDir, value: info.value?.docker_root_dir || '—' },
  { label: t.value.totalMemory, value: formatBytes(info.value?.total_memory) },
  { label: t.value.serverTime, value: info.value?.server_time ? new Date(info.value.server_time).toLocaleString() : '—' },
])

const breakdown = computed(() => [
  { label: t.value.running, value: info.value?.containers_running ?? 0, color: 'var(--green)' },
  { label: t.value.stopped, value: info.value?.containers_stopped ?? 0, color: 'var(--text-muted)' },
  { label: t.value.paused, value: info.value?.containers_paused ?? 0, color: 'var(--amber)' },
])

function formatBytes(b) {
  if (!b) return '—'
  if (b >= 1e9) return (b / 1e9).toFixed(1) + ' GB'
  if (b >= 1e6) return (b / 1e6).toFixed(1) + ' MB'
  return (b / 1e3).toFixed(0) + ' KB'
}

async function loadCached() {
  try {
    const [i, s] = await Promise.all([api.dashboardInfo(), api.dashboardStats()])
    info.value = i.data
    stats.value = s.data
    cacheTime.value = Date.now()
  } catch {}
}

onMounted(async () => {
  try { await loadCached() }
  finally { loading.value = false }
})
</script>

<style scoped>
.dashboard {}
.mb-6 { margin-bottom: 24px; }
.info-table { width: 100%; }
.info-key { font-size: 13px; color: var(--text-muted); padding: 7px 0; width: 130px; font-weight: 500; }
.info-val { font-size: 13.5px; font-family: var(--font-mono); color: var(--text-secondary); padding: 7px 0; }
.resource-gauges { display: flex; flex-direction: column; gap: 22px; margin-bottom: 24px; }
.gauge-item { display: flex; flex-direction: column; gap: 8px; }
.gauge-header { display: flex; justify-content: space-between; align-items: center; }
.gauge-label { font-size: 13.5px; color: var(--text-secondary); font-weight: 500; }
.gauge-val { font-size: 13.5px; font-family: var(--font-mono); color: var(--accent); }
.fill-red { background: var(--red) !important; }
.fill-amber { background: var(--amber) !important; }
.container-breakdown { display: flex; gap: 24px; padding: 14px 16px; background: var(--bg-hover); border-radius: var(--radius); border: 1px solid var(--border); }
.breakdown-item { display: flex; align-items: center; gap: 8px; }
.breakdown-dot { width: 8px; height: 8px; border-radius: 50%; }
.breakdown-label { font-size: 13px; color: var(--text-muted); }
.breakdown-val { font-size: 14px; font-weight: 600; color: var(--text-primary); }
.cache-hint { font-size: 12px; color: var(--text-muted); font-family: var(--font-mono); }
</style>
