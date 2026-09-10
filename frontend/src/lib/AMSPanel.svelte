<script lang="ts">
  import { Disc2 } from 'lucide-svelte';
  import { printer, showToast, type CanvasInfo, type TrayEntry } from '../stores';
  import { canvasLoad, canvasUnload, refreshCanvas, saveCanvasSlot, setCanvasAutoRefill } from '../api';
  import { toErrorMessage } from './errors';

  let refreshing = false;
  async function handleRefreshCanvas(silent = false) {
    refreshing = true;
    try {
      await refreshCanvas();
      if (!silent) showToast('Canvas refreshed', 'info');
    } catch (e) {
      showToast(toErrorMessage(e));
    } finally {
      refreshing = false;
    }
  }

  $: s = $printer.state;
  $: filamentDetected = s?.extruder?.filament_detected === 1;
  $: filamentDetectEnabled = s?.extruder?.filament_detect_enable === 1;
  $: connected = $printer.connected;
  $: activePrint = s?.print_status?.state === 'printing' || s?.print_status?.state === 'paused';

  $: canvasInfo = s?.canvas_info as CanvasInfo | undefined;
  $: activeCanvasId = canvasInfo?.active_canvas_id ?? 0;
  $: activeTrayId = canvasInfo?.active_tray_id ?? -1;
  $: canvas = canvasInfo?.canvas_list?.find((c) => c.canvas_id === activeCanvasId)
    ?? canvasInfo?.canvas_list?.[0];
  let trayList: TrayEntry[] = [];
  $: trayList = canvas?.tray_list ?? [];

  type Spool = {
    id: number;
    color: string | null;
    material: string;
    plasticType: string;
    plasticFamily: string;
    brand: string;
    empty: boolean;
  };

  $: spools = [0, 1, 2, 3].map<Spool>((i) => {
    const tray = trayList[i];
    if (!tray) return { id: i + 1, color: null, material: '-', plasticType: '-', plasticFamily: '-', brand: '', empty: true };
    const rawColor = tray.filament_color?.trim();
    const hexOnly = rawColor?.startsWith('#') ? rawColor.slice(1) : rawColor;
    const color = hexOnly && hexOnly.length >= 6 ? `#${hexOnly.slice(0, 6)}` : null;
    const material = (tray.tray_type || tray.filament_type || tray.filament_name || '').toUpperCase() || '-';
    const plasticType = tray.filament_name || '-';
    const plasticFamily = tray.filament_type || '-';
    const brand = tray.brand || '';
    return { id: i + 1, color, material, plasticType, plasticFamily, brand, empty: !color };
  });

  let loadedSlot = -1;
  $: {
    if (activeTrayId >= 0) {
      loadedSlot = trayList.findIndex((t) => t?.tray_id === activeTrayId);
    } else {
      loadedSlot = trayList.findIndex((t) => t?.status === 1);
    }
  }

  let selected = 0;
  $: selectedTray = trayList[selected];
  $: selectedTrayId = selectedTray?.tray_id ?? -1;
  const canvasSlotEditingEnabled = false;
  $: canOperateSlot = connected && !activePrint && !!selectedTray && selectedTrayId >= 0;
  $: canEditSlot = canvasSlotEditingEnabled && canOperateSlot;
  $: canLoadSlot = canOperateSlot && loadedSlot !== selected;
  $: canUnloadSlot = canOperateSlot && loadedSlot === selected;
  $: editSlotTitle = canvasSlotEditingEnabled
    ? activePrint ? 'Disabled while a print is active' : 'Edit selected slot'
    : 'Canvas slot editing is temporarily disabled';

  let actionPending: 'load' | 'unload' | 'save' | null = null;
  let editOpen = false;
  let editFilamentName = '';
  let editFilamentType = '';
  let editFilamentColor = '#888888';
  let editBrand = '';
  let editFilamentCode = '';
  let editMinTemp = 190;
  let editMaxTemp = 240;

  function selectedPayload() {
    return {
      canvas_id: activeCanvasId,
      tray_id: selectedTrayId,
      tray_slot: selected,
    };
  }

  async function handleLoad() {
    if (!canLoadSlot) return;
    actionPending = 'load';
    try {
      await canvasLoad(selectedPayload());
      await handleRefreshCanvas(true);
      showToast(`Slot ${selected + 1} loaded`, 'info');
    } catch (e) {
      showToast(toErrorMessage(e), 'error', 6000);
    } finally {
      actionPending = null;
    }
  }

  async function handleUnload() {
    if (!canUnloadSlot) return;
    actionPending = 'unload';
    try {
      await canvasUnload(selectedPayload());
      await handleRefreshCanvas(true);
      showToast(`Slot ${selected + 1} unloaded`, 'info');
    } catch (e) {
      showToast(toErrorMessage(e), 'error', 6000);
    } finally {
      actionPending = null;
    }
  }

  function openEdit() {
    if (!canEditSlot) return;
    const tray = selectedTray;
    editFilamentName = tray?.filament_name || tray?.tray_type || '';
    editFilamentType = tray?.filament_type || '';
    editBrand = tray?.brand || '';
    editFilamentCode = tray?.filament_code || '';
    const rawColor = tray?.filament_color?.trim() || '888888';
    editFilamentColor = rawColor.startsWith('#') ? rawColor : `#${rawColor.slice(0, 6)}`;
    editMinTemp = tray?.min_nozzle_temp || 190;
    editMaxTemp = tray?.max_nozzle_temp || 240;
    editOpen = true;
  }

  async function saveEdit() {
    if (!canEditSlot) return;
    actionPending = 'save';
    try {
      await saveCanvasSlot({
        ...selectedPayload(),
        filament_name: editFilamentName.trim(),
        filament_type: editFilamentType.trim(),
        filament_color: editFilamentColor,
        brand: editBrand.trim(),
        filament_code: editFilamentCode.trim(),
        min_nozzle_temp: Math.round(editMinTemp),
        max_nozzle_temp: Math.round(editMaxTemp),
      });
      editOpen = false;
      await handleRefreshCanvas(true);
      showToast(`Slot ${selected + 1} saved`, 'info');
    } catch (e) {
      showToast(toErrorMessage(e), 'error', 6000);
    } finally {
      actionPending = null;
    }
  }

  // auto-refill toggle with optimistic UI
  $: autoRefill = canvasInfo?.auto_refill ?? false;
  let autoRefillPending: boolean | null = null;
  $: displayAutoRefill = autoRefillPending ?? autoRefill;

  async function handleAutoRefill(val: boolean) {
    const prev = autoRefill;
    autoRefillPending = val;
    try {
      await setCanvasAutoRefill(val);
    } catch (e) {
      autoRefillPending = prev;
      showToast(toErrorMessage(e), 'error', 5000);
    } finally {
      autoRefillPending = null;
    }
  }

  function labelColorFor(bg: string | null): string {
    if (!bg) return 'var(--muted)';
    const hex = bg.replace('#', '');
    if (hex.length < 6) return '#fff';
    const r = parseInt(hex.slice(0, 2), 16);
    const g = parseInt(hex.slice(2, 4), 16);
    const b = parseInt(hex.slice(4, 6), 16);
    const lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
    return lum > 0.6 ? '#111' : '#fff';
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div class="title-row">
      <Disc2 size={12} strokeWidth={2} color="var(--accent)" aria-hidden="true" />
      <span class="panel-title">Canvas</span>
    </div>
    <div style="display:flex;align-items:center;gap:6px;">
      {#if !filamentDetectEnabled && connected}
        <span class="chip warn">Sensor off</span>
      {/if}
      <button class="refresh-btn" on:click={handleRefreshCanvas} disabled={refreshing} title="Refresh canvas data from printer">
        <svg width="12" height="12" viewBox="0 0 13 13" fill="none" class:spin={refreshing}>
          <path d="M11 6.5a4.5 4.5 0 11-1.3-3.2" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
          <path d="M11 2v3h-3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    </div>
  </div>

  <div class="ams-body">
    <div class="spools-grid">
      {#each spools as sp, i}
        <button
          class="spool"
          class:selected={selected === i}
          class:loaded={loadedSlot === i}
          class:empty={sp.empty}
          on:click={() => (selected = i)}
          aria-label={`Slot ${sp.id}: ${sp.material}${loadedSlot === i ? ' (loaded)' : ''}`}
        >
          <span class="spool-num">{sp.id}</span>
          <span class="spool-disc" style={sp.color ? `--spool-fill:${sp.color}; --spool-text:${labelColorFor(sp.color)};` : ''}>
            <span class="spool-hub"></span>
            <span class="spool-material">{sp.material}</span>
          </span>
          {#if loadedSlot === i}
            <span class="loaded-tag" title="Currently loaded">
              <svg width="9" height="9" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                <path d="M3 6.5L5 8.5L9 4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              Loaded
            </span>
          {/if}
          {#if sp.brand?.toLowerCase() === 'elegoo'}
            <span class="elegoo-tag" title="Elegoo filament">Elegoo</span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="slot-actions">
      <div class="slot-info">
        <div class="info-row">
          <span class="info-label">Slot</span>
          <span class="info-value mono">{spools[selected]?.id ?? '-'}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Type</span>
          <span class="info-value">{spools[selected]?.plasticType ?? '-'}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Family</span>
          <span class="info-value">{spools[selected]?.plasticFamily ?? '-'}</span>
        </div>
        <div class="info-row">
          <span class="info-label">State</span>
          <span class="info-value">
            {#if loadedSlot === selected}
              <span class="mini-chip on">Loaded</span>
            {:else if spools[selected]?.empty}
              <span class="mini-chip off">Empty</span>
            {:else}
              <span class="mini-chip">Idle</span>
            {/if}
          </span>
        </div>
      </div>

      <div class="slot-buttons">
        <button class="btn block" disabled={!canLoadSlot || actionPending !== null} on:click={handleLoad} title={activePrint ? 'Disabled while a print is active' : 'Load selected slot'}>
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M8 2v9M4.5 7.5L8 11l3.5-3.5M3 13.5h10" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          {actionPending === 'load' ? 'Loading...' : 'Load'}
        </button>
        <button class="btn block" disabled={!canUnloadSlot || actionPending !== null} on:click={handleUnload} title={activePrint ? 'Disabled while a print is active' : 'Unload selected slot'}>
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M8 13V4M4.5 7.5L8 4l3.5 3.5M3 2.5h10" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          {actionPending === 'unload' ? 'Unloading...' : 'Unload'}
        </button>
        <button class="btn block" disabled={!canEditSlot || actionPending !== null} on:click={openEdit} title={editSlotTitle}>
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M11.5 2.5l2 2L6 12H4v-2l7.5-7.5z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>
          </svg>
          Edit
        </button>
      </div>
    </div>
  </div>

  <div class="footer-row">
    <span class="footer-label">Auto Refill</span>
    <label class="toggle" title="Automatically switch to next spool when current runs out">
      <input
        type="checkbox"
        checked={displayAutoRefill}
        disabled={!connected}
        on:change={(e) => handleAutoRefill(e.currentTarget.checked)}
      />
      <span class="knob"></span>
    </label>
  </div>
</section>

{#if editOpen}
  <div class="modal-backdrop" role="presentation" on:click={() => editOpen = false}>
    <div class="modal-sheet slot-modal" role="dialog" aria-modal="true" aria-label={`Edit Canvas slot ${selected + 1}`} on:click|stopPropagation>
      <div class="modal-header">
        <div>
          <h2>Edit Slot {selected + 1}</h2>
          <p>Canvas {activeCanvasId} · Tray {selectedTrayId}</p>
        </div>
        <button class="icon-btn" on:click={() => editOpen = false} aria-label="Close">×</button>
      </div>
      <div class="modal-body">
        <label class="field">
          <span class="field-label">Name</span>
          <input class="input" bind:value={editFilamentName} placeholder="PLA White" />
        </label>
        <label class="field">
          <span class="field-label">Type</span>
          <input class="input" bind:value={editFilamentType} placeholder="PLA" />
        </label>
        <label class="field">
          <span class="field-label">Brand</span>
          <input class="input" bind:value={editBrand} placeholder="Elegoo" />
        </label>
        <label class="field">
          <span class="field-label">Filament Code</span>
          <input class="input mono" bind:value={editFilamentCode} placeholder="optional" />
        </label>
        <div class="field-grid">
          <label class="field">
            <span class="field-label">Color</span>
            <div class="color-line">
              <input class="color-picker" type="color" bind:value={editFilamentColor} />
              <input class="input mono" bind:value={editFilamentColor} />
            </div>
          </label>
          <label class="field">
            <span class="field-label">Min Nozzle</span>
            <input class="input mono" type="number" min="0" max="350" bind:value={editMinTemp} />
          </label>
          <label class="field">
            <span class="field-label">Max Nozzle</span>
            <input class="input mono" type="number" min="0" max="350" bind:value={editMaxTemp} />
          </label>
        </div>
      </div>
      <div class="modal-footer">
        <button class="btn ghost" on:click={() => editOpen = false}>Cancel</button>
        <button class="btn primary" disabled={actionPending !== null} on:click={saveEdit}>
          {actionPending === 'save' ? 'Saving...' : 'Save Slot'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .title-row {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .refresh-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    background: var(--surface2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .refresh-btn:hover:not(:disabled) { background: var(--border); color: var(--text); }
  .refresh-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .spin { animation: spin 0.9s linear infinite; transform-origin: center; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .ams-body {
    padding: 14px;
    display: grid;
    grid-template-columns: 1fr 150px;
    gap: 14px;
  }

  .spools-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    grid-auto-flow: column;
    gap: 10px;
    min-width: 0;
  }

  /* spool chip */
  .spool {
    --spool-fill: var(--surface3);
    --spool-text: var(--muted);
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface2);
    text-align: left;
    transition: border-color 0.15s, background 0.15s, transform 0.15s, box-shadow 0.15s;
  }
  .spool:hover { border-color: var(--border2); transform: translateY(-1px); box-shadow: 0 4px 10px -6px rgba(0,0,0,0.5); }
  .spool.selected {
    border-color: var(--accent);
    background: var(--accent-dim);
  }
  .spool.loaded { border-color: var(--border2); }
  .spool.loaded.selected { border-color: var(--accent); }

  .spool-num {
    font-size: 13px;
    font-weight: 700;
    color: var(--muted);
    font-family: var(--font-mono);
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }
  .spool.selected .spool-num { color: var(--accent); }

  .spool-disc {
    position: relative;
    width: 52px;
    height: 52px;
    border-radius: 50%;
    background: var(--spool-fill);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow:
      inset 0 0 0 1px rgba(255,255,255,0.08),
      inset 0 0 0 6px var(--spool-fill),
      inset 0 0 0 7px rgba(0,0,0,0.18);
  }
  .spool.empty .spool-disc {
    background:
      repeating-linear-gradient(
        45deg,
        var(--surface3) 0 4px,
        var(--surface2) 4px 8px
      );
    box-shadow: inset 0 0 0 1px var(--border2);
  }

  .spool-hub {
    position: absolute;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--bg-deep);
    border: 1px solid rgba(0,0,0,0.3);
  }

  .spool-material {
    position: relative;
    z-index: 1;
    font-size: 10.5px;
    font-weight: 700;
    color: var(--spool-text);
    letter-spacing: 0.04em;
    text-shadow: 0 1px 1px rgba(0,0,0,0.25);
    background: var(--spool-fill);
    padding: 2px 4px;
    border-radius: 2px;
  }
  .spool.empty .spool-material {
    color: var(--muted);
    background: transparent;
    text-shadow: none;
  }

  .loaded-tag {
    position: absolute;
    top: 6px;
    right: 6px;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--success);
    background: var(--success-dim);
    border: 1px solid rgba(74,140,92,0.3);
    padding: 2px 5px;
    border-radius: var(--radius-pill);
  }

  .elegoo-tag {
    position: absolute;
    bottom: 6px;
    right: 6px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--accent);
    background: rgba(45,135,240,0.12);
    border: 1px solid rgba(45,135,240,0.3);
    padding: 2px 5px;
    border-radius: var(--radius-pill);
  }

  /* slot actions */
  .slot-actions {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .slot-info {
    padding: 10px 11px;
    background: var(--surface2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .info-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .info-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
  }
  .info-value {
    font-size: 12px;
    color: var(--text);
  }

  .mini-chip {
    display: inline-flex;
    align-items: center;
    padding: 1px 7px;
    border-radius: var(--radius-pill);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    background: var(--surface3);
    color: var(--muted);
    border: 1px solid var(--border);
  }
  .mini-chip.on { color: var(--success); background: var(--success-dim); border-color: rgba(74,140,92,0.35); }
  .mini-chip.off { color: var(--muted2); }

  .slot-buttons {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .slot-buttons .btn {
    justify-content: flex-start;
    padding-left: 10px;
  }

  @media (max-width: 560px) {
    .ams-body { grid-template-columns: 1fr; }
  }

  .footer-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 14px 10px;
    border-top: 1px solid var(--border);
  }
  .footer-label {
    font-size: 12px;
    color: var(--muted);
  }

  .slot-modal {
    width: min(480px, calc(100vw - 32px));
  }
  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 18px 12px;
    border-bottom: 1px solid var(--border);
  }
  .modal-header h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 650;
    color: var(--text);
  }
  .modal-header p {
    margin-top: 2px;
    font-size: 12px;
    color: var(--muted);
  }
  .icon-btn {
    width: 28px;
    height: 28px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface2);
    color: var(--muted);
    font-size: 18px;
    line-height: 1;
  }
  .icon-btn:hover { color: var(--text); border-color: var(--border2); }
  .modal-body {
    padding: 16px 18px 4px;
  }
  .field-grid {
    display: grid;
    grid-template-columns: 1.3fr 1fr 1fr;
    gap: 10px;
  }
  .color-line {
    display: grid;
    grid-template-columns: 36px 1fr;
    gap: 8px;
    align-items: center;
  }
  .color-picker {
    width: 36px;
    height: 36px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
  }
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px 16px;
    border-top: 1px solid var(--border);
  }

  .toggle { position: relative; display: block; width: 36px; height: 20px; cursor: pointer; flex-shrink: 0; }
  .toggle input { opacity: 0; width: 0; height: 0; position: absolute; }
  .toggle input:disabled + .knob { opacity: 0.45; cursor: not-allowed; }
  .knob {
    position: absolute;
    inset: 0;
    background: var(--surface2);
    border: 1px solid var(--border2);
    border-radius: 20px;
    transition: background 0.2s;
  }
  .knob::before {
    content: '';
    position: absolute;
    width: 14px; height: 14px;
    left: 2px; top: 2px;
    background: var(--muted);
    border-radius: 50%;
    transition: transform 0.2s, background 0.2s;
  }
  input:checked + .knob { background: var(--surface2); border-color: var(--border2); }
  input:checked + .knob::before { transform: translateX(16px); background: var(--accent); }

  @media (max-width: 560px) {
    .field-grid { grid-template-columns: 1fr; }
  }
</style>
