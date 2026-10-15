// ── Docked Settings Panel Locking Delegation ──────────────────────────────────
import { invoke } from '@tauri-apps/api/core';

export const POPUP_CONTROLS = 'select, input[type="color"]';

export function setPanelLocked(locked) {
    invoke('set_settings_panel_locked', { locked }).catch(() => {});
}

export function initPanelLockDelegation() {
    const matches = (t) => t instanceof Element && t.closest(POPUP_CONTROLS);
    let popupLocked = false;
    const lockPopup = () => {
        popupLocked = true;
        setPanelLocked(true);
    };
    const unlockPopup = () => {
        if (!popupLocked) return;
        popupLocked = false;
        setPanelLocked(false);
    };

    document.addEventListener('mousedown', (e) => {
        if (matches(e.target)) lockPopup();
        else unlockPopup();
    }, true);
    document.addEventListener('focusin',  (e) => { if (matches(e.target)) lockPopup(); });
    document.addEventListener('change',   (e) => { if (matches(e.target)) unlockPopup(); });
    document.addEventListener('keydown',  (e) => { if (e.key === 'Escape') unlockPopup(); }, true);
    document.addEventListener('focusout', (e) => {
        if (!matches(e.target)) return;
        setTimeout(() => {
            if (document.hasFocus() && !matches(document.activeElement)) {
                unlockPopup();
            }
        }, 100);
    });
}
