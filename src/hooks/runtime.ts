import { getApiUrl } from '@/services/apiAdapt/shared';

export const isDesktopTauri = () => Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
export const isTauri = isDesktopTauri;
export const buildEventUrl = async (path: string) => `${await getApiUrl()}${path}`;
export const buildUrl = buildEventUrl;
export const authHeaders = () => ({});
