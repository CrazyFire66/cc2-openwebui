<script lang="ts">
  import { onMount } from 'svelte';
  import { getDebugInfo, scanNetwork, type DebugInfo, type DiscoveredPrinter } from '../../api';
  import { toErrorMessage } from '../errors';

  let info: DebugInfo | null = null;
  let loading = false;
  let error = '';
  let scanning = false;
  let scanError = '';
  let scanResults: DiscoveredPrinter[] = [];

  async function refresh() {
    loading = true;
    error = '';
    try {
      info = await getDebugInfo();
    } catch (e) {
      error = toErrorMessage(e) || 'Debug refresh failed';
    } finally {
      loading = false;
    }
  }

  async function runScan() {
    scanning = true;
    scanError = '';
    scanResults = [];
    try {
      const res = await scanNetwork();
      scanResults = res.printers;
      if (!scanResults.length) scanError = 'No MQTT candidates found.';
      await refresh();
    } catch (e) {
      scanError = toErrorMessage(e) || 'Network scan failed';
    } finally {
      scanning = false;
    }
  }

  onMount(refresh);
</script>

<div class="debug-actions">
  <button class="btn sm" on:click={refresh} disabled={loading}>{loading ? 'Refreshing...' : 'Refresh'}</button>
  <button class="btn sm" on:click={runScan} disabled={scanning}>{scanning ? 'Scanning...' : 'Run LAN scan'}</button>
</div>

{#if error}
  <div class="alert err">{error}</div>
{/if}

{#if info}
  <div class="debug-grid">
    <div class="debug-row"><span>Version</span><strong>{info.version}</strong></div>
    <div class="debug-row"><span>Configured</span><strong>{info.configured ? 'Yes' : 'No'}</strong></div>
    <div class="debug-row"><span>Active profile</span><strong>{info.active_printer_id || '--'}</strong></div>
    <div class="debug-row"><span>Profiles</span><strong>{info.printer_count}</strong></div>
    <div class="debug-row"><span>Printer IP</span><strong>{info.printer_ip || '--'}</strong></div>
    <div class="debug-row"><span>Printer ID</span><strong>{info.printer_id || '--'}</strong></div>
    <div class="debug-row"><span>MQTT raw</span><strong class:ok={info.connected_raw}>{info.connected_raw ? 'Connected' : 'Offline'}</strong></div>
    <div class="debug-row"><span>MQTT ws</span><strong class:ok={info.connected_ws}>{info.connected_ws ? 'Connected' : 'Offline'}</strong></div>
    <div class="debug-row"><span>Subnets</span><strong>{info.detected_subnets.length ? info.detected_subnets.map((s) => `${s}.0/24`).join(', ') : '--'}</strong></div>
    <div class="debug-row"><span>AI detection</span><strong>{info.detection_enabled ? 'Enabled' : 'Disabled'}</strong></div>
    <div class="debug-row"><span>Obico URL</span><strong>{info.obico_url}</strong></div>
    <div class="debug-row"><span>Notifications</span><strong>{info.notification_destinations}</strong></div>
  </div>
{/if}

{#if scanResults.length || scanError}
  <div class="scan-block">
    <div class="scan-title">Last scan</div>
    {#if scanError}<div class="alert err">{scanError}</div>{/if}
    {#each scanResults as printer}
      <div class="scan-row">
        <span class="mono">{printer.ip}</span>
        <span class="chip" class:ok={printer.verified}>{printer.verified ? 'CC2' : printer.needs_pincode ? 'PIN needed' : 'Candidate'}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .debug-actions { display: flex; gap: 8px; flex-wrap: wrap; }
  .debug-grid {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    overflow: hidden;
  }
  .debug-row {
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: 12px;
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    font-size: 12px;
  }
  .debug-row:first-child { border-top: none; }
  .debug-row span { color: var(--muted); }
  .debug-row strong { color: var(--text); font-weight: 500; overflow-wrap: anywhere; }
  .debug-row strong.ok { color: var(--success); }
  .scan-block {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    padding: 12px;
  }
  .scan-title { color: var(--muted); font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; margin-bottom: 8px; }
  .scan-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 8px 0; border-top: 1px solid var(--border); }
  .scan-row:first-of-type { border-top: none; }
  .chip { color: var(--warning); background: var(--warning-dim); border: 1px solid rgba(192,120,40,0.35); border-radius: var(--radius-pill); padding: 2px 7px; font-size: 10.5px; white-space: nowrap; }
  .chip.ok { color: var(--success); background: var(--success-dim); border-color: rgba(40,160,100,0.35); }
  .alert { padding: 10px 12px; border-radius: 7px; font-size: 12px; }
  .alert.err { color: var(--danger); background: var(--danger-dim); border: 1px solid rgba(192,57,74,0.35); }

  @media (max-width: 620px) {
    .debug-row { grid-template-columns: 1fr; gap: 3px; }
  }
</style>
