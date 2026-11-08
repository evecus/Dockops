<template>
  <div class="modal modal-xl" style="height:82vh;max-height:760px">
    <div class="modal-header">
      <div class="modal-title"><FolderOpen :size="16" /> 文件管理 — {{ container.name }}</div>
      <button class="modal-close" @click="$emit('close')"><X :size="15" /></button>
    </div>

    <!-- Breadcrumb + Toolbar -->
    <div class="files-toolbar">
      <div class="breadcrumb">
        <button class="crumb-item" @click="navigate('/')">/</button>
        <template v-for="(seg, i) in pathSegs" :key="i">
          <ChevronRight :size="12" style="color:var(--text-muted);flex-shrink:0" />
          <button class="crumb-item" @click="navigate(pathSegs.slice(0,i+1).join('/') || '/')">{{ seg }}</button>
        </template>
      </div>
      <div class="files-actions">
        <button class="btn btn-ghost btn-sm" @click="loadFiles"><RefreshCw :size="13" /></button>
        <label class="btn btn-ghost btn-sm" style="cursor:pointer">
          <Upload :size="13" /> 上传文件
          <input type="file" @change="uploadFile" style="display:none" />
        </label>
        <button class="btn btn-ghost btn-sm" @click="newFolderPrompt"><FolderPlus :size="13" /> 新建文件夹</button>
      </div>
    </div>

    <!-- File List -->
    <div class="files-body">
      <div v-if="loading" class="empty-state"><div class="spinner"></div></div>
      <div v-else-if="!entries.length" class="empty-state">
        <FolderOpen :size="36" /><p>空目录</p>
      </div>
      <table v-else class="data-table files-table">
        <thead>
          <tr>
            <th style="width:32px"></th>
            <th>名称</th>
            <th>大小</th>
            <th>权限</th>
            <th>修改时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <!-- Up dir -->
          <tr v-if="currentPath !== '/' && currentPath !== ''" class="file-row up-row" @click="goUp">
            <td><Folder :size="15" style="color:var(--accent)" /></td>
            <td colspan="4" style="color:var(--text-muted);font-style:italic">..</td>
            <td></td>
          </tr>
          <tr v-for="entry in sortedEntries" :key="entry.path"
            class="file-row"
            :class="{ selected: selected === entry.path }"
            @click="onRowClick(entry)"
            @dblclick="onDblClick(entry)">
            <td>
              <component :is="entry.is_dir ? Folder : getFileIcon(entry.name)" :size="15"
                :style="`color:${entry.is_dir ? 'var(--accent)' : 'var(--text-muted)'}`" />
            </td>
            <td>
              <span class="file-name">{{ entry.name }}</span>
            </td>
            <td>
              <span class="file-meta">{{ entry.is_dir ? '—' : fmtSize(entry.size) }}</span>
            </td>
            <td>
              <span class="tag" style="font-size:10px">{{ entry.mode }}</span>
            </td>
            <td>
              <span class="file-meta">{{ fmtDate(entry.mod_time) }}</span>
            </td>
            <td>
              <div class="file-row-actions" @click.stop>
                <a v-if="!entry.is_dir" :href="api.downloadContainerFile(container.name, entry.path)"
                  target="_blank" class="btn btn-icon" data-tip="下载">
                  <Download :size="13" />
                </a>
                <button class="btn btn-icon btn-danger-icon" @click="deleteEntry(entry)" data-tip="删除">
                  <Trash2 :size="13" />
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Preview pane -->
    <div v-if="preview" class="preview-pane">
      <div class="preview-header">
        <span class="preview-name">{{ preview.name }}</span>
        <button class="btn btn-icon" @click="preview=null"><X :size="13" /></button>
      </div>
      <pre class="preview-content">{{ preview.content }}</pre>
    </div>

    <div class="modal-footer" style="justify-content:space-between">
      <span class="file-meta">{{ currentPath }}</span>
      <button class="btn btn-ghost" @click="$emit('close')">关闭</button>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import {
  FolderOpen, Folder, X, RefreshCw, Upload, FolderPlus,
  ChevronRight, Download, Trash2, File, FileText, FileCode, Image
} from 'lucide-vue-next'
import api from '@/api'
import { useToastStore } from '@/stores/toast'

const props = defineProps({ container: Object })
defineEmits(['close'])
const toast = useToastStore()

const currentPath = ref('/')
const entries = ref([])
const loading = ref(false)
const selected = ref(null)
const preview = ref(null)

const pathSegs = computed(() =>
  currentPath.value.split('/').filter(Boolean)
)

const sortedEntries = computed(() =>
  [...entries.value].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1
    return a.name.localeCompare(b.name)
  })
)

function getFileIcon(name) {
  const ext = name.split('.').pop().toLowerCase()
  if (['jpg','jpeg','png','gif','svg','webp'].includes(ext)) return Image
  if (['js','ts','py','go','rs','cpp','c','java','sh','yaml','yml','json','toml','conf','md'].includes(ext)) return FileCode
  if (['txt','log','csv'].includes(ext)) return FileText
  return File
}

function fmtSize(b) {
  if (b >= 1e9) return (b/1e9).toFixed(1)+' GB'
  if (b >= 1e6) return (b/1e6).toFixed(1)+' MB'
  if (b >= 1e3) return (b/1e3).toFixed(0)+' KB'
  return b+' B'
}

function fmtDate(ts) {
  if (!ts) return '—'
  return new Date(ts * 1000).toLocaleString('zh-CN', { month:'2-digit', day:'2-digit', hour:'2-digit', minute:'2-digit' })
}

async function loadFiles() {
  loading.value = true
  try {
    const res = await api.listContainerFiles(props.container.name, currentPath.value)
    entries.value = res.data || []
  } catch (e) { toast.error('加载文件失败: ' + e) }
  finally { loading.value = false }
}

function navigate(path) {
  currentPath.value = path.startsWith('/') ? path : '/' + path
  selected.value = null
  preview.value = null
  loadFiles()
}

function goUp() {
  const parts = currentPath.value.split('/').filter(Boolean)
  parts.pop()
  navigate('/' + parts.join('/') || '/')
}

function onRowClick(entry) {
  selected.value = entry.path
}

async function onDblClick(entry) {
  if (entry.is_dir) {
    navigate(entry.path)
  } else {
    // Try to preview text files
    const ext = entry.name.split('.').pop().toLowerCase()
    const textExts = ['txt','log','yaml','yml','json','toml','conf','md','sh','py','js','ts','go','env','ini','cfg','xml']
    if (textExts.includes(ext) && entry.size < 500000) {
      try {
        const url = api.downloadContainerFile(props.container.name, entry.path)
        const res = await fetch(url)
        const text = await res.text()
        preview.value = { name: entry.name, content: text }
      } catch {}
    }
  }
}

async function uploadFile(e) {
  const file = e.target.files[0]
  if (!file) return
  try {
    await api.uploadContainerFile(props.container.name, currentPath.value, file)
    toast.success('上传成功')
    loadFiles()
  } catch (err) { toast.error('上传失败: ' + err) }
}

async function deleteEntry(entry) {
  if (!confirm(`确认删除 ${entry.name}?`)) return
  try {
    await api.deleteContainerFile(props.container.name, entry.path)
    toast.success('已删除')
    loadFiles()
  } catch (e) { toast.error('删除失败: ' + e) }
}

function newFolderPrompt() {
  const name = prompt('请输入文件夹名称:')
  if (!name) return
  // Create via mkdir exec - upload a dummy tar with empty dir
  toast.info('新建文件夹功能需要容器支持 mkdir 命令')
}

onMounted(loadFiles)
</script>

<style scoped>
.files-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-base);
}
.breadcrumb { display: flex; align-items: center; gap: 3px; }
.crumb-item {
  padding: 2px 5px;
  border-radius: 4px;
  font-size: 12.5px;
  font-family: var(--font-mono);
  color: var(--text-secondary);
  background: none;
  cursor: pointer;
  transition: all var(--transition);
}
.crumb-item:hover { background: var(--accent-dim); color: var(--accent-light); }
.files-actions { display: flex; align-items: center; gap: 6px; }

.files-body {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}
.files-table { font-size: 13px; }
.file-row { cursor: pointer; }
.file-row:hover td { background: rgba(6,182,212,0.03) !important; }
.file-row.selected td { background: rgba(6,182,212,0.06) !important; }
.up-row:hover td { background: rgba(6,182,212,0.03) !important; }

.file-name { font-weight: 500; color: var(--text-primary); }
.file-meta { font-size: 11.5px; color: var(--text-muted); font-family: var(--font-mono); }
.file-row-actions { display: flex; gap: 3px; opacity: 0; transition: opacity var(--transition); }
.file-row:hover .file-row-actions { opacity: 1; }
.btn-danger-icon { color: var(--red) !important; }
.btn-danger-icon:hover { background: rgba(240,84,100,0.1) !important; }

.preview-pane {
  border-top: 1px solid var(--border);
  background: var(--bg-base);
  max-height: 200px;
  display: flex;
  flex-direction: column;
}
.preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.preview-name { font-size: 12px; font-family: var(--font-mono); color: var(--text-secondary); }
.preview-content {
  flex: 1;
  overflow: auto;
  padding: 10px 14px;
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--text-code);
  white-space: pre;
  line-height: 1.6;
}

@media (max-width: 768px) {
  .files-toolbar { flex-wrap: wrap; gap: 8px; padding: 8px 10px; }
  .breadcrumb { flex-wrap: wrap; }
  .files-actions { flex-wrap: wrap; justify-content: flex-end; }
  .file-row-actions { opacity: 1; } /* 移动端始终显示操作按钮 */
  .files-table { font-size: 12px; }
  .file-meta { display: none; } /* 移动端隐藏文件大小/时间，减少拥挤 */
}
</style>
