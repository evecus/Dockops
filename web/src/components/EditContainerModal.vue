<template>
  <div class="modal modal-xl">
    <div class="modal-header">
      <div class="modal-title"><Pencil :size="16" /> 编辑容器</div>
      <button class="modal-close" @click="$emit('close')"><X :size="15" /></button>
    </div>

    <div v-if="loading" class="modal-body" style="display:flex;align-items:center;justify-content:center;min-height:300px">
      <div class="spinner"></div>
    </div>

    <template v-else>
      <div class="modal-body" style="padding:0">
        <div class="edit-layout">
          <div class="edit-sidebar">
            <div class="form-group">
              <label class="form-label">容器名称 *</label>
              <input v-model="form.name" class="form-input" placeholder="my-app" />
            </div>
          </div>

          <div class="edit-main">
            <div class="form-tabs">
              <div class="tabs">
                <div v-for="t in tabs" :key="t.id" class="tab" :class="{ active: activeTab === t.id }" @click="activeTab = t.id">
                  {{ t.label }}
                </div>
              </div>
            </div>

            <div v-if="activeTab === 'basic'" class="tab-content">
              <div class="grid-2">
                <div class="form-group">
                  <label class="form-label">镜像 *</label>
                  <input v-model="fields.image" class="form-input" placeholder="nginx:latest" />
                </div>
                <div class="form-group">
                  <label class="form-label">重启策略</label>
                  <select v-model="fields.restart" class="form-select">
                    <option value="">不重启</option>
                    <option value="always">always</option>
                    <option value="unless-stopped">unless-stopped</option>
                    <option value="on-failure">on-failure</option>
                  </select>
                </div>
              </div>
              <div class="form-group">
                <label class="form-label">Hostname</label>
                <input v-model="fields.hostname" class="form-input" placeholder="可选" />
              </div>
              <div class="form-group" style="margin-top:4px">
                <label class="form-label" style="display:flex;align-items:center;gap:6px">
                  <input type="checkbox" v-model="fields.privileged" />
                  Privileged 模式
                </label>
              </div>
            </div>

            <div v-if="activeTab === 'ports'" class="tab-content">
              <div class="list-editor">
                <div v-for="(p, i) in fields.ports" :key="i" class="list-row">
                  <input v-model="fields.ports[i]" class="form-input" placeholder="8080:80/tcp" />
                  <button class="btn btn-icon" @click="fields.ports.splice(i,1)"><X :size="13"/></button>
                </div>
                <button class="btn btn-ghost btn-sm" @click="fields.ports.push('')"><Plus :size="13"/> 添加端口</button>
              </div>
            </div>

            <div v-if="activeTab === 'volumes'" class="tab-content">
              <div class="list-editor">
                <div v-for="(v, i) in fields.volumes" :key="i" class="list-row">
                  <input v-model="fields.volumes[i]" class="form-input" placeholder="/host/path:/container/path" />
                  <button class="btn btn-icon" @click="fields.volumes.splice(i,1)"><X :size="13"/></button>
                </div>
                <button class="btn btn-ghost btn-sm" @click="fields.volumes.push('')"><Plus :size="13"/> 添加挂载</button>
              </div>
            </div>

            <div v-if="activeTab === 'env'" class="tab-content">
              <div class="list-editor">
                <div v-for="(e, i) in fields.env" :key="i" class="list-row">
                  <input v-model="fields.env[i]" class="form-input" placeholder="KEY=VALUE" />
                  <button class="btn btn-icon" @click="fields.env.splice(i,1)"><X :size="13"/></button>
                </div>
                <button class="btn btn-ghost btn-sm" @click="fields.env.push('')"><Plus :size="13"/> 添加变量</button>
              </div>
            </div>

            <div v-if="activeTab === 'advanced'" class="tab-content">
              <div class="form-group">
                <label class="form-label">启动命令</label>
                <input v-model="fields.command" class="form-input" placeholder="可选，覆盖默认 CMD" />
              </div>
              <div class="form-group">
                <label class="form-label">Entrypoint</label>
                <input v-model="fields.entrypoint" class="form-input" placeholder="可选" />
              </div>
              <div class="form-group">
                <label class="form-label">User</label>
                <input v-model="fields.user" class="form-input" placeholder="可选，例如 1000:1000" />
              </div>
              <div class="form-group">
                <label class="form-label">Network Mode</label>
                <select v-model="fields.network_mode" class="form-select">
                  <option value="">默认 (bridge)</option>
                  <option value="host">host</option>
                  <option value="none">none</option>
                </select>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div v-if="generatedYaml" class="yaml-preview-bar">
        <span class="form-label" style="flex-shrink:0">Compose 预览:</span>
        <pre class="yaml-preview-code">{{ generatedYaml }}</pre>
      </div>

      <div class="modal-footer">
        <button class="btn btn-ghost" @click="$emit('close')">取消</button>
        <button class="btn btn-primary" @click="submit" :disabled="submitting">
          <div v-if="submitting" class="spinner" style="width:13px;height:13px;border-width:2px"></div>
          保存并重启
        </button>
      </div>
    </template>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { X, Plus, Pencil } from 'lucide-vue-next'
import api from '@/api'
import { useToastStore } from '@/stores/toast'

const props = defineProps({ container: Object })
const emit = defineEmits(['close', 'saved'])
const toast = useToastStore()

const loading = ref(true)
const submitting = ref(false)
const activeTab = ref('basic')

// oldName is the current docker container name, used as the URL param
const oldName = ref('')
const form = ref({ name: '' })
const fields = ref({
  image: '', restart: 'unless-stopped', hostname: '', privileged: false,
  ports: [], volumes: [], env: [], command: '', entrypoint: '', user: '', network_mode: ''
})

const tabs = [
  { id: 'basic', label: '基础配置' },
  { id: 'ports', label: '端口映射' },
  { id: 'volumes', label: '卷挂载' },
  { id: 'env', label: '环境变量' },
  { id: 'advanced', label: '高级配置' },
]

const generatedYaml = computed(() => {
  if (!fields.value.image) return ''
  const f = fields.value
  const name = form.value.name || 'app'
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

async function submit() {
  if (!form.value.name) { toast.error('请填写容器名称'); return }
  if (!fields.value.image) { toast.error('请填写镜像'); return }

  submitting.value = true
  try {
    // Pass oldName as URL param so backend knows which docker container to stop.
    // Pass newName in body so backend knows what to create.
    await api.updateContainer(oldName.value, {
      name: form.value.name,
      compose_content: generatedYaml.value,
    })
    toast.success('容器已更新')
    emit('saved')
  } catch (e) {
    toast.error(typeof e === 'string' ? e : '操作失败')
  } finally {
    submitting.value = false
  }
}

onMounted(async () => {
  // props.container.name is the actual docker container name
  oldName.value = props.container.name
  try {
    const res = await api.getContainerFormData(props.container.name)
    const data = res.data
    // Backend returns name = docker container name from inspect
    form.value.name = data.name
    const f = data.fields || {}
    fields.value = {
      image: f.image || '',
      restart: f.restart || 'unless-stopped',
      hostname: f.hostname || '',
      privileged: f.privileged || false,
      ports: f.ports?.length ? f.ports : [],
      volumes: f.volumes?.length ? f.volumes : [],
      env: f.env?.length ? f.env : [],
      command: f.command || '',
      entrypoint: f.entrypoint || '',
      user: f.user || '',
      network_mode: f.network_mode || '',
    }
  } catch (e) {
    toast.error('加载容器数据失败')
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.edit-layout { display: flex; min-height: 480px; }
.edit-sidebar { width: 200px; border-right: 1px solid var(--border); padding: 20px; flex-shrink: 0; }
.edit-main { flex: 1; overflow: hidden; }
.form-tabs { padding: 16px 20px 0; }
.tab-content { padding: 16px 20px; display: flex; flex-direction: column; gap: 14px; }
.list-editor { display: flex; flex-direction: column; gap: 6px; }
.list-row { display: flex; align-items: center; gap: 6px; }
.yaml-preview-bar {
  display: flex; align-items: flex-start; gap: 12px; padding: 12px 24px;
  background: var(--bg-base); border-top: 1px solid var(--border);
  max-height: 160px; overflow: hidden;
}
.yaml-preview-code {
  font-family: var(--font-mono); font-size: 11px; color: var(--text-code);
  overflow-y: auto; max-height: 140px; flex: 1; white-space: pre;
}
@media (max-width: 768px) {
  .edit-layout { flex-direction: column; min-height: unset; }
  .edit-sidebar { width: 100%; border-right: none; border-bottom: 1px solid var(--border); padding: 12px 14px; }
  .edit-main { overflow: visible; }
  .form-tabs { padding: 12px 14px 0; }
  .tab-content { padding: 12px 14px; }
  .yaml-preview-bar { padding: 10px 14px; }
}
</style>
