<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settingsStore";
import * as api from "@/lib/backend/api";
import { Button } from "@/components/ui/button";
const settings = useSettingsStore();
const { t } = useI18n();
const key = ref("");
const busy = ref(false);
const message = ref("");
const failed = ref(false);
async function run(action: () => Promise<void>) {
  busy.value = true;
  message.value = "";
  failed.value = false;
  try {
    await action();
  } catch (error) {
    failed.value = true;
    message.value = t(String(error).includes("[auth]") ? "enterpriseAi.authFailed" : "enterpriseAi.operationFailed");
  } finally {
    busy.value = false;
  }
}
async function saveKey() {
  await run(async () => {
    await settings.saveEnterpriseKey(key.value.trim());
    key.value = "";
    message.value = t("enterpriseAi.saved");
  });
}
async function clearKey() {
  await run(async () => {
    await settings.saveEnterpriseKey("");
    key.value = "";
  });
}
async function changeModel(event: Event) {
  const model = (event.target as HTMLSelectElement).value;
  await run(async () => {
    await settings.saveEnterpriseModel(model);
  });
}
async function testConnection() {
  await run(async () => {
    const config = settings.aiConfigs[0];
    if (!config || !settings.activeModel) return;
    const result = await api.aiTestConnection({ ...config, model: settings.activeModel.modelId });
    failed.value = !result.success;
    message.value = result.success ? t("enterpriseAi.connected") : t(result.errorCategory === "auth" ? "enterpriseAi.authFailed" : "enterpriseAi.connectionFailed");
  });
}
</script>
<template>
  <div v-if="settings.enterpriseAiState" class="space-y-4" data-enterprise-ai-settings>
    <h3 class="text-sm font-medium">{{ settings.enterpriseAiState.serviceName }}</h3>
    <p class="text-xs text-muted-foreground">{{ t("enterpriseAi.description") }}</p>
    <label class="block space-y-2 text-xs">
      <span>{{ t("enterpriseAi.personalKey") }}</span>
      <input v-model="key" type="password" autocomplete="new-password" spellcheck="false" class="w-full rounded-md border bg-background px-3 py-2" :disabled="busy" />
    </label>
    <p class="text-xs text-muted-foreground" role="status">{{ t(settings.enterpriseAiState.hasApiKey ? "enterpriseAi.keyConfigured" : "enterpriseAi.keyRequired") }}</p>
    <label class="block space-y-2 text-xs">
      <span>{{ t("enterpriseAi.model") }}</span>
      <select :value="settings.activeModel?.modelId" :disabled="busy" class="w-full rounded-md border bg-background px-3 py-2" @change="changeModel">
        <option v-for="model in settings.enterpriseAiState.models" :key="model" :value="model">{{ model }}</option>
      </select>
    </label>
    <div class="flex flex-wrap gap-2">
      <Button size="sm" :disabled="busy || !key.trim()" @click="saveKey">{{ t("enterpriseAi.saveKey") }}</Button>
      <Button size="sm" variant="outline" :disabled="busy || !settings.enterpriseAiState.hasApiKey" @click="clearKey">{{ t("enterpriseAi.clearKey") }}</Button>
      <Button size="sm" variant="outline" :disabled="busy || !settings.enterpriseAiState.hasApiKey" @click="testConnection">{{ t("enterpriseAi.test") }}</Button>
    </div>
    <p v-if="message" role="status" class="text-xs" :class="failed ? 'text-destructive' : 'text-muted-foreground'">{{ message }}</p>
  </div>
</template>
