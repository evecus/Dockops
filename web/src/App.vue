<template>
  <RouterView />
  <!-- Global Toast -->
  <div class="toast-container">
    <TransitionGroup name="toast-anim">
      <div v-for="t in toasts.toasts" :key="t.id"
        class="toast" :class="`toast-${t.type}`">
        <component :is="toastIcon(t.type)" class="toast-icon" :size="15" />
        <span>{{ t.message }}</span>
        <button class="toast-close" @click="toasts.remove(t.id)">
          <X :size="13" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup>
import { RouterView } from 'vue-router'
import { CheckCircle, XCircle, Info, X } from 'lucide-vue-next'
import { useToastStore } from '@/stores/toast'

const toasts = useToastStore()
const toastIcon = (type) => ({ success: CheckCircle, error: XCircle, info: Info }[type] || Info)
</script>

<style>
.toast-icon { flex-shrink: 0; }
.toast-success .toast-icon { color: var(--green); }
.toast-error .toast-icon { color: var(--red); }
.toast-info .toast-icon { color: var(--accent); }
.toast-close {
  margin-left: auto;
  flex-shrink: 0;
  background: transparent;
  color: var(--text-muted);
  display: flex; align-items: center;
  padding: 2px;
  border-radius: 4px;
}
.toast-close:hover { color: var(--text-primary); }
.toast-anim-enter-active { animation: toast-in 0.3s cubic-bezier(0.34,1.56,0.64,1); }
.toast-anim-leave-active { animation: toast-in 0.2s ease reverse; }
</style>
