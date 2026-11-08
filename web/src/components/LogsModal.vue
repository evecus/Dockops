<template>
  <div class="modal modal-xl" style="height:80vh;max-height:700px">
    <div class="modal-header">
      <div class="modal-title"><ScrollText :size="16" /> 日志 — {{ container.name }}</div>
      <div style="display:flex;align-items:center;gap:8px">
        <button class="btn btn-ghost btn-sm" @click="toggleStream">
          <component :is="streaming ? Pause : Play" :size="13" />
          {{ streaming ? '暂停' : '继续' }}
        </button>
        <button class="btn btn-ghost btn-sm" @click="clearLogs">
          <Eraser :size="13" /> 清空
        </button>
        <button class="btn btn-ghost btn-sm" @click="downloadLogs">
          <Download :size="13" /> 下载
        </button>
        <button class="modal-close" @click="$emit('close')"><X :size="15" /></button>
      </div>
    </div>

    <div class="logs-toolbar">
      <div class="search-box">
        <Search :size="13" style="color:var(--text-muted)" />
        <input v-model="search" class="search-input" placeholder="搜索日志..." />
      </div>
      <label class="checkbox-label">
        <input type="checkbox" v-model="autoScroll" />
        <span>自动滚动</span>
      </label>
      <span class="log-count">{{ filteredLines.length }} 行</span>
    </div>

    <div class="logs-body" ref="logsEl" @scroll="onScroll">
      <div class="log-content">
        <div v-for="(line, i) in filteredLines" :key="i"
          class="log-line" :class="lineClass(line)">
          <span class="log-num">{{ i + 1 }}</span>
          <span class="log-text" v-html="highlight(line)"></span>
        </div>
        <div v-if="!filteredLines.length" class="empty-state" style="padding:30px 0">
          <ScrollText :size="36" /><p>暂无日志</p>
        </div>
      </div>
    </div>

    <div class="logs-status">
      <div class="status-dot" :class="streaming ? 'live' : 'paused'"></div>
      <span>{{ streaming ? '实时流' : '已暂停' }}</span>
      <span class="sep" style="margin-left:auto">{{ container.name }}</span>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { ScrollText, X, Play, Pause, Download, Eraser, Search } from 'lucide-vue-next'
import api from '@/api'

const props = defineProps({ container: Object })
defineEmits(['close'])

const lines = ref([])
const search = ref('')
const streaming = ref(true)
const autoScroll = ref(true)
const logsEl = ref(null)
let ws = null

const filteredLines = computed(() => {
  if (!search.value) return lines.value
  const q = search.value.toLowerCase()
  return lines.value.filter(l => l.toLowerCase().includes(q))
})

function lineClass(line) {
  if (/error|err|fatal|fail/i.test(line)) return 'log-error'
  if (/warn|warning/i.test(line)) return 'log-warn'
  if (/info|debug/i.test(line)) return 'log-info'
  return ''
}

function highlight(line) {
  if (!search.value) return escapeHtml(line)
  const q = search.value
  const escaped = escapeHtml(line)
  const re = new RegExp(escapeRe(escapeHtml(q)), 'gi')
  return escaped.replace(re, m => `<mark>${m}</mark>`)
}

function escapeHtml(s) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')
}
function escapeRe(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function scrollBottom() {
  nextTick(() => {
    if (logsEl.value) logsEl.value.scrollTop = logsEl.value.scrollHeight
  })
}

function onScroll() {
  if (!logsEl.value) return
  const el = logsEl.value
  const atBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - 40
  if (!atBottom) autoScroll.value = false
}

watch(() => lines.value.length, () => {
  if (autoScroll.value) scrollBottom()
})

function toggleStream() {
  streaming.value = !streaming.value
  if (streaming.value) connectWS()
  else ws?.close()
}

function clearLogs() { lines.value = [] }

function downloadLogs() {
  const blob = new Blob([lines.value.join('\n')], { type: 'text/plain' })
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = `${props.container.name}-logs.txt`
  a.click()
}

function connectWS() {
  ws?.close()
  const url = api.logsWsUrl(props.container.name)
  ws = new WebSocket(url)
  ws.onmessage = (e) => {
    const text = typeof e.data === 'string' ? e.data : ''
    text.split('\n').forEach(l => {
      const trimmed = l.trim()
      if (trimmed) lines.value.push(trimmed)
    })
    // Limit buffer
    if (lines.value.length > 5000) lines.value.splice(0, lines.value.length - 5000)
  }
  ws.onclose = () => { if (streaming.value) setTimeout(connectWS, 2000) }
}

onMounted(() => {
  // Load initial logs via REST
  api.containerLogs(props.container.name).then(res => {
    const text = res.data?.logs || ''
    lines.value = text.split('\n').filter(Boolean)
    scrollBottom()
  }).catch(() => {})

  connectWS()
})

onUnmounted(() => {
  ws?.close()
})
</script>

<style scoped>
.logs-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-base);
}
.search-box {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 5px 10px;
  flex: 1;
  max-width: 280px;
}
.search-input {
  background: none;
  border: none;
  color: var(--text-primary);
  font-size: 12.5px;
  width: 100%;
}
.checkbox-label {
  display: flex; align-items: center; gap: 5px;
  font-size: 12px; color: var(--text-muted); cursor: pointer;
  user-select: none;
}
.log-count { font-size: 11px; color: var(--text-muted); font-family: var(--font-mono); }
.logs-body {
  flex: 1;
  overflow-y: auto;
  background: #060b12;
}
.log-content { padding: 8px 0; }
.log-line {
  display: flex;
  gap: 12px;
  padding: 1.5px 12px;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  transition: background 0.1s;
}
.log-line:hover { background: rgba(6,182,212,0.04); }
.log-num {
  color: var(--text-muted);
  min-width: 40px;
  text-align: right;
  user-select: none;
  flex-shrink: 0;
}
.log-text { color: #b0c8d8; white-space: pre-wrap; word-break: break-all; flex: 1; }
.log-error .log-text { color: #f87171; }
.log-warn .log-text { color: #fcd34d; }
.log-info .log-text { color: #6ee7b7; }
:deep(mark) {
  background: rgba(245,158,11,0.3);
  color: #fcd34d;
  border-radius: 2px;
}
.logs-status {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 16px;
  border-top: 1px solid var(--border);
  font-size: 11.5px;
  color: var(--text-muted);
}
.status-dot { width: 6px; height: 6px; border-radius: 50%; }
.status-dot.live { background: var(--green); animation: pulse-dot 1.5s infinite; }
.status-dot.paused { background: var(--text-muted); }

@media (max-width: 768px) {
  .logs-toolbar { flex-wrap: wrap; gap: 8px; padding: 8px 10px; }
  .search-box { max-width: 100%; flex: 1 1 100%; }
  .log-line { font-size: 11px; padding: 1.5px 8px; gap: 6px; }
  .log-num { min-width: 28px; }
}
</style>
