<template>
  <div class="modal modal-xl" style="height:80vh;max-height:700px">
    <div class="modal-header">
      <div class="modal-title">
        <Terminal :size="16" />
        终端 — {{ container.name }}
      </div>
      <button class="modal-close" @click="$emit('close')"><X :size="15" /></button>
    </div>
    <div class="terminal-wrap">
      <div ref="termEl" class="term-container"></div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { Terminal, X } from 'lucide-vue-next'
import api from '@/api'

const props = defineProps({ container: Object })
defineEmits(['close'])

const termEl = ref(null)
let term = null
let ws = null
let fitAddon = null

onMounted(async () => {
  const { Terminal: XTerm } = await import('xterm')
  const { FitAddon } = await import('@xterm/addon-fit')
  const { WebLinksAddon } = await import('@xterm/addon-web-links')

  term = new XTerm({
    theme: {
      background: '#090e17',
      foreground: '#e8f4f8',
      cursor: '#06b6d4',
      cursorAccent: '#090e17',
      selectionBackground: 'rgba(6,182,212,0.25)',
      black: '#1a2b42',
      blue: '#22d3ee',
      cyan: '#06b6d4',
      green: '#10d97a',
      red: '#f05464',
      yellow: '#f59e0b',
      white: '#e8f4f8',
    },
    fontFamily: "'JetBrains Mono', monospace",
    fontSize: 13,
    lineHeight: 1.5,
    cursorBlink: true,
    scrollback: 5000,
    allowTransparency: true,
  })

  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.loadAddon(new WebLinksAddon())
  term.open(termEl.value)
  fitAddon.fit()

  const url = api.terminalWsUrl(props.container.name)
  ws = new WebSocket(url)

  ws.onopen = () => {
    term.write('\x1b[32m✓ 已连接到容器\x1b[0m\r\n')
    // Send initial resize
    const { rows, cols } = term
    ws.send(JSON.stringify({ type: 'resize', rows, cols }))
  }

  ws.onmessage = (e) => {
    if (e.data instanceof Blob) {
      e.data.arrayBuffer().then(buf => term.write(new Uint8Array(buf)))
    } else {
      term.write(e.data)
    }
  }

  ws.onerror = () => term.write('\r\n\x1b[31m连接错误\x1b[0m\r\n')
  ws.onclose = () => term.write('\r\n\x1b[33m连接已断开\x1b[0m\r\n')

  term.onData(data => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'input', data }))
    }
  })

  term.onResize(({ rows, cols }) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'resize', rows, cols }))
    }
  })

  const ro = new ResizeObserver(() => fitAddon?.fit())
  ro.observe(termEl.value)
})

onUnmounted(() => {
  ws?.close()
  term?.dispose()
})
</script>

<style scoped>
.terminal-wrap {
  flex: 1;
  background: #090e17;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.term-container {
  flex: 1;
  padding: 8px;
  overflow: hidden;
}
:deep(.xterm) { height: 100%; }
:deep(.xterm-screen) { height: 100% !important; }

@media (max-width: 768px) {
  .terminal-wrap { min-height: 300px; }
}
</style>
