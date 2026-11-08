import axios from 'axios'

const api = axios.create({ baseURL: '/api' })

api.interceptors.request.use(cfg => {
  const token = localStorage.getItem('token')
  if (token) cfg.headers.Authorization = `Bearer ${token}`
  return cfg
})

api.interceptors.response.use(
  res => res.data,
  err => {
    if (err.response?.status === 401) {
      localStorage.removeItem('token')
      location.href = '/login'
    }
    return Promise.reject(err.response?.data?.error || err.message || 'Request failed')
  }
)

export default {
  // Auth
  systemStatus: () => api.get('/system/status'),
  setup: (data) => api.post('/auth/setup', data),
  login: (data) => api.post('/auth/login', data),

  // Dashboard
  dashboardInfo: () => api.get('/dashboard/info'),
  dashboardStats: () => api.get('/dashboard/stats'),
  dashboardRefresh: () => api.post('/dashboard/refresh'),

  // Containers — all keyed by docker container name
  listContainers: () => api.get('/containers'),
  createContainer: (data) => api.post('/containers', data),
  // Returns a fetch Response for SSE streaming
  createContainerStream: (data) => fetch('/api/containers/stream', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${localStorage.getItem('token')}`,
    },
    body: JSON.stringify(data),
  }),
  parseDockerRun: (command) => api.post('/containers/parse-run', { command }),
  getContainer: (name) => api.get(`/containers/${name}`),
  getContainerFormData: (name) => api.get(`/containers/${name}/form-data`),
  updateContainer: (name, data) => api.put(`/containers/${name}`, data),
  deleteContainer: (name, removeCompose=false) => api.delete(`/containers/${name}`, { params: { remove_compose: removeCompose } }),
  startContainer: (name) => api.post(`/containers/${name}/start`),
  stopContainer: (name) => api.post(`/containers/${name}/stop`),
  restartContainer: (name) => api.post(`/containers/${name}/restart`),
  containerStats: (name) => api.get(`/containers/${name}/stats`),
  containerLogs: (name, tail=500) => api.get(`/containers/${name}/logs`, { params: { tail } }),
  listContainerFiles: (name, path='/') => api.get(`/containers/${name}/files`, { params: { path } }),
  downloadContainerFile: (name, path) => `/api/containers/${name}/files/download?path=${encodeURIComponent(path)}&token=${localStorage.getItem('token')}`,
  uploadContainerFile: (name, path, file) => {
    const fd = new FormData(); fd.append('file', file)
    return api.post(`/containers/${name}/files/upload?path=${encodeURIComponent(path)}`, fd)
  },
  deleteContainerFile: (name, path) => api.delete(`/containers/${name}/files`, { params: { path } }),
  updateContainerImage: (name) => api.post(`/containers/${name}/update`),

  // Images
  listImages: () => api.get('/images'),
  getImageUpdateStatus: () => api.get('/images/update-status'),
  checkImageUpdate: (tag) => fetch(`/api/images/check-update?tag=${encodeURIComponent(tag)}`, {
    headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` },
  }),
  pullImage: (image) => api.post('/images/pull', { image }, { responseType: 'stream' }),
  loadImage: (file) => { const fd = new FormData(); fd.append('file', file); return api.post('/images/load', fd) },
  deleteImage: (id, force=false) => api.delete(`/images/${id}`, { params: { force } }),

  // Networks
  listNetworks: () => api.get('/networks'),
  createNetwork: (data) => api.post('/networks', data),
  deleteNetwork: (id) => api.delete(`/networks/${id}`),
  pruneNetworks: () => api.post('/networks/prune'),

  // Volumes
  listVolumes: () => api.get('/volumes'),
  createVolume: (data) => api.post('/volumes', data),
  deleteVolume: (name, force=false) => api.delete(`/volumes/${name}`, { params: { force } }),
  pruneVolumes: () => api.post('/volumes/prune'),

  // Settings
  getSettings: () => api.get('/settings'),
  updateSettings: (data) => api.put('/settings', data),
  updateAdmin: (data) => api.put('/settings/admin', data),
  installDocker: () => api.post('/settings/install-docker'),

  // WS URLs
  terminalWsUrl: (name) => `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/ws/containers/${name}/terminal?token=${localStorage.getItem('token')}`,
  logsWsUrl: (name) => `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/ws/containers/${name}/logs?token=${localStorage.getItem('token')}`,
}
