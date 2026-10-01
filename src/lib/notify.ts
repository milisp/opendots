import { isDesktopTauri } from '@/hooks/runtime';

export async function notifyDesktop(title: string, body?: string, fallback?: () => void) {
  try {
    if (isDesktopTauri()) {
      const { isPermissionGranted, requestPermission, sendNotification } = await import('@tauri-apps/plugin-notification');
      let allowed = await isPermissionGranted();
      if (!allowed) allowed = (await requestPermission()) === 'granted';
      if (allowed) { sendNotification({ title, body }); return; }
    }
  } catch (error) { console.warn('Notification failed', error); }
  fallback?.();
}
