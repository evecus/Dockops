<template>
  <div class="modal modal-xl">
    <div class="modal-header">
      <div class="modal-title"><Plus :size="16" /> 创建容器</div>
      <button class="modal-close" @click="$emit('close')"><X :size="15" /></button>
    </div>

    <div class="modal-body" style="padding:0">
      <div class="create-layout">
        <!-- Left: mode selector -->
        <div class="create-sidebar">
          <div class="sidebar-section">
            <div class="form-label" style="margin-bottom:8px">创建方式</div>
            <div class="mode-list">
              <div v-for="m in modes" :key="m.id"
                class="mode-item" :class="{ active: mode === m.id }"
                @click="mode = m.id">
                <component :is="m.icon" :size="15" />
                <div>
                  <div class="mode-name">{{ m.label }}</div>
                  <div class="mode-desc">{{ m.desc }}</div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Right: content -->
        <div class="create-main">

          <!-- Upload mode -->
          <div v-if="mode === 'upload'" class="mode-panel">
            <div class="upload-zone" :class="{ dragging }"
              @dragover.prevent="dragging=true" @dragleave="dragging=false"
              @drop.prevent="onDrop">
              <Upload :size="32" style="opacity:0.4;margin-bottom:12px" />
              <p style="font-size:14px;color:var(--text-secondary)">拖拽 YAML 文件到此处</p>
              <p style="font-size:12px;color:var(--text-muted);margin-top:4px">或</p>
              <label class="btn btn-ghost" style="margin-top:8px;cursor:pointer">
                选择文件
                <input type="file" accept=".yml,.yaml" @change="onFileSelect" style="display:none" />
              </label>
            </div>
            <div v-if="composeContent" class="compose-preview">
              <div class="compose-preview-header">
                <span>预览</span>
                <button class="btn btn-ghost btn-sm" @click="composeContent=''">清空</button>
              </div>
              <pre class="code-block" style="max-height:300px">{{ composeContent }}</pre>
            </div>
          </div>

          <!-- Paste mode -->
          <div v-if="mode === 'paste'" class="mode-panel">
            <label class="form-label">粘贴 Compose 内容</label>
            <textarea v-model="composeContent" class="form-textarea"
              style="min-height:400px;font-size:12.5px"
              placeholder="version: '3.8'
services:
  app:
    image: nginx:latest
    container_name: my-nginx
    ports:
      - '80:80'
    restart: unless-stopped"></textarea>
          </div>

          <!-- Docker Run mode -->
          <div v-if="mode === 'run'" class="mode-panel">
            <div class="form-group">
              <label class="form-label">粘贴 docker run 命令</label>
              <textarea v-model="runCmd" class="form-textarea" style="min-height:100px;font-size:12.5px"
                placeholder="docker run -d --name nginx -p 80:80 -v /data:/data --restart unless-stopped nginx:latest"></textarea>
              <div style="display:flex;gap:8px;margin-top:8px">
                <button class="btn btn-ghost" style="flex:1;justify-content:center" @click="parseRun" :disabled="parsing">
                  <component :is="parsing ? RefreshCw : Wand2" :size="14" :class="parsing?'spin':''" />
                  {{ parsing ? '解析中...' : '预览解析结果' }}
                </button>
                <span style="font-size:11px;color:var(--text-muted);align-self:center">也可直接点「创建并启动」</span>
              </div>
            </div>
            <div v-if="parsedYaml" class="compose-preview">
              <div class="compose-preview-header">
                <span>生成的 Compose 配置（可编辑）</span>
                <button class="btn btn-ghost btn-sm" @click="parsedYaml='';composeContent=''">重新解析</button>
              </div>
              <textarea v-model="parsedYaml" class="form-textarea" style="min-height:280px;font-size:12.5px"
                @input="composeContent = parsedYaml"></textarea>
            </div>
          </div>

          <!-- Form mode -->
          <div v-if="mode === 'form'" class="mode-panel">
            <!-- Container name in form mode -->
            <div class="form-group">
              <label class="form-label">容器名称 *</label>
              <input v-model="formName" class="form-input" placeholder="my-app" />
            </div>

            <div class="form-tabs">
              <div class="tabs">
                <div v-for="t in formTabs" :key="t.id" class="tab" :class="{ active: formTab === t.id }" @click="formTab = t.id">
                  {{ t.label }}
                </div>
              </div>
            </div>

            <div v-if="formTab === 'basic'" class="tab-content">
              <div class="grid-2">
                <div class="form-group">
                  <label class="form-label">镜像 *</label>
                  <input v-model="formFields.image" class="form-input" placeholder="nginx:latest" />
                </div>
                <div class="form-group">
                  <label class="form-label">重启策略</label>
                  <select v-model="formFields.restart" class="form-select">
                    <option value="">不重启</option>
                    <option value="always">always</option>
                    <option value="unless-stopped">unless-stopped</option>
                    <option value="on-failure">on-failure</option>
                  </select>
                </div>
              </div>
              <div class="form-group">
                <label class="form-label">Hostname</label>
                <input v-model="formFields.hostname" class="form-input" placeholder="可选" />
              </div>
              <div class="form-group" style="margin-top:4px">
                <label class="form-label" style="display:flex;align-items:center;gap:6px">
                  <input type="checkbox" v-model="formFields.privileged" />
                  Privileged 模式
                </label>
              </div>
            </div>

            <div v-if="formTab === 'ports'" class="tab-content">
              <div class="list-editor">
                <div v-for="(p, i) in formFields.ports" :key="i" class="list-row">
                  <input v-model="formFields.ports[i]" class="form-input" placeholder="8080:80/tcp" />
                  <button class="btn btn-icon" @click="formFields.ports.splice(i,1)"><X :size="13"/></button>
                </div>
                <button class="btn btn-ghost btn-sm" @click="formFields.ports.push('')"><Plus :size="13"/> 添加端口</button>
              </div>
            </div>

            <div v-if="formTab === 'volumes'" class="tab-content">
              <div class="list-editor">
                <div v-for="(v, i) in formFields.volumes" :key="i" class="list-row">
                  <input v-model="formFields.volumes[i]" class="form-input" placeholder="/host/path:/container/path" />
                  <button class="btn btn-icon" @click="formFields.volumes.splice(i,1)"><X :size="13"/></button>
                </div>
                <button class="btn btn-ghost btn-sm" @click="formFields.volumes.push('')"><Plus :size="13"/> 添加挂载</button>
              </div>
            </div>

            <div v-if="formTab === 'env'" class="tab-content">
              <div class="list-editor">
                <div v-for="(e, i) in formFields.env" :key="i" class="list-row">
                  <input v-model="formFields.env[i]" class="form-input" placeholder="KEY=VALUE" />
                  <button class="btn btn-icon" @click="formFields.env.splice(i,1)"><X :size="13"/></button>
                </div>
                <button class="btn btn-ghost btn-sm" @click="formFields.env.push('')"><Plus :size="13"/> 添加变量</button>
              </div>
            </div>

            <div v-if="formTab === 'cmd'" class="tab-content">
              <div class="form-group">
                <label class="form-label">启动命令</label>
                <input v-model="formFields.command" class="form-input" placeholder="可选，覆盖默认 CMD" />
              </div>
              <div class="form-group">
                <label class="form-label">Entrypoint</label>
                <input v-model="formFields.entrypoint" class="form-input" placeholder="可选" />
              </div>
              <div class="form-group">
                <label class="form-label">User</label>
                <input v-model="formFields.user" class="form-input" placeholder="可选，例如 1000:1000" />
              </div>
              <div class="form-group">
                <label class="form-label">Network Mode</label>
                <select v-model="formFields.network_mode" class="form-select">
                  <option value="">默认 (bridge)</option>
                  <option value="host">host</option>
                  <option value="none">none</option>
                </select>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Generated YAML preview for form mode -->
    <div v-if="mode === 'form' && generatedYaml" class="yaml-preview-bar">
      <span class="form-label" style="flex-shrink:0">生成的 Compose:</span>
      <pre class="yaml-preview-code">{{ generatedYaml }}</pre>
    </div>

    <div class="modal-footer">
      <button class="btn btn-ghost" @click="$emit('close')">取消</button>
      <button class="btn btn-primary" @click="submit" :disabled="submitting">
        <div v-if="submitting" class="spinner" style="width:13px;height:13px;border-width:2px"></div>
        创建并启动
      </button>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { X, Plus, Upload, ClipboardPaste, Terminal, Settings2, RefreshCw, Wand2 } from 'lucide-vue-next'
import api from '@/api'
import { useToastStore } from '@/stores/toast'

const emit = defineEmits(['close', 'start-progress'])
const toast = useToastStore()

const mode = ref('upload')
const submitting = ref(false)
const dragging = ref(false)
const parsing = ref(false)
const runCmd = ref('')
const parsedYaml = ref('')
const formTab = ref('basic')

// Shared compose content for upload/paste/run modes
const composeContent = ref('')

// Form mode only
const formName = ref('')
const formFields = ref({
  image: '', restart: 'unless-stopped', hostname: '', privileged: false,
  ports: [], volumes: [], env: [], command: '', entrypoint: '', user: '', network_mode: ''
})

const modes = [
  { id: 'upload', label: '上传文件', desc: '上传 YAML 文件', icon: Upload },
  { id: 'paste', label: '粘贴内容', desc: '直接粘贴 Compose', icon: ClipboardPaste },
  { id: 'run', label: 'Docker Run', desc: '解析 docker run 命令', icon: Terminal },
  { id: 'form', label: '表单填写', desc: '图形化配置', icon: Settings2 },
]

const formTabs = [
  { id: 'basic', label: '基础配置' },
  { id: 'ports', label: '端口映射' },
  { id: 'volumes', label: '卷挂载' },
  { id: 'env', label: '环境变量' },
  { id: 'cmd', label: '高级配置' },
]

const generatedYaml = computed(() => {
  if (mode.value !== 'form' || !formFields.value.image) return ''
  const f = formFields.value
  const name = formName.value || f.image.split('/').pop().split(':')[0] || 'app'
  let y = `version: '3.8'\n\nservices:\n  ${name}:\n    image: ${f.image}\n    container_name: ${name}\n`
  if (f.restart) y += `    restart: ${f.restart}\n`
  if (f.hostname) y += `    hostname: ${f.hostname}\n`
  if (f.privileged) y += `    privileged: true\n`
  if (f.network_mode) y += `    network_mode: ${f.network_mode}\n`
  if (f.command) y += `    command: ${f.command}\n`
  if (f.entrypoint) y += `    entrypoint: ${f.entrypoint}\n`
  if (f.user) y += `    user: "${f.user}"\n`
  const ports = f.ports.filter(Boolean)
  if (ports.length) { y += '    ports:\n'; ports.forEach(p => y += `      - "${p}"\n`) }
  const vols = f.volumes.filter(Boolean)
  if (vols.length) { y += '    volumes:\n'; vols.forEach(v => y += `      - ${v}\n`) }
  const envs = f.env.filter(Boolean)
  if (envs.length) { y += '    environment:\n'; envs.forEach(e => y += `      - ${e}\n`) }
  return y
})

function onDrop(e) {
  dragging.value = false
  const file = e.dataTransfer.files[0]
  if (file) readFile(file)
}
function onFileSelect(e) {
  const file = e.target.files[0]
  if (file) readFile(file)
}
function readFile(file) {
  const reader = new FileReader()
  reader.onload = e => { composeContent.value = e.target.result }
  reader.readAsText(file)
}

async function parseRun() {
  if (!runCmd.value.trim()) return
  parsing.value = true
  try {
    const res = await api.parseDockerRun(runCmd.value.trim())
    parsedYaml.value = res.data.yaml
    composeContent.value = parsedYaml.value
    toast.success('解析成功')
  } catch (e) { toast.error('解析失败: ' + e) }
  finally { parsing.value = false }
}

async function submit() {
  if (mode.value === 'form') {
    if (!formFields.value.image) { toast.error('请填写镜像'); return }
    const content = generatedYaml.value
    // name is embedded in the yaml via container_name, also pass explicitly
    const name = formName.value || formFields.value.image.split('/').pop().split(':')[0] || 'app'
    // Emit to parent to show progress modal
    emit('start-progress', { name, compose_content: content })
  } else {
    // upload / paste / run — name comes from compose content
    // Auto-parse docker run if needed
    if (mode.value === 'run' && !composeContent.value.trim() && runCmd.value.trim()) {
      parsing.value = true
      try {
        const res = await api.parseDockerRun(runCmd.value.trim())
        parsedYaml.value = res.data.yaml
        composeContent.value = parsedYaml.value
      } catch (e) { toast.error('解析失败: ' + e); parsing.value = false; return }
      finally { parsing.value = false }
    }
    if (!composeContent.value.trim()) { toast.error('请提供 Compose 内容'); return }
    emit('start-progress', { name: '', compose_content: composeContent.value })
  }
}
</script>

<style scoped>
.create-layout { display: flex; min-height: 520px; }
.create-sidebar {
  width: 200px; border-right: 1px solid var(--border); padding: 20px;
  display: flex; flex-direction: column; gap: 20px; flex-shrink: 0;
}
.sidebar-section { display: flex; flex-direction: column; gap: 12px; }
.mode-list { display: flex; flex-direction: column; gap: 3px; }
.mode-item {
  display: flex; align-items: center; gap: 10px; padding: 8px 10px;
  border-radius: var(--radius); cursor: pointer; transition: all var(--transition);
  border: 1px solid transparent;
}
.mode-item:hover { background: var(--accent-dim); }
.mode-item.active { background: rgba(6,182,212,0.1); border-color: var(--border-3); color: var(--accent-light); }
.mode-name { font-size: 12.5px; font-weight: 500; }
.mode-desc { font-size: 11px; color: var(--text-muted); }
.mode-item.active .mode-desc { color: var(--accent); opacity: 0.7; }
.create-main { flex: 1; overflow: hidden; }
.mode-panel { padding: 20px; height: 100%; overflow-y: auto; display: flex; flex-direction: column; gap: 14px; }
.upload-zone {
  border: 2px dashed var(--border-2); border-radius: var(--radius-lg); padding: 40px;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  transition: all var(--transition); text-align: center; min-height: 200px;
}
.upload-zone.dragging { border-color: var(--accent); background: var(--accent-dim); }
.compose-preview { display: flex; flex-direction: column; gap: 8px; }
.compose-preview-header {
  display: flex; justify-content: space-between; align-items: center;
  font-size: 12px; color: var(--text-muted); font-weight: 500;
}
.form-tabs { margin-bottom: 4px; }
.tab-content { padding: 14px 0; display: flex; flex-direction: column; gap: 14px; }
.list-editor { display: flex; flex-direction: column; gap: 6px; }
.list-row { display: flex; align-items: center; gap: 6px; }
.yaml-preview-bar {
  display: flex; align-items: flex-start; gap: 12px; padding: 12px 24px;
  background: var(--bg-base); border-top: 1px solid var(--border); max-height: 160px; overflow: hidden;
}
.yaml-preview-code {
  font-family: var(--font-mono); font-size: 11px; color: var(--text-code);
  overflow-y: auto; max-height: 140px; flex: 1; white-space: pre;
}
.spin { animation: spin 0.8s linear infinite; }
@media (max-width: 768px) {
  .create-layout { flex-direction: column; min-height: unset; }
  .create-sidebar { width: 100%; border-right: none; border-bottom: 1px solid var(--border); padding: 14px; gap: 14px; }
  .mode-list { flex-direction: row; flex-wrap: wrap; gap: 6px; }
  .mode-item { flex: 1; min-width: 80px; flex-direction: column; text-align: center; padding: 8px 6px; gap: 4px; }
  .mode-panel { padding: 14px; }
  .upload-zone { padding: 24px 16px; }
  .yaml-preview-bar { padding: 10px 14px; }
}
</style>
