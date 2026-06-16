/**
 * Browser file picker utility.
 * Wraps <input type="file"> in a Promise for use in web mode file operations.
 */

interface FilePickerOptions {
  accept: string;
  multiple?: boolean;
}

/**
 * Shows a browser file picker and resolves with the selected File, or null if
 * the user cancels. Used by set_icon which should not error on cancel.
 */
export function pickFile(options: FilePickerOptions): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = options.accept;
    if (options.multiple) {
      input.multiple = true;
    }
    input.style.display = 'none';
    document.body.appendChild(input);

    const cleanup = () => {
      input.remove();
    };

    input.addEventListener('change', () => {
      const file = input.files?.[0] ?? null;
      cleanup();
      resolve(file);
    });

    // Detect cancel: when the window regains focus after the picker closes
    // without a file being selected, the change event won't fire.
    const onFocus = () => {
      window.removeEventListener('focus', onFocus);
      // Use a timeout to let the change event fire first if a file was selected
      setTimeout(() => {
        if (input.parentNode) {
          cleanup();
          resolve(null);
        }
      }, 300);
    };
    window.addEventListener('focus', onFocus);

    input.click();
  });
}

/**
 * Shows a browser file picker. Rejects with Error('Cancelled') if the user
 * cancels without selecting a file. Used by import_file to match desktop behavior.
 */
export function pickFileOrThrow(options: FilePickerOptions): Promise<File> {
  return new Promise((resolve, reject) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = options.accept;
    if (options.multiple) {
      input.multiple = true;
    }
    input.style.display = 'none';
    document.body.appendChild(input);

    const cleanup = () => {
      input.remove();
    };

    input.addEventListener('change', () => {
      const file = input.files?.[0];
      cleanup();
      if (file) {
        resolve(file);
      } else {
        reject(new Error('Cancelled'));
      }
    });

    // Detect cancel via focus return
    const onFocus = () => {
      window.removeEventListener('focus', onFocus);
      setTimeout(() => {
        if (input.parentNode) {
          cleanup();
          reject(new Error('Cancelled'));
        }
      }, 300);
    };
    window.addEventListener('focus', onFocus);

    input.click();
  });
}
