import { ref } from "vue";
import { emit, listen } from "@tauri-apps/api/event";

// Estado global singleton — compartilhado entre SettingsWindow e useSpellCoach
const wardAlertsEnabled  = ref(true);
const skillTipsEnabled   = ref(true);

const SETTINGS_EVENT = "coach-settings-changed";

function loadSettings() {
  try {
    const ward = localStorage.getItem("spell_coach_ward_alerts_enabled");
    if (ward !== null) wardAlertsEnabled.value = ward === "true";

    const skill = localStorage.getItem("spell_coach_skill_tips_enabled");
    if (skill !== null) skillTipsEnabled.value = skill === "true";
  } catch (_) {}
}

async function saveSettings() {
  try {
    localStorage.setItem("spell_coach_ward_alerts_enabled",  String(wardAlertsEnabled.value));
    localStorage.setItem("spell_coach_skill_tips_enabled",   String(skillTipsEnabled.value));
    await emit(SETTINGS_EVENT, {
      wardAlertsEnabled: wardAlertsEnabled.value,
      skillTipsEnabled:  skillTipsEnabled.value,
    });
  } catch (_) {}
}

// Recebe mudanças feitas em outras webviews (ex: SettingsWindow → main)
listen(SETTINGS_EVENT, (event: any) => {
  wardAlertsEnabled.value = Boolean(event.payload?.wardAlertsEnabled);
  skillTipsEnabled.value  = Boolean(event.payload?.skillTipsEnabled);
}).catch(() => {});

loadSettings();

export function useCoachSettings() {
  return {
    wardAlertsEnabled,
    skillTipsEnabled,
    saveSettings,
  };
}
