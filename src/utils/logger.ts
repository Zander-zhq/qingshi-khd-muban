type LogLevel = 'log' | 'warn' | 'error'

function maskValue(key: string, value: unknown): unknown {
  const lowerKey = key.toLowerCase()
  if (['password', 'new_password', 'token', 'authorization', 'sign', 'x-sign'].includes(lowerKey)) {
    return '[masked]'
  }
  return value
}

function sanitizePayload(payload: unknown): unknown {
  if (Array.isArray(payload)) {
    return payload.map((item) => sanitizePayload(item))
  }

  if (payload && typeof payload === 'object') {
    return Object.fromEntries(
      Object.entries(payload as Record<string, unknown>).map(([key, value]) => [key, sanitizePayload(maskValue(key, value))]),
    )
  }

  return payload
}

function print(level: LogLevel, scope: string, message: string, payload?: unknown) {
  const prefix = `[QS][${scope}] ${message}`
  if (payload === undefined) {
    console[level](prefix)
    return
  }

  console[level](prefix, sanitizePayload(payload))
}

export const logger = {
  log(scope: string, message: string, payload?: unknown) {
    print('log', scope, message, payload)
  },
  warn(scope: string, message: string, payload?: unknown) {
    print('warn', scope, message, payload)
  },
  error(scope: string, message: string, payload?: unknown) {
    print('error', scope, message, payload)
  },
}
