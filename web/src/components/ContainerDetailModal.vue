<!-- ContainerDetailModal.vue -->
<template>
  <div class="modal modal-lg">
    <div class="modal-header">
      <div class="modal-title">
        <Box :size="16" />
        {{ container.name }}
        <span class="badge" :class="badgeClass(container.state)" style="margin-left:4px">
          {{ stateLabel(container.state) }}
        </span>
      </div>
      <button class="modal-close" @click="$emit('close')"><X :size="15" /></button>
    </div>

    <div class="modal-body">
      <div class="detail-grid">
        <div class="detail-section">
          <div class="section-title">基础信息</div>
          <table class="detail-table">
            <tbody>
              <tr><td>创建方式</td><td><span class="tag">{{ container.create_mode }}</span></td></tr>
              <tr><td>Compose 目录</td><td><span class="tag">{{ container.compose_dir }}</span></td></tr>
              <tr><td>创建时间</td><td>{{ fmtDate(container.created_at) }}</td></tr>
              <tr><td>Docker 状态</td><td>{{ container.status || '—' }}</td></tr>
            </tbody>
          </table>
        </div>

        <div class="detail-section" v-if="docker">
          <div class="section-title">运行信息</div>
          <table class="detail-table">
            <tbody>
              <tr><td>Hostname</td><td>{{ docker.hostname || '—' }}</td></tr>
              <tr><td>重启策略</td><td>{{ docker.restart_policy || '—' }}</td></tr>
              <tr v-if="docker.networks">
                <td>网络</td>
                <td>
                  <span v-for="(ip, net) in docker.networks" :key="net" class="tag" style="margin-right:4px">
                    {{ net }}: {{ ip }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="detail-section" v-if="docker?.mounts?.length">
          <div class="section-title">挂载点</div>
          <div class="mount-list">
            <div v-for="m in docker.mounts" :key="m.destination" class="mount-item">
              <span class="tag">{{ m.type }}</span>
              <span class="mount-path">{{ m.source }}</span>
              <span style="color:var(--text-muted)">→</span>
              <span class="mount-path">{{ m.destination }}</span>
            </div>
          </div>
        </div>

        <div class="detail-section" v-if="docker?.env?.length">
          <div class="section-title">环境变量 <span class="sep">({{ docker.env.length }})</span></div>
          <div class="env-list">
            <div v-for="e in docker.env.slice(0, showAllEnv ? 999 : 6)" :key="e" class="env-item">
              <span class="font-mono" style="font-size:11.5px">{{ e }}</span>
            </div>
            <button v-if="docker.env.length > 6" class="btn btn-ghost btn-sm" @click="showAllEnv = !showAllEnv">
              {{ showAllEnv ? '收起' : `展开全部 ${docker.env.length} 项` }}
            </button>
          </div>
        </div>

        <div class="detail-section" v-if="stats">
          <div class="section-title">实时资源</div>
          <div class="stats-grid">
            <div class="stat-mini">
              <span class="stat-mini-label">CPU</span>
              <span class="stat-mini-val">{{ stats.cpu_percent?.toFixed(1) }}%</span>
            </div>
            <div class="stat-mini">
              <span class="stat-mini-label">内存</span>
              <span class="stat-mini-val">{{ fmtBytes(stats.memory_usage) }}</span>
            </div>
            <div class="stat-mini">
              <span class="stat-mini-label">网络↑</span>
              <span class="stat-mini-val">{{ fmtBytes(stats.net_tx_bytes) }}</span>
            </div>
            <div class="stat-mini">
              <span class="stat-mini-label">网络↓</span>
              <span class="stat-mini-val">{{ fmtBytes(stats.net_rx_bytes) }}</span>
            </div>
          </div>
        </div>

        <div class="detail-section">
          <div class="section-title">Compose 配置</div>
          <pre class="code-block" style="max-height:200px;font-size:11.5px">{{ container.compose_content }}</pre>
        </div>
      </div>
    </div>

    <div class="modal-footer">
      <button class="btn btn-ghost" @click="$emit('close')">关闭</button>
      <button class="btn btn-ghost" @click="$emit('edit')"><Pencil :size="13" /> 编辑</button>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { Box, X, Pencil } from 'lucide-vue-next'
import api from '@/api'

const props = defineProps({ container: Object })
defineEmits(['close', 'edit', 'refresh'])
const docker = ref(null)
const stats = ref(null)
const showAllEnv = ref(false)

function badgeClass(s) {
  if (s === 'running') return 'badge-green badge-dot'
  if (s === 'exited') return 'badge-red'
  return 'badge-muted'
}
function stateLabel(s) {
  return { running: '运行中', exited: '已退出', paused: '已暂停' }[s] || s || '—'
}
function fmtDate(d) {
  if (!d) return '—'
  return new Date(d).toLocaleString('zh-CN')
}
function fmtBytes(b) {
  if (!b) return '0 B'
  if (b >= 1e9) return (b/1e9).toFixed(1)+' GB'
  if (b >= 1e6) return (b/1e6).toFixed(1)+' MB'
  if (b >= 1e3) return (b/1e3).toFixed(0)+' KB'
  return b+' B'
}

onMounted(async () => {
  try {
    const res = await api.getContainer(props.container.name)
    docker.value = res.data?.docker
  } catch {}
  if (props.container.state === 'running') {
    try {
      const s = await api.containerStats(props.container.name)
      stats.value = s.data
    } catch {}
  }
})
</script>

<style scoped>
.detail-grid { display: flex; flex-direction: column; gap: 20px; }
.detail-section {}
.section-title {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.8px;
  color: var(--text-muted);
  margin-bottom: 10px;
  display: flex; align-items: center; gap: 8px;
}
.detail-table { width: 100%; }
.detail-table td {
  padding: 5px 0;
  font-size: 13px;
  vertical-align: top;
}
.detail-table td:first-child { color: var(--text-muted); width: 110px; }
.detail-table td:last-child { color: var(--text-secondary); }
.mount-list { display: flex; flex-direction: column; gap: 5px; }
.mount-item { display: flex; align-items: center; gap: 8px; font-size: 12px; }
.mount-path { font-family: var(--font-mono); font-size: 11.5px; color: var(--text-secondary); }
.env-list { display: flex; flex-direction: column; gap: 3px; }
.env-item { padding: 4px 8px; background: var(--bg-input); border-radius: 4px; }
.stats-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
.stat-mini {
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 10px 12px;
  display: flex; flex-direction: column; gap: 4px;
}
.stat-mini-label { font-size: 11px; color: var(--text-muted); }
.stat-mini-val { font-size: 16px; font-weight: 700; color: var(--accent-light); font-family: var(--font-mono); }
.font-mono { font-family: var(--font-mono); }

@media (max-width: 768px) {
  .stats-grid { grid-template-columns: repeat(2, 1fr); }
  .detail-table td:first-child { width: 80px; font-size: 12px; }
  .mount-item { flex-direction: column; align-items: flex-start; gap: 2px; }
}
</style>
