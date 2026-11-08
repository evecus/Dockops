<template>
  <div class="containers-page">
    <!-- Header -->
    <div class="page-header">
      <div class="header-left">
        <div class="filter-group">
          <button class="filter-btn" :class="filter === 'all' ? 'active' : ''" @click="filter='all'">全部 <span class="filter-count">{{ containers.length }}</span></button>
          <button class="filter-btn" :class="filter === 'running' ? 'active' : ''" @click="filter='running'">运行中 <span class="filter-count">{{ runningCount }}</span></button>
          <button class="filter-btn" :class="filter === 'stopped' ? 'active' : ''" @click="filter='stopped'">已停止</button>
        </div>
      </div>
      <div class="header-right">
        <button class="btn btn-ghost" @click="load">
          <RefreshCw :size="14" /> 刷新
        </button>
        <button class="btn btn-primary" @click="showCreate = true">
          <Plus :size="15" /> 创建容器
        </button>
      </div>
    </div>

    <!-- Grid -->
    <div v-if="loading" class="empty-state"><div class="spinner"></div></div>
    <div v-else-if="!filtered.length" class="empty-state">
      <Box :size="48" />
      <p>暂无容器</p>
      <button class="btn btn-primary" @click="showCreate = true"><Plus :size="14" /> 创建第一个容器</button>
    </div>
    <div v-else class="container-grid">
      <div v-for="ct in filtered" :key="ct.name" class="ct-card" @click="openDetail(ct)">
        <!-- Header -->
        <div class="ct-card-header">
          <div class="ct-name">
            <div class="ct-indicator" :class="stateClass(ct.state)"></div>
            <span>{{ ct.name }}</span>
          </div>
          <span v-if="ct.image_update_available" class="badge badge-update" title="有镜像更新可用">↑ 有更新</span>
        </div>

        <!-- Info -->
        <div class="ct-info">
          <div class="ct-info-row">
            <span class="ct-info-key">状态</span>
            <span class="badge" :class="badgeClass(ct.state)" style="font-size:11px">{{ stateLabel(ct.state) }}</span>
          </div>
          <div class="ct-info-row">
            <span class="ct-info-key">端口</span>
            <div class="ct-ports">
              <template v-if="uniquePorts(ct.ports).length">
                <span
                  class="tag ct-port-tag"
                  v-for="p in uniquePorts(ct.ports).slice(0, 4)"
                  :key="p.host_port"
                  @click.stop="openPort(p.host_port)"
                >{{ p.host_port }}→{{ p.container_port }}</span>
                <span v-if="uniquePorts(ct.ports).length > 4" class="sep">+{{ uniquePorts(ct.ports).length - 4 }}</span>
              </template>
              <span v-else class="ct-no-port">—</span>
            </div>
          </div>
        </div>

        <!-- Actions -->
        <div class="ct-actions" @click.stop>
          <button class="ct-action-btn" v-if="ct.state !== 'running'"
            @click="start(ct)" :disabled="!!pendingOp[ct.name]">
            <Play :size="13" /><span>启动</span>
          </button>
          <button class="ct-action-btn" v-else
            @click="stop(ct)" :disabled="!!pendingOp[ct.name]">
            <Square :size="13" /><span>停止</span>
          </button>
          <button class="ct-action-btn" @click="restart(ct)" :disabled="!!pendingOp[ct.name]">
            <RotateCcw :size="13" /><span>重启</span>
          </button>
          <button class="ct-action-btn" @click="openTerminal(ct)">
            <Terminal :size="13" /><span>终端</span>
          </button>
          <button class="ct-action-btn" @click="openLogs(ct)">
            <ScrollText :size="13" /><span>日志</span>
          </button>
          <button class="ct-action-btn" @click="openFiles(ct)">
            <FolderOpen :size="13" /><span>文件</span>
          </button>
          <button class="ct-action-btn" @click="editContainer(ct)">
            <Pencil :size="13" /><span>编辑</span>
          </button>
          <button class="ct-action-btn ct-action-danger" @click="confirmDelete(ct)" :disabled="!!pendingOp[ct.name]">
            <Trash2 :size="13" /><span>删除</span>
          </button>
          <button v-if="ct.image_update_available" class="ct-action-btn ct-action-update" @click="updateImage(ct)" :disabled="!!pendingOp[ct.name]">
            <DownloadCloud :size="13" /><span>更新</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Modals -->
    <Teleport to="body">
      <div v-if="showCreate" class="modal-overlay" @click.self="showCreate = false">
        <CreateContainerModal @close="showCreate = false" @start-progress="onStartProgress" />
      </div>

      <div v-if="progressData" class="modal-overlay">
        <CreateProgressModal
          :name="progressData.name"
          :compose-content="progressData.compose_content"
          @close="progressData = null"
          @created="onCreated" />
      </div>

      <div v-if="detailCt" class="modal-overlay" @click.self="detailCt = null">
        <ContainerDetailModal :container="detailCt" @close="detailCt = null"
          @edit="editContainer(detailCt)" @refresh="load" />
      </div>

      <div v-if="editCt" class="modal-overlay" @click.self="editCt = null">
        <EditContainerModal :container="editCt" @close="editCt = null" @saved="onSaved" />
      </div>

      <div v-if="termCt" class="modal-overlay" @click.self="termCt = null">
        <TerminalModal :container="termCt" @close="termCt = null" />
      </div>

      <div v-if="logsCt" class="modal-overlay" @click.self="logsCt = null">
        <LogsModal :container="logsCt" @close="logsCt = null" />
      </div>

      <div v-if="filesCt" class="modal-overlay" @click.self="filesCt = null">
        <FilesModal :container="filesCt" @close="filesCt = null" />
      </div>

      <!-- Delete confirm modal -->
      <div v-if="deleteCt" class="modal-overlay" @click.self="deleteCt = null">
        <div class="modal" style="max-width:420px">
          <div class="modal-header">
            <div class="modal-title"><Trash2 :size="16" /> 删除容器</div>
            <button class="modal-close" @click="deleteCt = null"><X :size="15" /></button>
          </div>
          <div class="modal-body">
            <p style="color:var(--text-secondary);font-size:14px">
              确定要删除容器 <strong style="color:var(--text-primary)">{{ deleteCt.name }}</strong> 吗？此操作不可恢复。
            </p>
            <label v-if="deleteCt.has_compose" style="display:flex;align-items:center;gap:8px;margin-top:12px;font-size:13px;color:var(--text-secondary);cursor:pointer">
              <input type="checkbox" v-model="deleteCompose" style="width:14px;height:14px;cursor:pointer" />
              同时删除 Compose 文件
            </label>
          </div>
          <div class="modal-footer">
            <button class="btn btn-ghost" @click="deleteCt = null">取消</button>
            <button class="btn btn-danger" @click="doDelete">确认删除</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import {
  Plus, RefreshCw, Box, Play, Square, RotateCcw, Terminal,
  ScrollText, FolderOpen, Pencil, Trash2, X, DownloadCloud
} from 'lucide-vue-next'
import api from '@/api'
import { useToastStore } from '@/stores/toast'
import CreateContainerModal from '@/components/CreateContainerModal.vue'
import CreateProgressModal from '@/components/CreateProgressModal.vue'
import ContainerDetailModal from '@/components/ContainerDetailModal.vue'
import EditContainerModal from '@/components/EditContainerModal.vue'
import TerminalModal from '@/components/TerminalModal.vue'
import LogsModal from '@/components/LogsModal.vue'
import FilesModal from '@/components/FilesModal.vue'

const toast = useToastStore()
const containers = ref([])
const loading = ref(true)
const filter = ref('all')
const showCreate = ref(false)
const detailCt = ref(null)
const editCt = ref(null)
const termCt = ref(null)
const logsCt = ref(null)
const filesCt = ref(null)
const deleteCt = ref(null)
const deleteCompose = ref(false)
const progressData = ref(null)

// Track in-progress operations per container name
const pendingOp = ref({})

const runningCount = computed(() => containers.value.filter(c => c.state === 'running').length)
const filtered = computed(() => {
  if (filter.value === 'running') return containers.value.filter(c => c.state === 'running')
  if (filter.value === 'stopped') return containers.value.filter(c => c.state !== 'running')
  return containers.value
})

function stateClass(s) {
  if (s === 'running') return 'running'
  if (s === 'exited' || s === 'dead') return 'stopped'
  return 'paused'
}
function badgeClass(s) {
  if (s === 'running') return 'badge-green badge-dot'
  if (s === 'exited' || s === 'dead') return 'badge-red'
  return 'badge-muted'
}
function stateLabel(s) {
  return { running: '运行中', exited: '已退出', paused: '已暂停', dead: '异常', created: '已创建' }[s] || s || '未知'
}
function shortPath(p) {
  if (!p) return '—'
  const parts = p.split('/')
  return parts.length > 3 ? '…/' + parts.slice(-2).join('/') : p
}
function uniquePorts(ports) {
  if (!ports?.length) return []
  const seen = new Set()
  return ports.filter(p => {
    if (!p.host_port || p.host_port === '0') return false
    if (seen.has(p.host_port)) return false
    seen.add(p.host_port)
    return true
  })
}
const hostName = window.location?.hostname ?? ''
function openPort(hostPort) {
  window.open(`http://${hostName}:${hostPort}`, '_blank')
}

async function load() {
  loading.value = true
  try {
    const res = await api.listContainers()
    containers.value = res.data || []
  } catch (e) { toast.error('加载容器失败') }
  finally { loading.value = false }
}

async function start(ct) {
  pendingOp.value[ct.name] = true
  toast.info(`正在启动 ${ct.name}…`)
  try {
    await api.startContainer(ct.name)
    toast.success(`启动 ${ct.name} 成功`)
    load()
  } catch (e) {
    toast.error(`启动 ${ct.name} 失败`)
  } finally {
    delete pendingOp.value[ct.name]
  }
}

async function stop(ct) {
  pendingOp.value[ct.name] = true
  toast.info(`正在停止 ${ct.name}…`)
  try {
    await api.stopContainer(ct.name)
    toast.success(`停止 ${ct.name} 成功`)
    load()
  } catch (e) {
    toast.error(`停止 ${ct.name} 失败`)
  } finally {
    delete pendingOp.value[ct.name]
  }
}

async function restart(ct) {
  pendingOp.value[ct.name] = true
  toast.info(`正在重启 ${ct.name}…`)
  try {
    await api.restartContainer(ct.name)
    toast.success(`重启 ${ct.name} 成功`)
    load()
  } catch (e) {
    toast.error(`重启 ${ct.name} 失败`)
  } finally {
    delete pendingOp.value[ct.name]
  }
}

async function updateImage(ct) {
  pendingOp.value[ct.name] = true
  toast.info(`正在更新 ${ct.name} 的镜像…`)
  try {
    await api.updateContainerImage(ct.name)
    toast.success(`${ct.name} 镜像更新成功，容器已重启`)
    load()
  } catch (e) {
    toast.error(`${ct.name} 镜像更新失败`)
  } finally {
    delete pendingOp.value[ct.name]
  }
}

function openDetail(ct) { detailCt.value = ct }
function openTerminal(ct) { termCt.value = ct }
function openLogs(ct) { logsCt.value = ct }
function openFiles(ct) { filesCt.value = ct }
function editContainer(ct) { editCt.value = ct; detailCt.value = null }
function confirmDelete(ct) { deleteCt.value = ct; deleteCompose.value = false }

async function doDelete() {
  const ct = deleteCt.value
  if (!ct) return
  const removeCompose = deleteCompose.value
  // Close confirm modal immediately
  deleteCt.value = null
  pendingOp.value[ct.name] = true
  toast.info(`正在删除 ${ct.name}…`)
  try {
    await api.deleteContainer(ct.name, removeCompose)
    toast.success(`删除 ${ct.name} 成功`)
    load()
  } catch (e) {
    toast.error(`删除 ${ct.name} 失败`)
  } finally {
    delete pendingOp.value[ct.name]
  }
}

function onStartProgress(data) {
  showCreate.value = false
  progressData.value = data
}

function onSaved() {
  editCt.value = null
  load()
}
function onCreated() {
  showCreate.value = false
  progressData.value = null
  load()
}

onMounted(load)
</script>

<style scoped>
.containers-page {}
.page-header {
  display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px;
}
.header-left, .header-right { display: flex; align-items: center; gap: 10px; }
.filter-group {
  display: flex; background: var(--bg-card); border: 1px solid var(--border);
  border-radius: var(--radius); padding: 3px; gap: 2px;
}
.filter-btn {
  padding: 5px 12px; border-radius: calc(var(--radius) - 3px); font-size: 12.5px;
  font-weight: 500; color: var(--text-muted); background: none;
  display: flex; align-items: center; gap: 6px; transition: all var(--transition); cursor: pointer;
}
.filter-btn:hover { color: var(--text-secondary); }
.filter-btn.active { background: var(--accent-dim); color: var(--accent-light); }
.filter-count {
  background: rgba(6,182,212,0.15); color: var(--accent);
  font-size: 10px; padding: 0 5px; border-radius: 99px;
}
.container-grid {
  display: grid; grid-template-columns: repeat(auto-fill, minmax(310px, 1fr)); gap: 16px;
}
.ct-card {
  background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius-lg);
  padding: 16px; cursor: pointer; transition: all var(--transition);
  display: flex; flex-direction: column; gap: 12px; position: relative; overflow: hidden;
}
.ct-card::before {
  content: ''; position: absolute; top: 0; left: 0; right: 0; height: 2px;
  background: linear-gradient(90deg, transparent, var(--accent), transparent);
  opacity: 0; transition: opacity var(--transition);
}
.ct-card:hover { border-color: var(--border-2); box-shadow: var(--shadow-cyan); transform: translateY(-2px); }
.ct-card:hover::before { opacity: 0.6; }
.ct-card-header { display: flex; align-items: center; justify-content: space-between; }
.ct-name { display: flex; align-items: center; gap: 8px; font-weight: 600; font-size: 14px; }
.ct-indicator { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.ct-indicator.running { background: var(--green); box-shadow: 0 0 8px var(--green); animation: pulse-dot 2s infinite; }
.ct-indicator.stopped { background: var(--text-muted); }
.ct-indicator.paused  { background: var(--amber); }
.ct-info { display: flex; flex-direction: column; gap: 6px; }
.ct-info-row { display: flex; align-items: center; gap: 8px; font-size: 12px; }
.ct-info-key { color: var(--text-muted); min-width: 28px; font-weight: 500; }
.ct-info-val { color: var(--text-secondary); font-family: var(--font-mono); font-size: 11px; }
.ct-ports { display: flex; flex-wrap: wrap; gap: 4px; }
.ct-actions {
  display: flex; flex-wrap: nowrap; gap: 4px; padding-top: 8px; border-top: 1px solid var(--border);
}
.ct-action-btn {
  display: flex; flex-direction: column; align-items: center; gap: 3px;
  padding: 6px 4px; border-radius: var(--radius); font-size: 10px; font-weight: 500;
  color: var(--text-muted); background: var(--bg-input); border: 1px solid var(--border);
  cursor: pointer; transition: all var(--transition); flex: 1;
}
.ct-action-btn:hover { color: var(--accent-light); background: var(--accent-dim); border-color: var(--border-2); }
.ct-action-btn:disabled { opacity: 0.4; cursor: not-allowed; pointer-events: none; }
.ct-action-danger:hover {
  color: var(--red) !important; background: rgba(240,84,100,0.08) !important;
  border-color: rgba(240,84,100,0.2) !important;
}
.ct-action-update {
  color: var(--amber, #f59e0b) !important;
  border-color: rgba(245,158,11,0.3) !important;
}
.ct-action-update:hover {
  background: rgba(245,158,11,0.1) !important;
  border-color: rgba(245,158,11,0.5) !important;
}
.badge-update {
  background: rgba(245,158,11,0.15);
  color: #f59e0b;
  border: 1px solid rgba(245,158,11,0.3);
  border-radius: 99px;
  font-size: 10px;
  font-weight: 600;
  padding: 2px 8px;
}
.ct-port-tag { cursor: pointer; transition: all var(--transition); }
.ct-port-tag:hover { color: var(--accent-light); background: var(--accent-dim); border-color: var(--border-3); }
.ct-no-port { font-size: 12px; color: var(--text-muted); }
@media (max-width: 768px) {
  .page-header { flex-direction: column; align-items: stretch; gap: 12px; margin-bottom: 16px; }
  .header-right { justify-content: flex-end; }
  .container-grid { grid-template-columns: 1fr; gap: 12px; }
  .ct-card { padding: 14px; }
}
</style>
