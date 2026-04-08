import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { unbindDeviceApi } from '../api/auth'
import { getAppCredentials } from '../utils/config'

export interface DeviceItem {
  device_sn: string
  last_login: string
}

export function useUnbindDevicePage() {
  const router = useRouter()
  const appId = ref('')

  onMounted(async () => {
    const creds = await getAppCredentials()
    appId.value = creds.appId
  })

  const acctno = ref('')
  const password = ref('')
  const loading = ref(false)
  const errMsg = ref('')
  const successMsg = ref('')

  const showDeviceList = ref(false)
  const devices = ref<DeviceItem[]>([])
  const selectedDevice = ref('')

  function clearMsg() {
    errMsg.value = ''
    successMsg.value = ''
  }

  async function handleUnbind(deviceSn?: string) {
    clearMsg()
    if (!acctno.value.trim()) { errMsg.value = '请输入账号（手机号）'; return }
    if (!password.value) { errMsg.value = '请输入密码'; return }

    loading.value = true
    try {
      const params: Record<string, string> = {
        app_id: appId.value,
        acctno: acctno.value.trim(),
        password: password.value,
      }
      if (deviceSn) params.device_sn = deviceSn

      const res = await unbindDeviceApi(params as any)

      if ((res as any).need_select && (res as any).devices) {
        devices.value = (res as any).devices
        selectedDevice.value = ''
        showDeviceList.value = true
      } else {
        showDeviceList.value = false
        successMsg.value = '解绑成功'
        setTimeout(() => router.push('/login'), 1500)
      }
    } catch (err: unknown) {
      errMsg.value = err instanceof Error ? err.message : '解绑失败'
    } finally {
      loading.value = false
    }
  }

  function handleSubmit() {
    handleUnbind()
  }

  function handleSelectDevice() {
    if (!selectedDevice.value) {
      errMsg.value = '请选择要解绑的设备'
      return
    }
    handleUnbind(selectedDevice.value)
  }

  return {
    router,
    acctno,
    password,
    loading,
    errMsg,
    successMsg,
    showDeviceList,
    devices,
    selectedDevice,
    clearMsg,
    handleSubmit,
    handleSelectDevice,
  }
}
