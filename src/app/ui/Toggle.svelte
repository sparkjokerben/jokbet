<script lang="ts">
  let {
    checked,
    label,
    onchange,
    disabled = false,
  }: { checked: boolean; label: string; onchange: (v: boolean) => void; disabled?: boolean } = $props();
</script>

<label class="toggle" class:disabled>
  <input type="checkbox" role="switch" {checked} {disabled} onchange={(e) => onchange(e.currentTarget.checked)} />
  <span class="track" aria-hidden="true"><span class="thumb"></span></span>
  <span>{label}</span>
</label>

<style>
  .toggle {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    padding: 4px 0;
  }
  .disabled {
    opacity: 0.5;
    cursor: default;
  }
  input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
  }
  .track {
    flex: none;
    width: 30px;
    height: 18px;
    border-radius: 9px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    position: relative;
    transition: background 0.15s;
  }
  .thumb {
    position: absolute;
    top: 1px;
    left: 1px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
    transition: transform 0.15s;
  }
  input:checked + .track {
    background: var(--accent);
    border-color: var(--accent);
  }
  input:checked + .track .thumb {
    transform: translateX(12px);
  }
  input:focus-visible + .track {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
</style>
