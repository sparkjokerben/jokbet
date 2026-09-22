<script lang="ts" generics="T extends string | number">
  let {
    options,
    value = $bindable(),
    label,
  }: { options: { value: T; label: string }[]; value: T; label: string } = $props();
</script>

<div class="seg" role="radiogroup" aria-label={label}>
  {#each options as o (o.value)}
    <button
      type="button"
      role="radio"
      aria-checked={value === o.value}
      class:on={value === o.value}
      onclick={() => (value = o.value)}>{o.label}</button
    >
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    padding: 2px;
    border-radius: 7px;
    background: var(--surface-2);
    gap: 2px;
  }
  button {
    border: 0;
    background: transparent;
    padding: 3px 10px;
    border-radius: 5px;
    cursor: pointer;
    color: var(--text-secondary);
  }
  button.on {
    background: var(--surface-1);
    color: var(--text-primary);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }
</style>
