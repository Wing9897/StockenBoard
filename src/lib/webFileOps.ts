/**
 * Browser-native file operation handlers for web mode.
 * These replace Tauri IPC commands when running outside a Tauri webview.
 */

import { pickFile, pickFileOrThrow } from './filePicker';
import { downloadBlob } from './blobDownload';

/**
 * Opens a browser file input for image files, reads the selected file as raw
 * bytes, and uploads to POST /api/icons/:symbol with octet-stream content type.
 * Resolves with undefined (no error) if the user cancels the picker.
 */
async function webSetIcon(args?: Record<string, unknown>): Promise<unknown> {
  const file = await pickFile({
    accept: 'image/png,image/jpeg,.png,.jpg,.jpeg,.webp,.svg',
  });

  if (!file) {
    return undefined;
  }

  const buffer = await file.arrayBuffer();
  const symbol = String(args?.symbol ?? '');

  const response = await fetch(`/api/icons/${encodeURIComponent(symbol)}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/octet-stream' },
    body: buffer,
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(text);
  }

  const envelope = await response.json();
  return envelope.data;
}

/**
 * Creates a Blob from content and triggers a browser download with the
 * specified filename. No server round-trip needed.
 */
export async function webExportFile(args?: Record<string, unknown>): Promise<void> {
  const filename = String(args?.filename ?? 'export.json');
  const content = String(args?.content ?? '');
  downloadBlob(filename, content, 'application/json');
}

/**
 * Opens a browser file input for .json files, reads the selected file as
 * UTF-8 text, and resolves with the content string.
 * Rejects with Error('Cancelled') if the user cancels.
 */
export async function webImportFile(): Promise<string> {
  const file = await pickFileOrThrow({ accept: '.json' });
  return file.text();
}

/**
 * Mapping of command names to their web mode handler functions.
 * Used by HttpTransport to intercept file operations in web mode.
 */
export const webModeHandlers: Record<string, (args?: Record<string, unknown>) => Promise<unknown>> = {
  set_icon: webSetIcon,
  export_file: webExportFile,
  import_file: webImportFile,
};
