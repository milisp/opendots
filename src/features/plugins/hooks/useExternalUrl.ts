import { useCallback } from 'react';
export function useExternalUrl() {
  const openExternalUrl = useCallback(async (url: string) => {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  }, []);
  return { openExternalUrl };
}
