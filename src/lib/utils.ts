import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Send a desktop notification
 * @param title - Notification title
 * @param body - Notification body text
 * @returns Promise that resolves when notification is sent
 */
export async function notify(title: string, body: string): Promise<void> {
  let permissionGranted = await isPermissionGranted()

  if (!permissionGranted) {
    const permission = await requestPermission()
    permissionGranted = permission === "granted"
  }

  if (permissionGranted) {
    await sendNotification({ title, body })
  } else {
    console.warn("Notification permission not granted")
  }
}
