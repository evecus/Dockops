<template>
  <div class="ns-page">
    <div class="ns-tabs">
      <button class="ns-tab" :class="{active:tab==='network'}" @click="tab='network'">
        <Network :size="15"/> 网络管理
      </button>
      <button class="ns-tab" :class="{active:tab==='volume'}" @click="tab='volume'">
        <Database :size="15"/> 存储卷管理
      </button>
    </div>

    <!-- Networks -->
    <div v-if="tab==='network'">
      <div class="section-header">
        <span class="section-count">{{ networks.length }} 个网络</span>
        <div style="display:flex;gap:10px">
          <button class="btn btn-ghost" @click="pruneNetworks"><Trash2 :size="14"/> 清理未使用</button>
          <button class="btn btn-ghost" @click="loadNetworks"><RefreshCw :size="14"/> 刷新</button>
          <button class="btn btn-primary" @click="showCreateNet=true"><Plus :size="14"/> 创建网络</button>
        </div>
      </div>
      <div v-if="netLoading" class="empty-state"><div class="spinner"></div></div>
      <div v-else-if="!networks.length" class="empty-state"><Network :size="48"/><p>暂无网络</p></div>
      <div v-else class="card" style="overflow:hidden">
        <table class="data-table">
          <thead><tr><th>网络名</th><th>ID</th><th>驱动</th><th>范围</th><th>IPv4</th><th>IPv6</th><th>容器数</th><th>创建时间</th><th>操作</th></tr></thead>
          <tbody>
            <tr v-for="net in networks" :key="net.id">
              <td><span class="net-name">{{ net.name }}</span></td>
              <td><span class="tag" style="font-size:10px">{{ net.id }}</span></td>
              <td><span class="badge" :class="driverBadge(net.driver)">{{ net.driver }}</span></td>
              <td><span class="badge badge-muted">{{ net.scope }}</span></td>
              <td><span class="font-mono">{{ net.ipv4||'—' }}</span></td>
              <td><span class="font-mono muted">{{ net.ipv6||'—' }}</span></td>
              <td><span class="badge" :class="net.containers>0?'badge-green':'badge-muted'">{{ net.containers }}</span></td>
              <td><span class="muted-sm">{{ fmtDate(net.created) }}</span></td>
              <td>
                <button class="btn btn-icon btn-danger-icon"
                  :disabled="['bridge','host','none'].includes(net.name)"
                  @click="confirmDeleteNet(net)" data-tip="删除"><Trash2 :size="13"/></button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Volumes -->
    <div v-if="tab==='volume'">
      <div class="section-header">
        <span class="section-count">{{ volumes.length }} 个存储卷</span>
        <div style="display:flex;gap:10px">
          <button class="btn btn-ghost" @click="pruneVolumes"><Trash2 :size="14"/> 清理未使用</button>
          <button class="btn btn-ghost" @click="loadVolumes"><RefreshCw :size="14"/> 刷新</button>
          <button class="btn btn-primary" @click="showCreateVol=true"><Plus :size="14"/> 创建存储卷</button>
        </div>
      </div>
      <div v-if="volLoading" class="empty-state"><div class="spinner"></div></div>
      <div v-else-if="!volumes.length" class="empty-state"><Database :size="48"/><p>暂无存储卷</p></div>
      <div v-else class="card" style="overflow:hidden">
        <table class="data-table">
          <thead><tr><th>卷名</th><th>驱动</th><th>范围</th><th>挂载点</th><th>创建时间</th><th>操作</th></tr></thead>
          <tbody>
            <tr v-for="vol in volumes" :key="vol.name">
              <td><span class="vol-name">{{ vol.name }}</span></td>
              <td><span class="badge badge-cyan">{{ vol.driver }}</span></td>
              <td><span class="badge badge-muted">{{ vol.scope }}</span></td>
              <td><span class="font-mono muted">{{ vol.mountpoint||'—' }}</span></td>
              <td><span class="muted-sm">{{ fmtDate(vol.created) }}</span></td>
              <td>
                <div style="display:flex;gap:4px">
                  <button class="btn btn-icon btn-danger-icon" @click="confirmDeleteVol(vol,false)" data-tip="删除"><Trash2 :size="13"/></button>
                  <button class="btn btn-icon btn-danger-icon" @click="confirmDeleteVol(vol,true)" data-tip="强制删除"><Zap :size="13"/></button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <Teleport to="body">
      <!-- Create Network -->
      <div v-if="showCreateNet" class="modal-overlay" @click.self="showCreateNet=false">
        <div class="modal" style="max-width:440px">
          <div class="modal-header">
            <div class="modal-title"><Network :size="16"/> 创建网络</div>
            <button class="modal-close" @click="showCreateNet=false"><X :size="15"/></button>
          </div>
          <div class="modal-body" style="display:flex;flex-direction:column;gap:14px">
            <div class="form-group">
              <label class="form-label">网络名称 *</label>
              <input v-model="newNet.name" class="form-input" placeholder="my-network" autofocus/>
            </div>
            <div class="form-group">
              <label class="form-label">驱动类型</label>
              <select v-model="newNet.driver" class="form-select">
                <option value="bridge">bridge（默认）</option>
                <option value="host">host</option>
                <option value="overlay">overlay</option>
                <option value="macvlan">macvlan</option>
                <option value="ipvlan">ipvlan</option>
                <option value="none">none</option>
              </select>
            </div>
            <div class="driver-desc">{{ driverDesc(newNet.driver) }}</div>
          </div>
          <div class="modal-footer">
            <button class="btn btn-ghost" @click="showCreateNet=false">取消</button>
            <button class="btn btn-primary" @click="doCreateNet" :disabled="!newNet.name||creating">
              <div v-if="creating" class="spinner" style="width:13px;height:13px;border-width:2px"></div>创建
            </button>
          </div>
        </div>
      </div>

      <!-- Create Volume -->
      <div v-if="showCreateVol" class="modal-overlay" @click.self="showCreateVol=false">
        <div class="modal" style="max-width:420px">
          <div class="modal-header">
            <div class="modal-title"><Database :size="16"/> 创建存储卷</div>
            <button class="modal-close" @click="showCreateVol=false"><X :size="15"/></button>
          </div>
          <div class="modal-body">
            <div class="form-group">
              <label class="form-label">卷名称 *</label>
              <input v-model="newVolName" class="form-input" placeholder="my-volume" autofocus/>
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn btn-ghost" @click="showCreateVol=false">取消</button>
            <button class="btn btn-primary" @click="doCreateVol" :disabled="!newVolName||creating">
              <div v-if="creating" class="spinner" style="width:13px;height:13px;border-width:2px"></div>创建
            </button>
          </div>
        </div>
      </div>

      <!-- Delete Network -->
      <div v-if="deleteNet" class="modal-overlay" @click.self="deleteNet=null">
        <div class="modal" style="max-width:400px">
          <div class="modal-header"><div class="modal-title"><Trash2 :size="16"/> 删除网络</div>
            <button class="modal-close" @click="deleteNet=null"><X :size="15"/></button></div>
          <div class="modal-body"><p style="color:var(--text-secondary);font-size:14px">确认删除网络 <strong style="color:var(--text-primary)">{{ deleteNet.name }}</strong>？</p></div>
          <div class="modal-footer">
            <button class="btn btn-ghost" @click="deleteNet=null">取消</button>
            <button class="btn btn-danger" @click="doDeleteNet" :disabled="deleting">
              <div v-if="deleting" class="spinner" style="width:12px;height:12px;border-width:2px"></div>删除</button>
          </div>
        </div>
      </div>

      <!-- Delete Volume -->
      <div v-if="deleteVol" class="modal-overlay" @click.self="deleteVol=null">
        <div class="modal" style="max-width:400px">
          <div class="modal-header"><div class="modal-title"><Trash2 :size="16"/> 删除存储卷</div>
            <button class="modal-close" @click="deleteVol=null"><X :size="15"/></button></div>
          <div class="modal-body">
            <p style="color:var(--text-secondary);font-size:14px">确认{{ deleteVolForce?'强制':'' }}删除 <strong style="color:var(--text-primary)">{{ deleteVol.name }}</strong>？</p>
            <p v-if="deleteVolForce" style="color:var(--red);font-size:12.5px;margin-top:8px">⚠ 强制删除可能影响正在使用该卷的容器</p>
          </div>
          <div class="modal-footer">
            <button class="btn btn-ghost" @click="deleteVol=null">取消</button>
            <button class="btn btn-danger" @click="doDeleteVol" :disabled="deleting">
              <div v-if="deleting" class="spinner" style="width:12px;height:12px;border-width:2px"></div>
              {{ deleteVolForce?'强制删除':'确认删除' }}</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { Network, Database, Plus, RefreshCw, Trash2, X, Zap } from 'lucide-vue-next'
import api from '@/api'
import { useToastStore } from '@/stores/toast'
const toast = useToastStore()
const tab = ref('network')
const networks = ref([]), volumes = ref([])
const netLoading = ref(true), volLoading = ref(true)
const showCreateNet = ref(false), showCreateVol = ref(false)
const newNet = ref({ name: '', driver: 'bridge' }), newVolName = ref('')
const creating = ref(false), deleting = ref(false)
const deleteNet = ref(null), deleteVol = ref(null), deleteVolForce = ref(false)

function driverBadge(d) { return { bridge: 'badge-cyan', host: 'badge-amber', overlay: 'badge-purple', none: 'badge-muted' }[d] || 'badge-muted' }
function driverDesc(d) {
  return { bridge: '默认网络驱动，适用于同一主机上容器互联', host: '容器直接使用宿主机网络，无隔离', overlay: '跨宿主机容器网络，适用于 Swarm 集群', macvlan: '为容器分配 MAC 地址，表现为物理设备', ipvlan: '共享宿主机 MAC，支持更多网络场景', none: '禁用所有网络连接' }[d] || ''
}
function fmtDate(d) { if(!d)return'—'; try{return new Date(d).toLocaleString('zh-CN',{year:'numeric',month:'2-digit',day:'2-digit',hour:'2-digit',minute:'2-digit'})}catch{return d} }
async function loadNetworks() { netLoading.value=true; try{const r=await api.listNetworks();networks.value=r.data||[]}catch{toast.error('加载网络失败')}finally{netLoading.value=false} }
async function loadVolumes() { volLoading.value=true; try{const r=await api.listVolumes();volumes.value=r.data||[]}catch{toast.error('加载存储卷失败')}finally{volLoading.value=false} }
async function pruneNetworks() { if(!confirm('确认清理所有未使用网络？'))return; try{await api.pruneNetworks();toast.success('清理完成');loadNetworks()}catch(e){toast.error(e)} }
async function pruneVolumes() { if(!confirm('确认清理所有未使用存储卷？此操作不可逆！'))return; try{await api.pruneVolumes();toast.success('清理完成');loadVolumes()}catch(e){toast.error(e)} }
async function doCreateNet() { creating.value=true; try{await api.createNetwork({name:newNet.value.name,driver:newNet.value.driver});toast.success('网络已创建');showCreateNet.value=false;newNet.value={name:'',driver:'bridge'};loadNetworks()}catch(e){toast.error(typeof e==='string'?e:'创建失败')}finally{creating.value=false} }
async function doCreateVol() { creating.value=true; try{await api.createVolume({name:newVolName.value});toast.success('存储卷已创建');showCreateVol.value=false;newVolName.value='';loadVolumes()}catch(e){toast.error(typeof e==='string'?e:'创建失败')}finally{creating.value=false} }
function confirmDeleteNet(net){deleteNet.value=net}
function confirmDeleteVol(vol,force){deleteVol.value=vol;deleteVolForce.value=force}
async function doDeleteNet() { deleting.value=true; try{await api.deleteNetwork(deleteNet.value.name);toast.success('网络已删除');deleteNet.value=null;loadNetworks()}catch(e){toast.error(typeof e==='string'?e:'删除失败')}finally{deleting.value=false} }
async function doDeleteVol() { deleting.value=true; try{await api.deleteVolume(deleteVol.value.name,deleteVolForce.value);toast.success('存储卷已删除');deleteVol.value=null;loadVolumes()}catch(e){toast.error(typeof e==='string'?e:'删除失败')}finally{deleting.value=false} }
onMounted(()=>{loadNetworks();loadVolumes()})
</script>

<style scoped>
.ns-tabs{display:flex;gap:4px;background:var(--bg-card);border:1px solid var(--border);border-radius:var(--radius-lg);padding:4px;width:fit-content;margin-bottom:24px}
.ns-tab{display:flex;align-items:center;gap:7px;padding:8px 18px;border-radius:var(--radius);font-size:13.5px;font-weight:500;color:var(--text-muted);background:transparent;cursor:pointer;transition:all var(--transition)}
.ns-tab:hover{color:var(--text-secondary)}
.ns-tab.active{background:rgba(6,182,212,0.1);color:var(--accent-light);border:1px solid var(--border-3)}
.section-header{display:flex;align-items:center;justify-content:space-between;margin-bottom:16px}
.section-count{font-size:13px;color:var(--text-muted);font-weight:500}
.net-name{font-weight:600;color:var(--text-primary);font-size:13.5px}
.vol-name{font-family:var(--font-mono);font-size:12.5px;color:var(--text-primary);word-break:break-all}
.font-mono{font-family:var(--font-mono);font-size:12px;color:var(--text-secondary)}
.muted{color:var(--text-muted)!important}
.muted-sm{font-size:12px;color:var(--text-muted)}
.btn-danger-icon{color:var(--red)!important}
.btn-danger-icon:hover{background:rgba(240,84,100,0.1)!important;border-color:rgba(240,84,100,0.25)!important}
.btn-danger-icon:disabled{opacity:0.3;cursor:not-allowed}
.badge-purple{background:rgba(167,139,250,0.1);color:var(--purple);border:1px solid rgba(167,139,250,0.2)}
.badge-amber{background:rgba(245,158,11,0.1);color:var(--amber);border:1px solid rgba(245,158,11,0.2)}
.driver-desc{padding:8px 12px;background:var(--bg-input);border-radius:var(--radius);font-size:12px;color:var(--text-muted);line-height:1.5}

@media (max-width: 768px) {
  .ns-tabs { width: 100%; }
  .ns-tab { flex: 1; justify-content: center; padding: 8px 10px; font-size: 13px; }
  .section-header {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
    margin-bottom: 14px;
  }
  .section-header > div { justify-content: flex-end; }
  /* 表格横向滚动 */
  .card { overflow-x: auto !important; -webkit-overflow-scrolling: touch; }
  .data-table { min-width: 600px; }
}
</style>
