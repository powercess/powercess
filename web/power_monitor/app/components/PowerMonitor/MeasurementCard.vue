<script setup lang="ts">
import type { DeviceInfo, MeasurementMsg } from "~/types";

// ── Props ─────────────────────────────────────────────────────────────────────

const props = defineProps<{
  device: DeviceInfo;
  measurement?: MeasurementMsg;
}>();

// ── 格式化工具 ────────────────────────────────────────────────────────────────

function fmtNum(val: number, decimals = 2): string {
  return val.toFixed(decimals);
}

/** 格式化通电时间：秒 → "X 小时 Y 分钟" */
function fmtUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h > 0) return `${h} 小时 ${m} 分钟`;
  return `${m} 分钟`;
}

/** 格式化采集时间 */
function fmtTime(iso: string): string {
  return new Date(iso).toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/** 功率因数类型标签 */
function pfTypeLabel(pfType: string): string {
  const map: Record<string, string> = {
    inductive: "感性",
    capacitive: "容性",
    resistive: "",
    感性: "感性",
    容性: "容性",
  };
  return map[pfType] ?? pfType;
}

// ── 测量值指标配置 ─────────────────────────────────────────────────────────────

const metrics = computed(() => {
  const m = props.measurement;
  if (!m) return [];

  return [
    {
      label: "电压",
      value: `${fmtNum(m.voltage_v)} V`,
      icon: "i-heroicons-bolt",
    },
    {
      label: "电流",
      value: `${fmtNum(m.current_a, 3)} A`,
      icon: "i-heroicons-arrow-right-circle",
    },
    {
      label: "功率",
      value: `${fmtNum(m.power_w)} W`,
      icon: "i-heroicons-fire",
    },
    {
      label: "频率",
      value: `${fmtNum(m.frequency_hz, 1)} Hz`,
      icon: "i-heroicons-chart-bar",
    },
    {
      label: "功率因数",
      value: `${fmtNum(m.power_factor, 3)}${pfTypeLabel(m.pf_type) ? ` (${pfTypeLabel(m.pf_type)})` : ""}`,
      icon: "i-heroicons-signal",
    },
    {
      label: "累计用电",
      value: `${fmtNum(m.energy_kwh, 3)} kWh`,
      icon: "i-heroicons-cpu-chip",
    },
    {
      label: "通电时间",
      value: fmtUptime(m.uptime_secs),
      icon: "i-heroicons-clock",
    },
  ];
});
</script>

<template>
  <UCard class="power-measurement-card">
    <!-- 卡片头部：设备信息 + 状态 -->
    <template #header>
      <div class="flex items-center justify-between gap-2">
        <div class="flex items-center gap-2 min-w-0">
          <UIcon
            name="i-heroicons-cpu-chip"
            class="text-primary-500 shrink-0 size-5"
          />
          <div class="min-w-0">
            <p class="font-semibold text-sm truncate">
              {{ device.name }}
            </p>
            <p class="text-xs text-muted font-mono truncate">
              {{ device.mac }}
            </p>
          </div>
        </div>
        <div class="flex flex-col items-end gap-1 shrink-0">
          <UBadge
            v-if="device.label"
            variant="soft"
            size="sm"
            class="max-w-32 truncate"
          >
            {{ device.label }}
          </UBadge>
          <UBadge
            :color="measurement ? 'success' : 'neutral'"
            variant="subtle"
            size="sm"
          >
            <UIcon
              :name="
                measurement ? 'i-heroicons-check-circle' : 'i-heroicons-clock'
              "
              class="mr-1 size-3"
            />
            {{ measurement ? "有数据" : "等待中" }}
          </UBadge>
        </div>
      </div>
    </template>

    <!-- 无数据占位 -->
    <div
      v-if="!measurement"
      class="flex flex-col items-center justify-center py-8 gap-2 text-muted"
    >
      <UIcon name="i-heroicons-signal-slash" class="size-10 opacity-40" />
      <p class="text-sm">暂无测量数据</p>
    </div>

    <!-- 测量值指标网格 -->
    <div v-else class="grid grid-cols-2 gap-3">
      <div
        v-for="metric in metrics"
        :key="metric.label"
        class="flex items-start gap-2 rounded-lg bg-elevated p-3"
      >
        <UIcon
          :name="metric.icon"
          class="size-4 text-primary-400 mt-0.5 shrink-0"
        />
        <div class="min-w-0">
          <p class="text-xs text-muted leading-none mb-1">
            {{ metric.label }}
          </p>
          <p class="text-sm font-semibold font-mono truncate">
            {{ metric.value }}
          </p>
        </div>
      </div>
    </div>

    <!-- 卡片底部：采集时间 -->
    <template v-if="measurement" #footer>
      <div class="flex items-center gap-1.5 text-xs text-muted">
        <UIcon name="i-heroicons-clock" class="size-3.5" />
        采集于 {{ fmtTime(measurement.recorded_at) }}
      </div>
    </template>
  </UCard>
</template>
