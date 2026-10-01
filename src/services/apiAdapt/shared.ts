import { toast } from '@/components/ui/use-toast';

let apiUrl: Promise<string> | undefined;

// Must match DEFAULT_API_PORT in src-tauri/src/api.rs.
const DEFAULT_API_URL = 'http://127.0.0.1:26929';

// In Tauri, ask the backend for the real port (it may be overridden or fall back);
// in a plain browser (dev), use the default.
export function getApiUrl() {
  apiUrl ??= isDesktopTauri()
    ? import('@tauri-apps/api/core')
        .then(({ invoke }) => invoke<number>('api_port'))
        .then((port) => `http://127.0.0.1:${port}`)
    : Promise.resolve(DEFAULT_API_URL);
  return apiUrl;
}

async function request<T>(path: string, method: 'GET' | 'POST', body?: unknown): Promise<T> {
  const response = await fetch(`${await getApiUrl()}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!response.ok) {
    const message = await response.text();
    toast({ title: 'Request failed', description: message || `HTTP ${response.status}`, variant: 'destructive' });
    throw new Error(message || `HTTP ${response.status}`);
  }
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export function getJson<T>(path: string): Promise<T> { return request<T>(path, 'GET'); }
export function postJson<T>(path: string, body?: unknown): Promise<T> { return request<T>(path, 'POST', body); }
export function postJsonWithOptions<T>(path: string, body?: unknown, _options?: { suppressToast?: boolean }): Promise<T> { return request<T>(path, 'POST', body); }
export async function postNoContent(path: string, body?: unknown): Promise<void> { await request<unknown>(path, 'POST', body); }
export const isDesktopTauri = () => Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
export const isTauri = isDesktopTauri;
export const dual = <T>(_command: string, _args: Record<string, unknown> | undefined, path: string, body?: unknown) => postJson<T>(path, body);
export const dualGet = <T>(_command: string, _args: Record<string, unknown> | undefined, path: string) => getJson<T>(path);
export const dualVoid = async (_command: string, _args: Record<string, unknown> | undefined, path: string, body?: unknown) => postNoContent(path, body);
