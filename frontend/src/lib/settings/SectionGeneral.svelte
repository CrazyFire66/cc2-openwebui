<script lang="ts">
  import { checkForUpdates, type PrinterProfile, type VersionInfo } from '../../api';

  export let printer: { ip: string; printer_id: string; pincode: string };
  export let printers: PrinterProfile[] = [];
  export let activePrinterId = '';

  let checkState: 'idle' | 'checking' | 'done' = 'idle';
  let checkResult: VersionInfo | null = null;

  $: pincodeValid = printer.pincode === '' || /^[A-Za-z0-9]{6}$/.test(printer.pincode);
  $: profilesValid = printers.every((p) => p.pincode === '' || /^[A-Za-z0-9]{6}$/.test(p.pincode));

  function updatePincode(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    printer = {
      ...printer,
      pincode: input.value.replace(/[^A-Za-z0-9]/g, '').slice(0, 6),
    };
  }

  function makeProfileId(ip: string, printerId: string): string {
    return printerId.trim() ? `cc2-${printerId.trim()}` : `cc2-${ip.trim().replace(/\./g, '-')}`;
  }

  function addCurrentPrinter() {
    if (!printer.ip.trim()) return;
    const id = activePrinterId || makeProfileId(printer.ip, printer.printer_id);
    const profile = {
      id,
      label: printer.printer_id ? `CC2 ${printer.printer_id}` : `CC2 ${printer.ip}`,
      ip: printer.ip.trim(),
      printer_id: printer.printer_id.trim(),
      pincode: printer.pincode.trim(),
    };
    printers = [...printers.filter((p) => p.id !== id), profile];
    activePrinterId = id;
  }

  function selectProfile(profile: PrinterProfile) {
    activePrinterId = profile.id;
    printer = { ip: profile.ip, printer_id: profile.printer_id, pincode: profile.pincode };
  }

  function removeProfile(id: string) {
    printers = printers.filter((p) => p.id !== id);
    if (activePrinterId === id) {
      const next = printers[0];
      activePrinterId = next?.id ?? '';
      if (next) selectProfile(next);
    }
  }

  function updateProfile(id: string, key: keyof PrinterProfile, value: string) {
    const normalized = key === 'pincode' ? value.replace(/[^A-Za-z0-9]/g, '').slice(0, 6) : value;
    printers = printers.map((p) => p.id === id ? { ...p, [key]: normalized } : p);
    const active = printers.find((p) => p.id === activePrinterId);
    if (active) selectProfile(active);
  }

  function inputValue(e: Event): string {
    return (e.currentTarget as HTMLInputElement).value;
  }

  async function doCheckUpdate() {
    checkState = 'checking';
    checkResult = null;
    try {
      checkResult = await checkForUpdates();
    } catch {
      checkResult = null;
    }
    checkState = 'done';
  }
</script>

<div class="group">
  <div class="row">
    <div class="row-label">
      <div class="row-title">Updates</div>
      <div class="row-sub">Check if a newer version is available on GitHub.</div>
    </div>
    <div class="update-area">
      <button class="btn sm" on:click={doCheckUpdate} disabled={checkState === 'checking'}>
        {checkState === 'checking' ? 'Checking…' : 'Check for updates'}
      </button>
      {#if checkState === 'done'}
        {#if checkResult && !checkResult.up_to_date}
          <a class="check-link" href="https://github.com/CrazyFire66/cc2-openwebui/releases" target="_blank" rel="noopener">v{checkResult.latest_version} available →</a>
        {:else if checkResult}
          <span class="check-ok">Up to date (v{checkResult.current_version})</span>
        {:else}
          <span class="check-err">Check failed</span>
        {/if}
      {/if}
    </div>
  </div>
</div>

<div class="group">
  <div class="row">
    <div class="row-label">
      <div class="row-title">IP Address</div>
      <div class="row-sub">Found on your printer's network settings screen.</div>
    </div>
    <input id="ip" class="input mono row-input" type="text" bind:value={printer.ip} placeholder="192.168.1.100" />
  </div>
  <div class="row">
    <div class="row-label">
      <div class="row-title">Pincode</div>
      <div class="row-sub" class:row-sub-warn={!pincodeValid}>
        {pincodeValid ? 'Optional 6-character access code from the printer screen.' : 'Use exactly 6 letters or numbers.'}
      </div>
    </div>
    <input
      id="pin"
      class="input mono row-input short"
      class:invalid={!pincodeValid}
      type="text"
      value={printer.pincode}
      on:input={updatePincode}
      placeholder="Optional"
      maxlength="6"
      autocomplete="off"
      spellcheck="false"
    />
  </div>
</div>

<div class="group">
  <div class="profiles-head">
    <div>
      <div class="row-title">Printer profiles</div>
      <div class="row-sub" class:row-sub-warn={!profilesValid}>Save multiple printers and choose which one this Web UI connects to.</div>
    </div>
    <button class="btn sm" on:click={addCurrentPrinter} disabled={!printer.ip.trim() || !pincodeValid}>Save current</button>
  </div>
  {#if printers.length === 0}
    <div class="empty-profiles">No saved printers yet.</div>
  {:else}
    <div class="profiles">
      {#each printers as profile}
        <div class="profile-row" class:active={profile.id === activePrinterId}>
          <button class="profile-select" on:click={() => selectProfile(profile)} title="Use this printer">
            <span class="status-dot"></span>
          </button>
          <input
            class="input profile-label"
            value={profile.label}
            on:input={(e) => updateProfile(profile.id, 'label', inputValue(e))}
            placeholder="Printer name"
          />
          <input
            class="input mono profile-ip"
            value={profile.ip}
            on:input={(e) => updateProfile(profile.id, 'ip', inputValue(e))}
            placeholder="192.168.150.100"
          />
          <input
            class="input mono profile-pin"
            class:invalid={!!profile.pincode && !/^[A-Za-z0-9]{6}$/.test(profile.pincode)}
            value={profile.pincode}
            on:input={(e) => updateProfile(profile.id, 'pincode', inputValue(e))}
            placeholder="PIN"
            maxlength="6"
            autocomplete="off"
            spellcheck="false"
          />
          <button class="icon-btn" on:click={() => removeProfile(profile.id)} title="Remove printer profile">
            <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path d="M3 4h10M6.5 4V2.5h3V4M5 4l.5 9h5l.5-9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .group {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    overflow: hidden;
  }
  .row {
    display: grid;
    grid-template-columns: 1fr 220px;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
  }
  .row:first-child { border-top: none; }
  .row-label { min-width: 0; }
  .row-title { font-size: 12.5px; font-weight: 500; color: var(--text); }
  .row-sub { font-size: 11.5px; color: var(--muted); margin-top: 2px; line-height: 1.45; }
  .row-sub-warn { color: var(--danger); }
  .row-input { width: 100%; }
  .row-input.short { max-width: 140px; justify-self: end; }
  .invalid { border-color: var(--danger); box-shadow: 0 0 0 3px var(--danger-dim); }

  .profiles-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  .empty-profiles { padding: 14px 16px; color: var(--muted); font-size: 12px; }
  .profiles { display: flex; flex-direction: column; }
  .profile-row {
    display: grid;
    grid-template-columns: 30px minmax(120px, 1fr) 150px 90px 30px;
    gap: 8px;
    align-items: center;
    padding: 10px 12px;
    border-top: 1px solid var(--border);
  }
  .profile-row:first-child { border-top: none; }
  .profile-row.active { background: var(--accent-dim); }
  .profile-select, .icon-btn {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    background: var(--surface2);
    color: var(--muted);
    border-radius: 7px;
  }
  .profile-select:hover, .icon-btn:hover { border-color: var(--border2); color: var(--text); }
  .profile-row.active .profile-select { border-color: rgba(45,135,240,0.45); color: var(--accent); }
  .status-dot { width: 8px; height: 8px; border-radius: 50%; background: currentColor; opacity: 0.45; }
  .profile-row.active .status-dot { opacity: 1; box-shadow: 0 0 0 3px var(--accent-dim); }
  .profile-label, .profile-ip, .profile-pin { min-width: 0; }

  .update-area { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .check-ok { font-size: 11.5px; color: var(--success, #4caf50); }
  .check-err { font-size: 11.5px; color: var(--danger); }
  .check-link { font-size: 11.5px; color: var(--accent); text-decoration: none; }
  .check-link:hover { text-decoration: underline; }

  @media (max-width: 700px) {
    .row { grid-template-columns: 1fr; gap: 8px; }
    .row-input.short { max-width: none; justify-self: stretch; }
    .profiles-head { align-items: flex-start; flex-direction: column; }
    .profile-row { grid-template-columns: 30px 1fr 30px; }
    .profile-ip, .profile-pin { grid-column: 2 / 3; }
  }
</style>
