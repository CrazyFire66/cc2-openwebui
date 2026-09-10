import { writable } from 'svelte/store';

export type Language = 'en' | 'de';

const KEY = 'cc2_language';

export const language = writable<Language>(readLanguage());

export function setLanguage(lang: Language) {
  language.set(lang);
  if (typeof localStorage !== 'undefined') localStorage.setItem(KEY, lang);
  if (typeof document !== 'undefined') document.documentElement.lang = lang;
}

export function initLanguage() {
  setLanguage(readLanguage());
}

function readLanguage(): Language {
  if (typeof localStorage === 'undefined') return 'en';
  return localStorage.getItem(KEY) === 'de' ? 'de' : 'en';
}

const de: Record<string, string> = {
  'Settings': 'Einstellungen',
  'Close settings': 'Einstellungen schließen',
  'Settings sections': 'Einstellungsbereiche',
  'Setup': 'Einrichtung',
  'Features': 'Funktionen',
  'Diagnostics': 'Diagnose',
  'Advanced': 'Erweitert',
  'Printer': 'Drucker',
  'Connection': 'Verbindung',
  'AI Detection': 'AI-Erkennung',
  'Failure alerts': 'Fehlerwarnungen',
  'Notifications': 'Benachrichtigungen',
  'ntfy, Discord, Telegram': 'ntfy, Discord, Telegram',
  'Debug': 'Debug',
  'Runtime status': 'Laufzeitstatus',
  'Activity Logs': 'Aktivitätsprotokoll',
  'Events & errors': 'Ereignisse & Fehler',
  'UI settings': 'UI-Einstellungen',
  'Modify UI': 'Oberfläche anpassen',
  'Danger Zone': 'Gefahrenbereich',
  'Reset everything': 'Alles zurücksetzen',
  'Printer Connection': 'Druckerverbindung',
  'The IP your CC2 exposes on the LAN.': 'Die IP, unter der dein CC2 im LAN erreichbar ist.',
  'AI Failure Detection': 'AI-Fehlererkennung',
  'AI failure detection with Obico ML container.': 'AI-Fehlererkennung mit Obico-ML-Container.',
  'Push events via ntfy or Discord webhook. Add and configure notification destinations.': 'Ereignisse per ntfy, Discord, Telegram oder Webhook senden.',
  'Push events via ntfy, Discord, Telegram, or webhook. Add and configure notification destinations.': 'Ereignisse per ntfy, Discord, Telegram oder Webhook senden und Ziele konfigurieren.',
  'Runtime status, scan diagnostics, and connection details.': 'Laufzeitstatus, Scan-Diagnose und Verbindungsdetails.',
  'Connection events, print jobs, detections, etc': 'Verbindungsereignisse, Druckjobs, Erkennungen usw.',
  'UI Settings': 'UI-Einstellungen',
  'Show or hide UI elements.': 'UI-Elemente ein- oder ausblenden.',
  'Irreversible actions. Make sure you know what you are doing.': 'Nicht rückgängig machbare Aktionen. Bitte bewusst verwenden.',
  'Close': 'Schließen',
  'Save changes': 'Änderungen speichern',
  'Saving…': 'Speichern…',
  'Changes saved': 'Änderungen gespeichert',
  'Detection preset': 'Erkennungsprofil',
  'Choose a sensitivity profile, then fine tune the sliders below.': 'Wähle ein Empfindlichkeitsprofil und stelle die Regler danach fein ein.',
  'Careful': 'Vorsichtig',
  'Balanced': 'Ausgewogen',
  'Strict': 'Streng',
  'Spaghetti risk': 'Spaghetti-Risiko',
  'Obico ML scores tangled extrusion and failed print patterns.': 'Obico ML bewertet verhedderte Extrusion und Fehlermuster im Druckbild.',
  'Object boxes': 'Objekt-Boxen',
  'Detected objects/blobs are stored as boxes with confidence and optional labels.': 'Erkannte Objekte oder Klumpen werden als Boxen mit Konfidenz und optionalem Label gespeichert.',
  'Camera health': 'Kamera-Status',
  'Camera loss and restore events can trigger notifications.': 'Kameraausfall und Wiederherstellung können Benachrichtigungen auslösen.',
  'Confirmed frames': 'Bestätigte Frames',
  'Multiple bad frames are required before alerting or auto-pausing.': 'Mehrere schlechte Frames sind nötig, bevor gewarnt oder automatisch pausiert wird.',
  'Ignore zones': 'Ignorierte Zonen',
  'Marked areas are excluded from score decisions.': 'Markierte Bereiche werden bei Score-Entscheidungen ausgeschlossen.',
  'Auto protection': 'Automatischer Schutz',
  'High rolling risk can pause the printer and send snapshots.': 'Ein dauerhaft hohes Risiko kann den Druck pausieren und Snapshots senden.',
  'Obico ML URL': 'Obico-ML-URL',
  'Endpoint of the local Obico detection container.': 'Endpoint des lokalen Obico-Erkennungscontainers.',
  'Notify threshold': 'Warnschwelle',
  'Score above which a push notification is sent.': 'Score, ab dem eine Push-Benachrichtigung gesendet wird.',
  'Pause threshold': 'Pausenschwelle',
  'Score above which the print is automatically paused.': 'Score, ab dem der Druck automatisch pausiert wird.',
  'Pause threshold must be ≥ notify threshold.': 'Die Pausenschwelle muss größer oder gleich der Warnschwelle sein.',
  'Check interval': 'Prüfintervall',
  'Seconds between frame analyses.': 'Sekunden zwischen Bildanalysen.',
  'seconds': 'Sekunden',
  'Test detection': 'Erkennung testen',
  'Run the ML model on the current camera frame and see the result.': 'Führt das ML-Modell auf dem aktuellen Kamerabild aus und zeigt das Ergebnis.',
  'Run test': 'Test starten',
  'Detection snapshots': 'Erkennungs-Snapshots',
  'Loading…': 'Lädt…',
  'No detection snapshots yet.': 'Noch keine Erkennungs-Snapshots.',
  'Export JSON': 'JSON exportieren',
  'Import JSON': 'JSON importieren',
  'Backup and restore': 'Backup und Wiederherstellung',
  'Export or import printer profiles, detection settings, server config, and notification destinations.': 'Druckerprofile, Erkennungseinstellungen, Serverkonfiguration und Benachrichtigungsziele exportieren oder importieren.',
  'Reset all settings': 'Alle Einstellungen zurücksetzen',
  'Clears printer connection, detection config, notifications, and returns to the onboarding flow. Cannot be undone.': 'Löscht Druckerverbindung, Erkennungskonfiguration und Benachrichtigungen und startet die Einrichtung neu. Nicht rückgängig machbar.',
  'Printer IP, pincode and all config wiped.': 'Drucker-IP, PIN und gesamte Konfiguration werden gelöscht.',
  'All detection history, events and snapshots deleted.': 'Erkennungshistorie, Ereignisse und Snapshots werden gelöscht.',
  'Notification destinations cleared.': 'Benachrichtigungsziele werden gelöscht.',
  'Active MQTT connection closed.': 'Aktive MQTT-Verbindung wird geschlossen.',
  'Add destination': 'Ziel hinzufügen',
  'ntfy, Discord, Telegram, or generic webhook.': 'ntfy, Discord, Telegram oder generischer Webhook.',
  'No destinations configured. Add one below.': 'Keine Ziele konfiguriert. Füge unten eines hinzu.',
  'Loading destinations…': 'Ziele werden geladen…',
  'Type': 'Typ',
  'Label': 'Name',
  'Server': 'Server',
  'Topic': 'Topic',
  'Tap URL': 'Öffnungs-URL',
  'Webhook URL': 'Webhook-URL',
  'URL': 'URL',
  'Bot token': 'Bot-Token',
  'Chat ID': 'Chat-ID',
  'Topic ID': 'Topic-ID',
  'Progress': 'Fortschritt',
  'Off': 'Aus',
  'Every 5%': 'Alle 5%',
  'Every 10%': 'Alle 10%',
  'Every 25%': 'Alle 25%',
  'Every 50%': 'Alle 50%',
  'Attach camera snapshot to failure messages': 'Kamera-Snapshot an Fehlermeldungen anhängen',
  'Print started': 'Druck gestartet',
  'Print finished': 'Druck beendet',
  'Print paused': 'Druck pausiert',
  'Print resumed': 'Druck fortgesetzt',
  'Print stopped': 'Druck gestoppt',
  'Progress updates': 'Fortschrittsmeldungen',
  'Failure risk': 'Fehlerrisiko',
  'Failure confirmed': 'Fehler bestätigt',
  'Auto-paused': 'Automatisch pausiert',
  'Camera lost': 'Kamera verloren',
  'Camera restored': 'Kamera wiederhergestellt',
  'Detection unavailable': 'Erkennung nicht verfügbar',
  'Printer disconnected': 'Drucker getrennt',
  'Printer connected': 'Drucker verbunden',
  'Emergency stop': 'Not-Aus',
  'Printer error': 'Druckerfehler',
  'ID not match': 'ID stimmt nicht überein',
  'Auth error': 'Authentifizierungsfehler',
  'Edit': 'Bearbeiten',
  'Delete': 'Löschen',
  'Confirm': 'Bestätigen',
  'Cancel': 'Abbrechen',
  'Save': 'Speichern',
  'Test': 'Test',
  'Sending…': 'Senden…',
  'Sent!': 'Gesendet!',
  'Language': 'Sprache',
  'Interface language': 'Sprache der Oberfläche',
  'English': 'Englisch',
  'German': 'Deutsch',
  'Show Job info': 'Job-Info anzeigen',
  'Show Controls': 'Steuerung anzeigen',
  'Show DetectionPanel': 'Erkennung anzeigen',
  'Show FileList': 'Dateiliste anzeigen',
  'Show Camera': 'Kamera anzeigen',
  'Show TempPanel': 'Temperaturen anzeigen',
  'Show CanvasPanel': 'Canvas anzeigen',
  'Refresh': 'Aktualisieren',
  'Run LAN scan': 'LAN-Scan starten',
  'Version': 'Version',
  'Configured': 'Konfiguriert',
  'Yes': 'Ja',
  'No': 'Nein',
  'Active profile': 'Aktives Profil',
  'Profiles': 'Profile',
  'Printer IP': 'Drucker-IP',
  'Printer ID': 'Drucker-ID',
  'MQTT raw': 'MQTT raw',
  'MQTT ws': 'MQTT WS',
  'Connected': 'Verbunden',
  'Disconnected': 'Getrennt',
  'No active print': 'Kein aktiver Druck',
  'Idle': 'Bereit',
  'No print job active': 'Kein Druckjob aktiv',
  'CONTROL': 'STEUERUNG',
  'Control': 'Steuerung',
  'Home All': 'Alle Achsen homen',
  'Home X': 'X homen',
  'Home Y': 'Y homen',
  'Home Z': 'Z homen',
  'Print Speed': 'Druckgeschwindigkeit',
  'Silent': 'Leise',
  'Sport': 'Sport',
  'Ludicrous': 'Extrem',
  'Model': 'Modell',
  'Assist': 'Hilfslüfter',
  'Case': 'Gehäuse',
  'Chamber Light': 'Bauraumlicht',
  'FAILURE DETECTION': 'FEHLERERKENNUNG',
  'Failure Detection': 'Fehlererkennung',
  'Normal': 'Normal',
  'notify': 'Warnung',
  'pause': 'Pause',
  'Printer not printing': 'Drucker druckt nicht',
  'Test Detection': 'Erkennung testen',
  'History': 'Historie',
  'FILES': 'DATEIEN',
  'Files': 'Dateien',
  'Upload': 'Hochladen',
  'Upload .gcode file to printer': '.gcode-Datei zum Drucker hochladen',
  'CAMERA': 'KAMERA',
  'Camera': 'Kamera',
  'LIVE': 'LIVE',
  'Edit Zones': 'Zonen bearbeiten',
  'Camera stream': 'Kamerastream',
  'Fullscreen': 'Vollbild',
  'TEMPERATURE': 'TEMPERATUR',
  'Temperature': 'Temperatur',
  'COMPONENT': 'KOMPONENTE',
  'CURRENT': 'AKTUELL',
  'TARGET': 'ZIEL',
  'SENSITIVE': 'EMPFINDLICH',
  'Nozzle': 'Düse',
  'Heated Bed': 'Heizbett',
  'Chamber': 'Bauraum',
  'Refresh canvas data from printer': 'Canvas-Daten vom Drucker aktualisieren',
  'LOADED': 'GELADEN',
  'TYPE': 'TYP',
  'FAMILY': 'FAMILIE',
  'STATE': 'STATUS',
  'Loaded': 'Geladen',
  'Load': 'Laden',
  'Unload': 'Entladen',
  'Auto Refill': 'Automatisch nachfüllen',
  'Automatically switch to next spool when current runs out': 'Automatisch zur nächsten Spule wechseln, wenn die aktuelle leer ist',
  'Subnets': 'Subnetze',
  'AI detection': 'AI-Erkennung',
  'Enabled': 'Aktiviert',
  'Disabled': 'Deaktiviert',
  'Obico URL': 'Obico-URL',
  'Update on GitHub': 'Update auf GitHub',
  'View on GitHub →': 'Auf GitHub ansehen →',
  'A new version is available': 'Eine neue Version ist verfügbar',
  'Dismiss update notice': 'Update-Hinweis ausblenden',
};

const originals = new WeakMap<Node, string>();
const attrOriginals = new WeakMap<Element, Map<string, string>>();

export function t(text: string, lang: Language): string {
  return lang === 'de' ? de[text] ?? text : text;
}

export function localize(node: HTMLElement, lang: Language) {
  let current = lang;
  const apply = () => translateTree(node, current);
  const observer = new MutationObserver(() => apply());
  apply();
  observer.observe(node, { childList: true, characterData: true, subtree: true });

  return {
    update(lang: Language) {
      current = lang;
      if (typeof document !== 'undefined') document.documentElement.lang = lang;
      apply();
    },
    destroy() {
      observer.disconnect();
    },
  };
}

function translateTree(root: HTMLElement, lang: Language) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const parent = node.parentElement;
      if (!parent) return NodeFilter.FILTER_REJECT;
      const tag = parent.tagName.toLowerCase();
      if (['script', 'style', 'textarea', 'code'].includes(tag)) return NodeFilter.FILTER_REJECT;
      if (!node.textContent?.trim()) return NodeFilter.FILTER_REJECT;
      return NodeFilter.FILTER_ACCEPT;
    },
  });

  const texts: Text[] = [];
  while (walker.nextNode()) texts.push(walker.currentNode as Text);
  for (const textNode of texts) translateTextNode(textNode, lang);

  const elements = root.querySelectorAll<HTMLElement>('[placeholder], [title], [aria-label]');
  for (const el of elements) {
    translateAttribute(el, 'placeholder', lang);
    translateAttribute(el, 'title', lang);
    translateAttribute(el, 'aria-label', lang);
  }
}

function translateTextNode(node: Text, lang: Language) {
  const current = node.textContent ?? '';
  const known = originals.get(node);
  if (known == null) {
    originals.set(node, current);
  } else if (current !== known && current !== t(known, 'de')) {
    originals.set(node, current);
  }
  const original = originals.get(node) ?? '';
  const trimmed = original.trim();
  if (!trimmed) return;
  const translated = t(trimmed, lang);
  const leading = original.match(/^\s*/)?.[0] ?? '';
  const trailing = original.match(/\s*$/)?.[0] ?? '';
  const next = `${leading}${translated}${trailing}`;
  if (node.textContent !== next) node.textContent = next;
}

function translateAttribute(el: Element, attr: string, lang: Language) {
  const value = el.getAttribute(attr);
  if (value == null) return;
  let map = attrOriginals.get(el);
  if (!map) {
    map = new Map();
    attrOriginals.set(el, map);
  }
  if (!map.has(attr)) map.set(attr, value);
  const original = map.get(attr) ?? value;
  el.setAttribute(attr, t(original, lang));
}
