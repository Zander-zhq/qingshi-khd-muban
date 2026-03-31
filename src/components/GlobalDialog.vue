<script setup lang="ts">
import { computed } from 'vue'
import { closeDialog, dialogState } from '@/utils/dialog'

const messageLines = computed(() => dialogState.message.split('\n'))

function handleMaskClick() {
  closeDialog()
}

function handleDialogClick(e: MouseEvent) {
  e.stopPropagation()
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="dialogState.visible"
      class="global-dialog-mask"
      @click="handleMaskClick"
    >
      <section class="global-dialog-panel" @click="handleDialogClick">
        <header class="global-dialog-title">{{ dialogState.title }}</header>
        <div class="global-dialog-content">
          <p v-for="(line, index) in messageLines" :key="index">{{ line }}</p>
        </div>
        <footer class="global-dialog-actions">
          <button type="button" class="global-dialog-btn" @click="closeDialog">
            {{ dialogState.confirmText }}
          </button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.global-dialog-mask {
  position: fixed;
  inset: 0;
  z-index: 3000;
  background: rgba(15, 23, 42, 0.42);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.global-dialog-panel {
  width: min(420px, 100%);
  border-radius: 14px;
  background: #fff;
  border: 1px solid #e2e8f0;
  box-shadow: 0 18px 48px rgba(15, 23, 42, 0.22);
  overflow: hidden;
}

.global-dialog-title {
  padding: 16px 18px 12px;
  font-size: 1rem;
  font-weight: 700;
  color: #0f172a;
}

.global-dialog-content {
  padding: 0 18px 16px;
  color: #334155;
  font-size: 0.9rem;
  line-height: 1.6;
  white-space: normal;
  word-break: break-word;
}

.global-dialog-content p + p {
  margin-top: 6px;
}

.global-dialog-actions {
  padding: 12px 18px 16px;
  display: flex;
  justify-content: flex-end;
}

.global-dialog-btn {
  min-width: 84px;
  height: 34px;
  border-radius: 8px;
  border: 1px solid #0d9488;
  background: linear-gradient(135deg, #0d9488, #14b8a6);
  color: #fff;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
}

.global-dialog-btn:hover {
  filter: brightness(1.03);
}
</style>
