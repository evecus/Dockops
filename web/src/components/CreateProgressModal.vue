<template>
  <div class="modal" style="max-width:620px">
    <div class="modal-header">
      <div class="modal-title">
        <component :is="done ? CheckCircle2 : Loader2" :size="16"
          :class="done ? 'text-green' : 'spin'" />
        {{ done ? '容器创建成功' : '正在创建容器...' }}
      </div>
    </div>

    <div class="modal-body">
      <div class="log-box" ref="logBoxRef">
        <div v-for="(line, i) in logs" :key="i" class="log-line" :class="line.type">
          <span class="log-prefix">{{ line.type === 'error' ? '✗' : '›' }}</span>
          <span class="log-text">{{ line.text }}</span>
        </div>
        <div v-if="!done && !error" class="log-line log-cursor">
          <span class="cursor-dot"></span>
        </div>
      </div>
    </div>

    <div class="modal-footer">
      <button v-if="done || error" class="btn btn-primary" @click="$emit('close')">
        {{ done ? '完成' : '关闭' }}
      </button>
      <span v-else class="text-muted" style="font-size:13px">请稍候...</span>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, nextTick } from 'vue'
import { Loader2, CheckCircle2 } from 'lucide-vue-next'
import api from '@/api'

const props = defineProps({
  name: String,
  composeContent: String,
})
const emit = defineEmits(['close', 'created'])

const logs = ref([])
const done = ref(false)
const error = ref(false)
const logBoxRef = ref(null)

function addLog(type, text) {
  logs.value.push({ type, text })
  nextTick(() => {
    if (logBoxRef.value) logBoxRef.value.scrollTop = logBoxRef.value.scrollHeight
  })
}

onMounted(async () => {
  addLog('info', '准备创建容器...')
  try {
    const response = await api.createContainerStream({
      name: props.name || '',
      compose_content: props.composeContent,
    })

    const reader = response.body.getReader()
    const decoder = new TextDecoder()
    let buffer = ''

    while (true) {
      const { done: streamDone, value } = await reader.read()
      if (streamDone) break

      buffer += decoder.decode(value, { stream: true })
      const parts = buffer.split('\n\n')
      buffer = parts.pop() // keep incomplete chunk

      for (const part of parts) {
        const lines = part.split('\n')
        let eventType = 'log'
        let data = ''
        for (const line of lines) {
          if (line.startsWith('event:')) eventType = line.slice(6).trim()
          if (line.startsWith('data:')) data = line.slice(5).trim()
        }
        if (!data) continue

        if (eventType === 'done') {
          addLog('success', `容器 "${data}" 创建并启动成功！`)
          done.value = true
          emit('created')
        } else if (eventType === 'error') {
          addLog('error', data)
          error.value = true
        } else {
          addLog(eventType === 'info' ? 'info' : 'log', data)
        }
      }
    }
  } catch (e) {
    addLog('error', '请求失败: ' + e.message)
    error.value = true
  }
})
</script>

<style scoped>
.log-box {
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 14px;
  min-height: 280px;
  max-height: 420px;
  overflow-y: auto;
  font-family: var(--font-mono);
  font-size: 12px;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.log-line {
  display: flex;
  gap: 8px;
  line-height: 1.6;
  word-break: break-all;
}
.log-prefix { color: var(--text-muted); flex-shrink: 0; }
.log-text { color: var(--text-secondary); }
.log-line.info .log-prefix { color: var(--accent); }
.log-line.info .log-text { color: var(--accent-light); }
.log-line.success .log-prefix { color: var(--green); }
.log-line.success .log-text { color: var(--green); font-weight: 600; }
.log-line.error .log-prefix { color: var(--red); }
.log-line.error .log-text { color: var(--red); }
.log-cursor {
  display: flex;
  align-items: center;
  padding: 2px 0;
}
.cursor-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
  animation: blink 1s infinite;
}
.text-green { color: var(--green); }
.spin { animation: spin 1s linear infinite; }
@keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0; } }
</style>
