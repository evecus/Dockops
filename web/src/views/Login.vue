<template>
  <div class="auth-page">
    <div class="auth-card">
      <!-- Lang toggle top-right -->
      <button class="auth-lang-btn" @click="i18n.toggle()">{{ t.langLabel }}</button>

      <div class="auth-logo">
        <div class="logo-icon">
          <img src="/apple-touch-icon.png" width="28" height="28" style="border-radius:6px;display:block;" alt="DockOps" />
        </div>
        <h1>Dock<span>Ops</span></h1>
      </div>

      <p class="auth-subtitle">{{ isSetup ? t.createAdmin : t.loginPanel }}</p>

      <div v-if="!checked" class="auth-loading">
        <div class="spinner"></div>
      </div>

      <form v-else @submit.prevent="submit" class="auth-form">
        <div class="form-group">
          <label class="form-label">{{ t.username }}</label>
          <input v-model="form.username" class="form-input" :placeholder="t.inputUsername"
            autocomplete="username" required />
        </div>
        <div class="form-group">
          <label class="form-label">{{ t.password }}</label>
          <div style="position:relative">
            <input v-model="form.password" class="form-input"
              :type="showPwd ? 'text' : 'password'"
              :placeholder="t.inputPassword" autocomplete="current-password" required />
            <button type="button" class="pwd-toggle" @click="showPwd = !showPwd">
              <Eye v-if="!showPwd" :size="15" />
              <EyeOff v-else :size="15" />
            </button>
          </div>
        </div>

        <div v-if="error" class="auth-error">
          <AlertCircle :size="14" /> {{ error }}
        </div>

        <button type="submit" class="btn btn-primary auth-submit" :disabled="loading">
          <div v-if="loading" class="spinner" style="width:14px;height:14px;border-width:2px"></div>
          <span>{{ isSetup ? t.createAccount : t.login }}</span>
        </button>
      </form>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { Eye, EyeOff, AlertCircle } from 'lucide-vue-next'
import api from '@/api'
import { useI18nStore } from '@/stores/i18n'

const router = useRouter()
const i18n = useI18nStore()
const t = computed(() => i18n.t)

const isSetup = ref(false)
const checked = ref(false)
const loading = ref(false)
const error = ref('')
const showPwd = ref(false)
const form = ref({ username: '', password: '' })

onMounted(async () => {
  try {
    const res = await api.systemStatus()
    const setup = res.data?.setup
    if (!setup) {
      isSetup.value = true
    } else {
      isSetup.value = false
      if (localStorage.getItem('token')) {
        router.replace('/dashboard')
        return
      }
    }
  } catch {}
  checked.value = true
})

async function submit() {
  error.value = ''
  loading.value = true
  try {
    if (isSetup.value) {
      await api.setup(form.value)
      const res = await api.login(form.value)
      localStorage.setItem('token', res.data.token)
    } else {
      const res = await api.login(form.value)
      localStorage.setItem('token', res.data.token)
    }
    router.replace('/dashboard')
  } catch (e) {
    error.value = typeof e === 'string' ? e : t.value.operationFailed
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.auth-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-base);
}
.auth-card {
  position: relative;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-xl);
  padding: 40px;
  width: 400px;
  box-shadow: var(--shadow-lg);
}
.auth-lang-btn {
  position: absolute;
  top: 20px; right: 20px;
  padding: 5px 11px;
  border-radius: var(--radius);
  font-size: 13px; font-weight: 600;
  background: var(--bg-hover);
  color: var(--text-secondary);
  border: 1px solid var(--border);
  cursor: pointer;
  transition: all var(--transition);
}
.auth-lang-btn:hover {
  background: var(--accent-dim);
  color: var(--accent);
  border-color: var(--accent-light);
}
.auth-logo {
  display: flex; align-items: center; gap: 12px; margin-bottom: 8px;
}
.auth-logo h1 { font-size: 24px; font-weight: 700; letter-spacing: -0.5px; }
.auth-logo h1 span { color: var(--accent); }
.auth-subtitle { font-size: 14px; color: var(--text-muted); margin-bottom: 28px; }
.auth-loading { display: flex; justify-content: center; padding: 20px; }
.auth-form { display: flex; flex-direction: column; gap: 16px; }
.pwd-toggle {
  position: absolute; right: 10px; top: 50%; transform: translateY(-50%);
  background: none; color: var(--text-muted); display: flex;
}
.pwd-toggle:hover { color: var(--text-secondary); }
.auth-error {
  display: flex; align-items: center; gap: 6px;
  padding: 9px 13px;
  background: rgba(239,68,68,0.05);
  border: 1px solid rgba(239,68,68,0.2);
  border-radius: var(--radius);
  font-size: 13.5px; color: var(--red);
}
.auth-submit {
  width: 100%; justify-content: center; padding: 11px; font-size: 14px; gap: 8px;
}
.auth-submit:disabled { opacity: 0.6; cursor: not-allowed; transform: none !important; }
</style>
