<script lang="ts">
  import { formatCount } from "../lib/format";
  import { t } from "../lib/i18n";

  let { value, rate, dimmed }: { value: number; rate: boolean; dimmed: boolean } = $props();
</script>

<div class="counter" class:dimmed>
  {rate ? t("perMinute", { n: formatCount(value) }) : formatCount(value)}
</div>

<style>
  .counter {
    padding: 2px 8px;
    border-radius: 999px;
    background: rgba(31, 30, 29, 0.78);
    color: #fff;
    font: 600 12px/16px -apple-system, "Segoe UI", system-ui, sans-serif;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .dimmed {
    opacity: 0.5;
  }
</style>
