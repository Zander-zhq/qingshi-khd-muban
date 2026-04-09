<script setup lang="ts">
import { getBrand } from '../brand'

const brand = getBrand()

defineProps<{
  visible: boolean
  showAcceptButton?: boolean
}>()

const emit = defineEmits<{
  close: []
  accept: []
}>()
</script>

<template>
  <Transition name="modal">
    <div v-if="visible" class="disclaimer-overlay" @click.self="emit('close')">
      <div class="disclaimer-box">
        <div class="disclaimer-header">
          <div class="disclaimer-icon">
            <i class="pi pi-shield"></i>
          </div>
          <h3>免责声明</h3>
          <button type="button" class="disclaimer-close" @click="emit('close')">
            <i class="pi pi-times"></i>
          </button>
        </div>
        <div class="disclaimer-body">
          <div class="disclaimer-brand">{{ brand.brand_name }} · {{ brand.product_name }}</div>
          <div class="disclaimer-content">{{ $props.visible ? brand.disclaimer : '' }}</div>
        </div>
        <div class="disclaimer-footer">
          <button v-if="showAcceptButton" class="disclaimer-btn disclaimer-btn--primary" @click="emit('accept')">
            <i class="pi pi-check"></i>
            已阅读并同意
          </button>
          <button v-else class="disclaimer-btn disclaimer-btn--secondary" @click="emit('close')">
            我知道了
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.disclaimer-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.5);
  backdrop-filter: blur(3px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1200;
}

.disclaimer-box {
  width: 520px;
  max-width: 90vw;
  max-height: 80vh;
  background: #fff;
  border-radius: 16px;
  box-shadow: 0 20px 60px rgba(15, 23, 42, 0.25);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.disclaimer-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 20px 24px 16px;
  border-bottom: 1px solid #f1f5f9;
}

.disclaimer-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: var(--app-primary-light, #f0fdf4);
  color: var(--app-primary, #22c55e);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.1rem;
  flex-shrink: 0;
}

.disclaimer-header h3 {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 700;
  color: #0f172a;
  flex: 1;
}

.disclaimer-close {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  color: #94a3b8;
  border-radius: 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.disclaimer-close:hover {
  background: #f1f5f9;
  color: #334155;
}

.disclaimer-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}

.disclaimer-brand {
  font-size: 0.78rem;
  color: #94a3b8;
  margin-bottom: 12px;
  letter-spacing: 0.05em;
}

.disclaimer-content {
  font-size: 0.88rem;
  color: #334155;
  line-height: 1.8;
  white-space: pre-wrap;
  word-break: break-word;
}

.disclaimer-footer {
  padding: 16px 24px 20px;
  border-top: 1px solid #f1f5f9;
  display: flex;
  justify-content: center;
}

.disclaimer-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-width: 160px;
  padding: 10px 28px;
  border: none;
  border-radius: 10px;
  font-size: 0.9rem;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.2s;
}

.disclaimer-btn--primary {
  background: var(--app-primary, #22c55e);
  color: #fff;
  box-shadow: 0 4px 14px rgba(34, 197, 94, 0.3);
}

.disclaimer-btn--primary:hover {
  box-shadow: 0 6px 20px rgba(34, 197, 94, 0.4);
  transform: translateY(-1px);
}

.disclaimer-btn--secondary {
  background: #f1f5f9;
  color: #334155;
}

.disclaimer-btn--secondary:hover {
  background: #e2e8f0;
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-active .disclaimer-box,
.modal-leave-active .disclaimer-box {
  transition: transform 0.2s ease;
}
.modal-enter-from .disclaimer-box {
  transform: scale(0.95);
}
.modal-leave-to .disclaimer-box {
  transform: scale(0.95);
}
</style>
