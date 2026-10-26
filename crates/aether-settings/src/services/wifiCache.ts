export interface WifiNetwork {
  ssid: string;
  signal: number;
  security: string;
  connected: boolean;
}

// Shared cache survives WifiPage unmounts during tab switching.
let cachedNetworks: WifiNetwork[] | null = null;
let cachedEnabled: boolean | null = null;
let scanPromise: Promise<WifiNetwork[]> | null = null;

export function getCachedNetworks() {
  return cachedNetworks;
}

export function getCachedWifiEnabled() {
  return cachedEnabled;
}

export function setCachedWifiEnabled(enabled: boolean) {
  cachedEnabled = enabled;

  if (!enabled) {
    cachedNetworks = [];
  }
}

export async function loadWifiNetworks(
  scan: () => Promise<WifiNetwork[]>,
  forceRefresh = false,
): Promise<WifiNetwork[]> {
  if (!forceRefresh && cachedNetworks !== null) {
    return cachedNetworks;
  }

  if (scanPromise) {
    return scanPromise;
  }

  scanPromise = scan()
    .then((networks) => {
      cachedNetworks = networks;
      return networks;
    })
    .finally(() => {
      scanPromise = null;
    });

  return scanPromise;
}