// ── Canonical Application Config State ─────────────────────────────────────────

export const DEFAULT_CONFIG = {
    theme_settings: {},
    rotation_enabled: false,
    rotation_interval_min: 60,
    pause_on_battery: false,
    onboarding_seen: false,
    target_fps: 30
};

let currentConfig = { ...DEFAULT_CONFIG };

export function getConfig() {
    return currentConfig;
}

export function setConfig(newConfig) {
    currentConfig = { ...DEFAULT_CONFIG, ...newConfig };
    return currentConfig;
}
