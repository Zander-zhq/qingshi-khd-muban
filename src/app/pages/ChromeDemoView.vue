<script setup lang="ts">
import { ref, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Button from 'primevue/button'
import Select from 'primevue/select'
import InputText from 'primevue/inputtext'
import Tag from 'primevue/tag'
import Message from 'primevue/message'

const PRESETS = [
  { label: '抖音', value: 'https://www.douyin.com' },
  { label: '快手', value: 'https://www.kuaishou.com' },
  { label: 'B站', value: 'https://www.bilibili.com' },
  { label: '百度', value: 'https://www.baidu.com' },
]

const selectedPreset = ref(PRESETS[0])
const customUrl = ref('')
const debugPort = ref(9222)
const isRunning = ref(false)
const statusMsg = ref('')
const loading = ref(false)

let pollTimer: ReturnType<typeof setInterval> | null = null

function getUrl() {
  return customUrl.value.trim() || selectedPreset.value.value
}

async function launchChrome() {
  loading.value = true
  statusMsg.value = ''
  try {
    const port = await invoke<number>('launch_chrome_app', {
      url: getUrl(),
      width: 1200,
      height: 800,
      port: debugPort.value,
    })
    isRunning.value = true
    statusMsg.value = `Chrome 已启动，CDP 端口: ${port}`
    startPolling()
  } catch (e: any) {
    statusMsg.value = `启动失败: ${e}`
  } finally {
    loading.value = false
  }
}

async function killChrome() {
  try {
    await invoke('kill_chrome_app')
    isRunning.value = false
    statusMsg.value = 'Chrome 已关闭'
    stopPolling()
  } catch (e: any) {
    statusMsg.value = `关闭失败: ${e}`
  }
}

async function checkStatus() {
  try {
    const running = await invoke<boolean>('is_chrome_running')
    if (isRunning.value && !running) {
      isRunning.value = false
      statusMsg.value = 'Chrome 已被用户关闭'
      stopPolling()
    }
  } catch {}
}

function startPolling() {
  stopPolling()
  pollTimer = setInterval(checkStatus, 2000)
}
function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

onUnmounted(() => {
  stopPolling()
})
</script>

<template>
  <div class="p-4 flex flex-col gap-5">
    <Message severity="info" :closable="false">
      <div class="text-sm leading-relaxed">
        <b>Chrome --app 模式演示</b>：启动一个独立的 Chrome 进程，没有地址栏和标签栏，
        看起来像嵌入的 WebView。同时开放 CDP 端口，DrissionPage 可以连接控制它。
      </div>
    </Message>

    <div class="flex flex-col gap-3 p-4 border border-surface-200 dark:border-surface-700 rounded-lg">
      <div class="font-semibold text-base">启动配置</div>

      <div class="flex items-center gap-3">
        <label class="w-20 text-sm shrink-0">预设网站</label>
        <Select
          v-model="selectedPreset"
          :options="PRESETS"
          optionLabel="label"
          class="w-48"
          :disabled="isRunning"
        />
      </div>

      <div class="flex items-center gap-3">
        <label class="w-20 text-sm shrink-0">自定义 URL</label>
        <InputText
          v-model="customUrl"
          placeholder="留空则使用上方预设"
          class="flex-1"
          :disabled="isRunning"
        />
      </div>

      <div class="flex items-center gap-3">
        <label class="w-20 text-sm shrink-0">CDP 端口</label>
        <InputText
          v-model.number="debugPort"
          type="number"
          class="w-32"
          :disabled="isRunning"
        />
        <span class="text-xs text-surface-500">DrissionPage 连接用此端口</span>
      </div>
    </div>

    <div class="flex items-center gap-3">
      <Button
        :label="isRunning ? '正在运行...' : '启动 Chrome'"
        icon="pi pi-play"
        :disabled="isRunning"
        :loading="loading"
        @click="launchChrome"
      />
      <Button
        label="关闭 Chrome"
        icon="pi pi-stop"
        severity="danger"
        :disabled="!isRunning"
        @click="killChrome"
      />
      <Tag v-if="isRunning" severity="success" value="运行中" />
      <Tag v-else severity="secondary" value="未启动" />
    </div>

    <div v-if="statusMsg" class="text-sm text-surface-600 dark:text-surface-400">
      {{ statusMsg }}
    </div>

    <div v-if="isRunning" class="flex flex-col gap-2 p-4 bg-surface-50 dark:bg-surface-800 rounded-lg">
      <div class="font-semibold text-sm">DrissionPage 连接示例</div>
      <pre class="text-xs bg-surface-900 text-green-400 p-3 rounded font-mono leading-relaxed overflow-x-auto">from DrissionPage import Chromium

browser = Chromium({{ debugPort }})
tab = browser.latest_tab
print(tab.title)
print(tab.url)</pre>
    </div>
  </div>
</template>
