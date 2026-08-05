import { useEffect, useState } from 'react';

export function useKeyboardShortcut(
  key: string,
  callback: (e: KeyboardEvent) => void,
  modifiers: { ctrl?: boolean; alt?: boolean; shift?: boolean; meta?: boolean; primary?: boolean } = {}
) {
  useEffect(() => {
    const isMac = /Mac|iPod|iPhone|iPad/.test(navigator.platform || "") || /Mac/.test(navigator.userAgent || "");

    const handler = (event: KeyboardEvent) => {
      // Resolve "primary" modifier: Cmd on Mac, Ctrl on Win
      const reqCtrl = modifiers.primary ? !isMac : !!modifiers.ctrl;
      const reqMeta = modifiers.primary ? isMac : !!modifiers.meta;
      
      // Resolve "alt" modifier: For standardizing, if alt is requested, we use Option on Mac and Alt on Windows.
      const reqAlt = !!modifiers.alt;
      const reqShift = !!modifiers.shift;

      const targetCode = `Key${key.toUpperCase()}`;
      const keyMatches = 
        event.key.toLowerCase() === key.toLowerCase() || 
        event.code === targetCode;

      if (
        keyMatches &&
        reqCtrl === event.ctrlKey &&
        reqAlt === event.altKey &&
        reqShift === event.shiftKey &&
        reqMeta === event.metaKey
      ) {
        event.preventDefault();
        callback(event);
      }
    };

    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [key, callback, modifiers]);
}

// Helper hook to get display strings for shortcuts based on OS
export function useShortcutDisplay() {
  const [isMac, setIsMac] = useState(false);
  
  useEffect(() => {
    setIsMac(/Mac|iPod|iPhone|iPad/.test(navigator.platform || "") || /Mac/.test(navigator.userAgent || ""));
  }, []);

  return {
    isMac,
    primaryKey: isMac ? '⌘' : 'Ctrl',
    altKey: isMac ? '⌥' : 'Alt',
    shiftKey: isMac ? '⇧' : 'Shift',
    format: (shortcutDef: string) => {
      // shortcutDef might be "Alt+D" or "Primary+N"
      if (isMac) {
        return shortcutDef
          .replace(/Alt\+/i, 'Option + ')
          .replace(/Ctrl\+/i, 'Control + ')
          .replace(/Primary\+/i, '⌘ ')
          .replace(/Shift\+/i, 'Shift + ');
      } else {
        return shortcutDef
          .replace(/Primary\+/i, 'Ctrl + ')
          .replace(/Alt\+/i, 'Alt + ');
      }
    }
  };
}
