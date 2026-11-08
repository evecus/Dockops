<template>
  <div class="images-page">
    <div class="page-header">
      <div class="img-stats">
        <div class="img-stat"><span class="img-stat-val">{{ images.length }}</span><span class="img-stat-label">总镜像</span></div>
        <div class="img-stat-sep"></div>
        <div class="img-stat"><span class="img-stat-val" style="color:var(--green)">{{ usedCount }}</span><span class="img-stat-label">使用中</span></div>
        <div class="img-stat-sep"></div>
        <div class="img-stat"><span class="img-stat-val" style="color:var(--accent)">{{ totalSize }}</span><span class="img-stat-label">总大小</span></div>
      </div>
      <div style="display:flex;gap:10px">
        <button class="btn btn-ghost" @click="load"><RefreshCw :size="14" /> 刷新</button>
        <button class="btn btn-ghost" @click="showLoad = true"><Upload :size="14" /> 导入镜像</button>
        <button class="btn btn-primary" @click="showPull = true"><Download :size="14" /> 拉取镜像</button>
      </div>
    </div>

    <div class="search-row">
      <div class="search-box">
        <Search :size="13" style="color:var(--text-muted)" />
        <input v-model="search" class="search-input" placeholder="搜索镜像名称或 ID..." />
        <button v-if="search" class="search-clear" @click="search=''"><X :size="12" /></button>
      </div>
    </div>

    <div v-if="loading" class="empty-state"><div class="spinner"></div></div>
    <div v-else-if="!filtered.length" class="empty-state">
      <HardDrive :size="48" /><p>{{ search ? '未找到匹配镜像' : '暂无本地镜像' }}</p>
    </div>
    <div v-else class="image-grid">
      <div v-for="img in filtered" :key="img.id" class="img-card">
        <div class="img-card-top">
          <div class="img-icon"><HardDrive :size="18" style="color:var(--accent);opacity:0.8" /></div>
          <span class="tag" style="font-size:10px">{{ img.id }}</span>
        </div>
        <div class="img-tags">
          <template v-if="img.repo_tags?.length && img.repo_tags[0] !== '<none>:<none>'">
            <div v-for="tag in img.repo_tags" :key="tag" class="img-tag-row">
              <span class="img-repo">{{ repoName(tag) }}</span>
              <span class="badge badge-cyan" style="font-size:10px">{{ tagVer(tag) }}</span>
            </div>
          </template>
          <span v-else class="sep" style="font-size:12px;font-style:italic">无标签</span>
        </div>
        <div class="img-meta">
          <div class="img-meta-item"><Database :size="11" /><span>{{ fmtSize(img.size) }}</span></div>
          <div class="img-meta-item"><Clock :size="11" /><span>{{ fmtDate(img.created) }}</span></div>
        </div>
        <div class="img-actions">
          <button class="btn btn-ghost btn-sm" style="flex:1;justify-content:center" @click="repull(img)"
            :disabled="!!updateStatus[img.id]">
            <component :is="updateStatus[img.id] ? RefreshCw : DownloadCloud" :size="13"
              :class="updateStatus[img.id] ? 'spin' : ''"/>
            更新
          </button>
          <button class="btn btn-danger btn-sm" style="flex:1;justify-content:center" @click="confirmDelete(img)"
            :disabled="!!updateStatus[img.id]">
            <Trash2 :size="13"/> 删除
          </button>
        </div>
        <!-- Update progress -->
        <div v-if="updateStatus[img.id]" class="update-progress">
          <div class="update-progress-bar">
            <div class="update-progress-fill"
              :class="updateStatus[img.id].phase === 'error' ? 'fill-error' :
                      updateStatus[img.id].phase === 'done' ? 'fill-done' : 'fill-active'">
            </div>
          </div>
          <div class="update-msg" :class="updateStatus[img.id].phase">
            {{ updateStatus[img.id].msg }}
          </div>
        </div>
      </div>
    </div>

    <Teleport to="body">
      <!-- Pull Modal -->
      <div v-if="showPull" class="modal-overlay" @click.self="closePull">
        <div class="modal" style="max-width:500px">
          <div class="modal-header">
            <div class="modal-title"><Download :size="16"/> 拉取镜像</div>
            <button class="modal-close" @click="closePull"><X :size="15"/></button>
          </div>
          <div class="modal-body">
            <div class="form-group">
              <label class="form-label">镜像名称</label>
              <input v-model="pullRef" class="form-input" placeholder="nginx:latest / ghcr.io/user/image:tag"
                @keyup.enter="doPull" autofocus />
            </div>
            <div v-if="pullLog" class="pull-log"><pre>{{ pullLog }}</pre></div>
          </div>
          <div class="modal-footer">
            <button class="btn btn-ghost" @click="closePull">{{ pullDone ? '关闭' : '取消' }}</button>
            <button v-if="!pullDone" class="btn btn-primary" @click="doPull" :disabled="pulling==='pull'||!pullRef.trim()">
              <div v-if="pulling==='pull'" class="spinner" style="width:13px;height:13px;border-width:2px"></div>
              <Download v-else :size="14"/> 开始拉取
            </button>
            <button v-else class="btn btn-success" @click="closePull"><CheckCircle :size="14"/> 完成</button>
          </div>
        </div>
      </div>

      <!-- Load Modal -->
      <div v-if="showLoad" class="modal-overlay" @click.self="showLoad=false">
        <div class="modal" style="max-width:460px">
          <div class="modal-header">
            <div class="modal-title"><Upload :size="16"/> 导入镜像</div>
            <button class="modal-close" @click="showLoad=false"><X :size="15"/></button>
          </div>
          <div class="modal-body">
            <div class="upload-zone" :class="{dragging:loadDragging}"
              @dragover.prevent="loadDragging=true" @dragleave="loadDragging=false"
              @drop.prevent="onLoadDrop">
              <Upload :size="28" style="opacity:0.4;margin-bottom:10px"/>
              <p style="font-size:13px;color:var(--text-secondary)">拖拽 .tar 镜像文件到此处</p>
              <label class="btn btn-ghost btn-sm" style="margin-top:10px;cursor:pointer">
                选择文件<input type="file" accept=".tar,.tar.gz,.tgz" @change="onLoadFile" style="display:none"/>
              </label>
            </div>
            <div v-if="loadFile" class="load-file-info">
              <File :size="14" style="color:var(--accent)"/>
              <span>{{ loadFile.name }}</span>
              <span class="sep">{{ fmtSize(loadFile.size) }}</span>
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn btn-ghost" @click="showLoad=false">取消</button>
            <button class="btn btn-primary" @click="doLoad" :disabled="!loadFile||loadingFile">
              <div v-if="loadingFile" class="spinner" style="width:13px;height:13px;border-width:2px"></div>
              导入
            </button>
          </div>
        </div>
      </div>

      <!-- Delete Confirm -->
      <div v-if="deletingImg" class="modal-overlay" @click.self="deletingImg=null">
        <div class="modal" style="max-width:420px">
          <div class="modal-header">
            <div class="modal-title"><Trash2 :size="16"/> 删除镜像</div>
            <button class="modal-close" @click="deletingImg=null"><X :size="15"/></button>
          </div>
          <div class="modal-body">
            <p style="font-size:13.5px;color:var(--text-secondary);margin-bottom:14px">
              确认删除 <strong style="color:var(--text-primary)">{{ deletingImg.repo_tags?.[0]||deletingImg.id }}</strong>？
            </p>
            <label style="display:flex;align-items:center;gap:6px;font-size:13px;cursor:pointer">
              <input type="checkbox" v-model="forceDelete"/>
              <span>强制删除（即使容器正在使用）</span>
            </label>
          </div>
          <div class="modal-footer">
            <button class="btn btn-ghost" @click="deletingImg=null">取消</button>
            <button class="btn btn-danger" @click="doDelete" :disabled="deleting">
              <div v-if="deleting" class="spinner" style="width:12px;height:12px;border-width:2px"></div>
              {{ forceDelete?'强制删除':'确认删除' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { RefreshCw, Upload, Download, Search, X, HardDrive, Trash2, Database, Clock, DownloadCloud, File, CheckCircle } from 'lucide-vue-next'
import api from '@/api'
import { useToastStore } from '@/stores/toast'
const toast = useToastStore()
const images = ref([]), loading = ref(true), search = ref(''), pulling = ref(null)
// updateStatus: { [imgId]: { phase: 'checking'|'pulling'|'done'|'error', msg: string } }
const updateStatus = ref({})
const showPull = ref(false), showLoad = ref(false), pullRef = ref(''), pullLog = ref(''), pullDone = ref(false)
const deletingImg = ref(null), deleting = ref(false), forceDelete = ref(false)
const loadFile = ref(null), loadDragging = ref(false), loadingFile = ref(false)
const usedCount = computed(()=>images.value.filter(i=>i.containers>0).length)
const totalSize = computed(()=>fmtSize(images.value.reduce((s,i)=>s+(i.size||0),0)))
const filtered = computed(()=>{
  if(!search.value)return images.value
  const q=search.value.toLowerCase()
  return images.value.filter(img=>img.id.toLowerCase().includes(q)||img.repo_tags?.some(t=>t.toLowerCase().includes(q)))
})
function repoName(tag){const p=tag.split(':')[0].split('/');return p[p.length-1]}
function tagVer(tag){const p=tag.split(':');return p.length>1?p[1]:'latest'}
function fmtSize(b){if(!b)return'0 B';if(b>=1e9)return(b/1e9).toFixed(2)+' GB';if(b>=1e6)return(b/1e6).toFixed(0)+' MB';if(b>=1e3)return(b/1e3).toFixed(0)+' KB';return b+' B'}
function fmtDate(ts){if(!ts)return'—';return new Date(ts*1000).toLocaleDateString('zh-CN')}
async function load(){loading.value=true;try{const r=await api.listImages();images.value=r.data||[]}catch{toast.error('加载镜像失败')}finally{loading.value=false}}
async function doPull(){
  if(!pullRef.value.trim()||pulling.value==='pull')return
  pulling.value='pull';pullLog.value=`拉取 ${pullRef.value}...\n`
  try{
    const resp=await fetch('/api/images/pull',{method:'POST',headers:{'Content-Type':'application/json',Authorization:`Bearer ${localStorage.getItem('token')}`},body:JSON.stringify({image:pullRef.value})})
    const reader=resp.body.getReader();const dec=new TextDecoder()
    while(true){const{done,value}=await reader.read();if(done)break
      dec.decode(value).split('\n').forEach(line=>{if(line.startsWith('data:')){const d=line.slice(5).trim();try{const o=JSON.parse(d);if(o.status)pullLog.value+=o.status+(o.progress?' '+o.progress:'')+'\n'}catch{if(d)pullLog.value+=d+'\n'}}})}
    pullDone.value=true;pullLog.value+='\n✓ 拉取完成！';toast.success('镜像拉取成功');load()
  }catch(e){pullLog.value+='\n✗ 拉取失败: '+e;toast.error('拉取失败')}finally{pulling.value=null}
}
function closePull(){showPull.value=false;pullRef.value='';pullLog.value='';pullDone.value=false}
async function repull(img) {
  const tag = img.repo_tags?.[0]
  if (!tag || tag === '<none>:<none>') { toast.error('无法更新无标签镜像'); return }

  const setStatus = (phase, msg) => {
    updateStatus.value = { ...updateStatus.value, [img.id]: { phase, msg } }
  }
  const clearStatus = () => {
    const s = { ...updateStatus.value }
    delete s[img.id]
    updateStatus.value = s
  }

  setStatus('checking', '正在检测版本...')
  try {
    const resp = await api.checkImageUpdate(tag)
    const reader = resp.body.getReader()
    const decoder = new TextDecoder()
    let buffer = ''

    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      buffer += decoder.decode(value, { stream: true })
      const parts = buffer.split('\n\n')
      buffer = parts.pop()
      for (const part of parts) {
        let eventType = '', data = ''
        for (const line of part.split('\n')) {
          if (line.startsWith('event:')) eventType = line.slice(6).trim()
          if (line.startsWith('data:')) data = line.slice(5).trim()
        }
        if (!data) continue
        if (eventType === 'checking') setStatus('checking', data)
        else if (eventType === 'pulling') setStatus('pulling', data)
        else if (eventType === 'progress') setStatus('pulling', '正在拉取镜像...')
        else if (eventType === 'up-to-date') {
          setStatus('done', data)
          toast.success(data)
          setTimeout(clearStatus, 3000)
        } else if (eventType === 'updated') {
          setStatus('done', data)
          toast.success(data)
          setTimeout(() => { clearStatus(); load() }, 2000)
        } else if (eventType === 'error') {
          setStatus('error', data)
          toast.error(data)
          setTimeout(clearStatus, 4000)
        }
      }
    }
  } catch(e) {
    setStatus('error', '更新失败: ' + e.message)
    setTimeout(clearStatus, 4000)
  }
}
function confirmDelete(img){deletingImg.value=img;forceDelete.value=false}
async function doDelete(){
  if(!deletingImg.value)return;deleting.value=true
  try{await api.deleteImage(deletingImg.value.repo_tags?.[0]||deletingImg.value.id,forceDelete.value);toast.success('镜像已删除');deletingImg.value=null;load()}
  catch(e){toast.error(typeof e==='string'?e:'删除失败')}finally{deleting.value=false}
}
function onLoadDrop(e){loadDragging.value=false;loadFile.value=e.dataTransfer.files[0]||null}
function onLoadFile(e){loadFile.value=e.target.files[0]||null}
async function doLoad(){
  if(!loadFile.value)return;loadingFile.value=true
  try{await api.loadImage(loadFile.value);toast.success('镜像导入成功');showLoad.value=false;loadFile.value=null;load()}
  catch(e){toast.error('导入失败: '+e)}finally{loadingFile.value=false}
}
onMounted(load)
</script>

<style scoped>
.page-header{display:flex;align-items:center;justify-content:space-between;margin-bottom:20px}
.img-stats{display:flex;align-items:center;background:var(--bg-card);border:1px solid var(--border);border-radius:var(--radius-lg);padding:12px 20px}
.img-stat{display:flex;flex-direction:column;align-items:center;gap:2px;padding:0 18px}
.img-stat-val{font-size:22px;font-weight:700;line-height:1}
.img-stat-label{font-size:11px;color:var(--text-muted);font-weight:500}
.img-stat-sep{width:1px;background:var(--border);height:28px}
.search-row{margin-bottom:20px}
.search-box{display:flex;align-items:center;gap:8px;background:var(--bg-card);border:1px solid var(--border);border-radius:var(--radius);padding:9px 14px;max-width:400px;transition:all var(--transition)}
.search-box:focus-within{border-color:var(--accent);box-shadow:0 0 0 3px var(--accent-glow)}
.search-input{background:none;border:none;color:var(--text-primary);font-size:13px;flex:1}
.search-clear{background:none;color:var(--text-muted);display:flex;padding:2px;border-radius:4px;cursor:pointer}
.image-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(270px,1fr));gap:14px}
.img-card{background:var(--bg-card);border:1px solid var(--border);border-radius:var(--radius-lg);padding:16px;display:flex;flex-direction:column;gap:12px;transition:all var(--transition);position:relative;overflow:hidden}
.img-card::after{content:'';position:absolute;bottom:0;left:0;right:0;height:1px;background:linear-gradient(90deg,transparent,var(--accent),transparent);opacity:0;transition:opacity var(--transition)}
.img-card:hover{border-color:var(--border-2);box-shadow:var(--shadow-cyan);transform:translateY(-2px)}
.img-card:hover::after{opacity:0.4}
.img-card-top{display:flex;align-items:center;justify-content:space-between}
.img-icon{width:34px;height:34px;background:rgba(6,182,212,0.08);border:1px solid var(--border);border-radius:var(--radius);display:flex;align-items:center;justify-content:center}
.img-tags{display:flex;flex-direction:column;gap:6px;min-height:36px}
.img-tag-row{display:flex;align-items:center;justify-content:space-between;gap:8px}
.img-repo{font-size:14px;font-weight:600;color:var(--text-primary);word-break:break-all}
.img-meta{display:flex;align-items:center;gap:14px}
.img-meta-item{display:flex;align-items:center;gap:4px;font-size:11.5px;color:var(--text-muted)}
.img-actions{display:flex;gap:6px;padding-top:4px;border-top:1px solid var(--border)}
.pull-log{margin-top:14px;background:var(--bg-base);border:1px solid var(--border);border-radius:var(--radius);padding:12px;max-height:200px;overflow-y:auto}
.pull-log pre{font-family:var(--font-mono);font-size:11.5px;color:var(--text-code);white-space:pre-wrap;line-height:1.6}
.upload-zone{border:2px dashed var(--border-2);border-radius:var(--radius-lg);padding:36px;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;transition:all var(--transition)}
.upload-zone.dragging{border-color:var(--accent);background:var(--accent-dim)}
.load-file-info{display:flex;align-items:center;gap:8px;margin-top:12px;padding:8px 12px;background:var(--bg-input);border-radius:var(--radius);font-size:13px;color:var(--text-secondary)}
.spin{animation:spin 0.8s linear infinite}

@media (max-width: 768px) {
  .page-header {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
    margin-bottom: 16px;
  }
  .page-header > div { justify-content: flex-end; }
  .img-stats {
    justify-content: space-around;
    padding: 10px 12px;
  }
  .img-stat { padding: 0 8px; }
  .search-box { max-width: 100%; }
  .image-grid {
    grid-template-columns: 1fr;
    gap: 12px;
  }
}
.update-progress {
  padding: 8px 0 2px;
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.update-progress-bar {
  height: 3px;
  background: var(--border);
  border-radius: 99px;
  overflow: hidden;
}
.update-progress-fill {
  height: 100%;
  border-radius: 99px;
  transition: width 0.3s;
}
.fill-active {
  width: 100%;
  background: linear-gradient(90deg, var(--accent), var(--accent-light));
  animation: progress-slide 1.5s infinite;
}
.fill-done {
  width: 100%;
  background: var(--green);
  animation: none;
}
.fill-error {
  width: 100%;
  background: var(--red);
  animation: none;
}
.update-msg {
  font-size: 11px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}
.update-msg.done { color: var(--green); }
.update-msg.error { color: var(--red); }
.update-msg.pulling, .update-msg.checking { color: var(--accent); }
@keyframes progress-slide {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(200%); }
}
</style>