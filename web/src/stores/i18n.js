import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

const messages = {
  zh: {
    // Nav
    dashboard: '仪表盘',
    containers: '容器管理',
    images: '镜像管理',
    networkStorage: '网络 & 存储',
    settings: '系统设置',
    logout: '退出登录',

    // Topbar
    dockerConnected: 'Docker 已连接',
    dockerDisconnected: 'Docker 未连接',
    refreshDashboard: '刷新仪表盘数据',

    // Page descriptions
    dashboardDesc: '主机状态与资源概览',
    containersDesc: '管理 Docker Compose 容器',
    imagesDesc: '查看与管理本地镜像',
    networkStorageDesc: '网络与卷管理',
    settingsDesc: '配置系统参数',

    // Auth
    login: '登录',
    register: '注册',
    loginPanel: '登录管理面板',
    createAdmin: '创建管理员账号',
    username: '用户名',
    password: '密码',
    inputUsername: '输入用户名',
    inputPassword: '输入密码',
    createAccount: '创建账号',
    operationFailed: '操作失败，请重试',

    // Dashboard
    runningContainers: '运行中容器',
    allContainers: '全部容器',
    localImages: '本地镜像',
    cpuCores: 'CPU 核心',
    hostInfo: '主机信息',
    resourceUsage: '资源占用',
    dockerVersion: 'Docker 版本',
    os: '操作系统',
    arch: '系统架构',
    kernelVersion: '内核版本',
    storageDriver: '存储驱动',
    dockerRootDir: 'Docker 根目录',
    totalMemory: '总内存',
    serverTime: '系统时间',
    cpuUsage: 'CPU 占用',
    memUsage: '内存占用',
    running: '运行中',
    stopped: '已停止',
    paused: '已暂停',
    updatedAgo: (s) => s < 60 ? `${s}s 前更新` : s < 3600 ? `${Math.floor(s/60)}m 前更新` : `${Math.floor(s/3600)}h 前更新`,

    // Lang toggle
    langLabel: 'EN',
  },
  en: {
    // Nav
    dashboard: 'Dashboard',
    containers: 'Containers',
    images: 'Images',
    networkStorage: 'Network & Storage',
    settings: 'Settings',
    logout: 'Logout',

    // Topbar
    dockerConnected: 'Docker Connected',
    dockerDisconnected: 'Docker Disconnected',
    refreshDashboard: 'Refresh dashboard data',

    // Page descriptions
    dashboardDesc: 'Host status & resource overview',
    containersDesc: 'Manage Docker Compose containers',
    imagesDesc: 'View and manage local images',
    networkStorageDesc: 'Network and volume management',
    settingsDesc: 'Configure system parameters',

    // Auth
    login: 'Login',
    register: 'Register',
    loginPanel: 'Sign in to Dashboard',
    createAdmin: 'Create Admin Account',
    username: 'Username',
    password: 'Password',
    inputUsername: 'Enter username',
    inputPassword: 'Enter password',
    createAccount: 'Create Account',
    operationFailed: 'Operation failed, please try again',

    // Dashboard
    runningContainers: 'Running Containers',
    allContainers: 'Total Containers',
    localImages: 'Local Images',
    cpuCores: 'CPU Cores',
    hostInfo: 'Host Info',
    resourceUsage: 'Resource Usage',
    dockerVersion: 'Docker Version',
    os: 'Operating System',
    arch: 'Architecture',
    kernelVersion: 'Kernel Version',
    storageDriver: 'Storage Driver',
    dockerRootDir: 'Docker Root Dir',
    totalMemory: 'Total Memory',
    serverTime: 'Server Time',
    cpuUsage: 'CPU Usage',
    memUsage: 'Memory Usage',
    running: 'Running',
    stopped: 'Stopped',
    paused: 'Paused',
    updatedAgo: (s) => s < 60 ? `${s}s ago` : s < 3600 ? `${Math.floor(s/60)}m ago` : `${Math.floor(s/3600)}h ago`,

    // Lang toggle
    langLabel: '中',
  }
}

export const useI18nStore = defineStore('i18n', () => {
  const lang = ref(localStorage.getItem('dockops_lang') || 'zh')

  function toggle() {
    lang.value = lang.value === 'zh' ? 'en' : 'zh'
    localStorage.setItem('dockops_lang', lang.value)
  }

  function setLang(l) {
    lang.value = l
    localStorage.setItem('dockops_lang', l)
  }

  const t = computed(() => messages[lang.value])

  return { lang, toggle, setLang, t }
})
