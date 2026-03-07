<script setup lang="ts">
import type { WsStatus } from "~/types";

// ── Props ─────────────────────────────────────────────────────────────────────

const props = defineProps<{
  backendOnline: boolean | null;
  wsStatus: WsStatus;
  wsError: string | null;
  lastUpdatedAt: Date | null;
  activeDevices: number;
  totalDevices: number;
  totalPowerW: number;
  totalEnergyKwh: number;
}>();

defineEmits<{
  refresh: [];
  reconnect: [];
}>();

// ── WS 状态映射 ───────────────────────────────────────────────────────────────

const wsStatusConfig = computed(() => {
  const map: Record<
    WsStatus,
    {
      label: string;
      color: "success" | "warning" | "error" | "neutral";
      icon: string;
    }
  > = {
    open: { label: "实时连接", color: "success", icon: "i-heroicons-signal" },
    connecting: {
      label: "连接中…",
      color: "warning",
      icon: "i-heroicons-arrow-path",
    },
    closed: {
      label: "已断开",
      color: "neutral",
      icon: "i-heroicons-signal-slash",
    },
    error: {
      label: "连接异常",
      color: "error",
      icon: "i-heroicons-exclamation-circle",
    },
  };
  return map[props.wsStatus];
});

/** 格式化最后更新时间 */
const lastUpdatedStr = computed(() => {
  if (!props.lastUpdatedAt) return "—";
  return props.lastUpdatedAt.toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
});
</script>

<template>
  <div class="flex flex-col gap-4">
    <!-- 顶部：标题 + 操作按钮 -->
    <div class="flex items-center justify-between flex-wrap gap-3">
      <div class="flex items-center gap-3">
        <UIcon name="i-heroicons-bolt" class="size-7 text-primary-500" />
        <div>
          <h1 class="text-xl font-bold leading-tight">用电监控</h1>
          <p class="text-sm text-muted">实时追踪 BLE 功率计数据</p>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <UButton
          variant="ghost"
          size="sm"
          icon="i-heroicons-arrow-path"
          title="刷新快照"
          @click="$emit('refresh')"
        >
          刷新
        </UButton>
        <UButton
          v-if="wsStatus !== 'open'"
          variant="soft"
          color="primary"
          size="sm"
          icon="i-heroicons-arrow-path-rounded-square"
          @click="$emit('reconnect')"
        >
          重连 WS
        </UButton>
      </div>
    </div>

    <!-- 状态栏 -->
    <div class="flex flex-wrap items-center gap-2">
      <!-- 后端状态 -->
      <UBadge
        :color="
          backendOnline === null
            ? 'neutral'
            : backendOnline
              ? 'success'
              : 'error'
        "
        variant="subtle"
        size="md"
      >
        <UIcon
          :name="
            backendOnline ? 'i-heroicons-server' : 'i-heroicons-server-stack'
          "
          class="mr-1.5 size-3.5"
        />
        {{
          backendOnline === null
            ? "检测中…"
            : backendOnline
              ? "后端在线"
              : "后端离线"
        }}
      </UBadge>

      <!-- WebSocket 状态 -->
      <UBadge :color="wsStatusConfig.color" variant="subtle" size="md">
        <UIcon
          :name="wsStatusConfig.icon"
          class="mr-1.5 size-3.5"
          :class="{ 'animate-spin': wsStatus === 'connecting' }"
        />
        {{ wsStatusConfig.label }}
        <span v-if="wsError" class="ml-1 opacity-70">· {{ wsError }}</span>
      </UBadge>

      <!-- 最后更新时间 -->
      <UBadge variant="soft" color="neutral" size="md">
        <UIcon name="i-heroicons-clock" class="mr-1.5 size-3.5" />
        更新于 {{ lastUpdatedStr }}
      </UBadge>
    </div>

    <!-- 统计概览 -->
    <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
      <UCard class="text-center" :ui="{ body: 'p-4' }">
        <p class="text-2xl font-bold text-primary-500">
          {{ activeDevices }}
          <span class="text-base font-normal text-muted"
            >/ {{ totalDevices }}</span
          >
        </p>
        <p class="text-xs text-muted mt-1">在线设备</p>
      </UCard>

      <UCard class="text-center" :ui="{ body: 'p-4' }">
        <p class="text-2xl font-bold text-warning-500">
          {{ totalPowerW.toFixed(1) }}
          <span class="text-sm font-normal text-muted">W</span>
        </p>
        <p class="text-xs text-muted mt-1">总有功功率</p>
      </UCard>

      <UCard class="text-center" :ui="{ body: 'p-4' }">
        <p class="text-2xl font-bold text-success-500">
          {{ totalEnergyKwh.toFixed(3) }}
          <span class="text-sm font-normal text-muted">kWh</span>
        </p>
        <p class="text-xs text-muted mt-1">总用电量</p>
      </UCard>

      <UCard class="text-center" :ui="{ body: 'p-4' }">
        <div class="flex justify-center items-center h-8">
          <div
            class="size-4 rounded-full"
            :class="
              wsStatus === 'open'
                ? 'bg-success-400 animate-pulse'
                : 'bg-neutral-300 dark:bg-neutral-600'
            "
          />
        </div>
        <p class="text-xs text-muted mt-1">实时推流</p>
      </UCard>
    </div>
  </div>
</template>
