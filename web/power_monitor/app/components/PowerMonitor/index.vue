<script setup lang="ts">
/**
 * PowerMonitor 主组件
 *
 * 功能：
 * - 展示所有受监控 BLE 设备的实时用电数据
 * - 通过 WebSocket 实时接收后端推送（每次采集后立即下发）
 * - 初始化时通过 REST 接口拉取设备列表及最新快照
 *
 * 该组件只负责渲染，所有数据获取封装在 usePowerMonitor composable 中。
 */

const {
  deviceMeasurements,
  stats,
  isLoading,
  backendOnline,
  fetchError,
  lastUpdatedAt,
  wsStatus,
  wsError,
  refresh,
  reconnect,
} = usePowerMonitor()
</script>

<template>
  <div class="power-monitor flex flex-col gap-6 p-4 md:p-6">
    <!-- 状态栏 + 统计 -->
    <PowerMonitorStatusBar
      :backend-online="backendOnline"
      :ws-status="wsStatus"
      :ws-error="wsError"
      :last-updated-at="lastUpdatedAt"
      :active-devices="stats.activeDevices"
      :total-devices="stats.totalDevices"
      :total-power-w="stats.totalPowerW"
      :total-energy-kwh="stats.totalEnergyKwh"
      @refresh="refresh"
      @reconnect="reconnect"
    />

    <!-- 错误提示 -->
    <UAlert
      v-if="fetchError"
      color="error"
      variant="subtle"
      :title="fetchError"
      icon="i-heroicons-exclamation-triangle"
    />

    <!-- 加载骨架 -->
    <div
      v-if="isLoading"
      class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3"
    >
      <UCard
        v-for="n in 3"
        :key="n"
      >
        <div class="flex flex-col gap-3">
          <div class="flex items-center gap-2">
            <USkeleton class="size-5 rounded-full" />
            <USkeleton class="h-4 w-32" />
          </div>
          <div class="grid grid-cols-2 gap-2">
            <USkeleton
              v-for="i in 6"
              :key="i"
              class="h-14 rounded-lg"
            />
          </div>
        </div>
      </UCard>
    </div>

    <!-- 设备卡片列表 -->
    <div
      v-else-if="deviceMeasurements.length > 0"
      class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3"
    >
      <PowerMonitorMeasurementCard
        v-for="{ device, measurement } in deviceMeasurements"
        :key="device.mac"
        :device="device"
        :measurement="measurement"
      />
    </div>

    <!-- 无设备占位 -->
    <div
      v-else
      class="flex flex-col items-center justify-center py-20 gap-4 text-muted"
    >
      <UIcon
        name="i-heroicons-device-phone-mobile"
        class="size-16 opacity-30"
      />
      <p class="text-base">
        未发现受监控设备
      </p>
      <UButton
        variant="soft"
        icon="i-heroicons-arrow-path"
        @click="refresh"
      >
        重新拉取
      </UButton>
    </div>
  </div>
</template>
