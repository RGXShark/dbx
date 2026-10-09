// @vitest-environment happy-dom
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useSettingsStore } from "../settingsStore";
const api = vi.hoisted(() => ({
  getEnterpriseAiState: vi.fn(),
  loadAiConfigs: vi.fn(),
  loadAiChatSelection: vi.fn(),
  saveEnterpriseAiKey: vi.fn(),
  saveEnterpriseAiModel: vi.fn(),
  saveAiConfigItem: vi.fn(),
  loadAiConfig: vi.fn(),
  loadAiProviderConfigs: vi.fn(),
  saveAiChatSelection: vi.fn(),
}));
vi.mock("@/lib/backend/api", () => api);
const policy = { schemaVersion: 1 as const, serviceName: "Company AI", endpoint: "https://company.example/v1", models: ["a", "b"], defaultModel: "a", selectedModel: "a", hasApiKey: false };
beforeEach(() => {
  vi.resetAllMocks();
  setActivePinia(createPinia());
  localStorage.clear();
  api.getEnterpriseAiState.mockResolvedValue({ ...policy });
  api.loadAiConfigs.mockResolvedValue([{ id: "__enterprise_ai__", name: "Company AI", isDefault: true, provider: "openai-compatible", apiKey: "", authMethod: "bearer", endpoint: policy.endpoint, model: "a", apiStyle: "completions", models: [{ name: "a" }, { name: "b" }] }]);
  api.saveAiChatSelection.mockResolvedValue(undefined);
  api.saveEnterpriseAiKey.mockResolvedValue(undefined);
  api.saveEnterpriseAiModel.mockResolvedValue(undefined);
  api.loadAiChatSelection.mockResolvedValue({ active: { configId: "old-provider", modelId: "old-model" }, effortPreferences: [] });
});
describe("desktop enterprise AI state", () => {
  it("uses the compiled policy and requires a fresh personal key", async () => {
    const store = useSettingsStore();
    await store.initAiConfigs();
    expect(store.isEnterpriseAi).toBe(true);
    expect(store.aiPolicyReady).toBe(true);
    expect(store.activeModel).toEqual({ configId: "__enterprise_ai__", modelId: "a" });
    expect(store.isConfigured).toBe(false);
    expect(api.loadAiConfigs).not.toHaveBeenCalled();
    expect(api.loadAiConfig).not.toHaveBeenCalled();
    expect(api.loadAiProviderConfigs).not.toHaveBeenCalled();
  });
  it("uses key presence without exposing the key to the store", async () => {
    api.getEnterpriseAiState.mockResolvedValue({ ...policy, hasApiKey: true });
    const store = useSettingsStore();
    await store.initAiConfigs();
    expect(store.isConfigured).toBe(true);
    expect(store.aiConfigs[0].apiKey).toBe("");
    await store.saveEnterpriseModel("b");
    expect(store.activeModel?.modelId).toBe("b");
    expect(api.saveEnterpriseAiModel).toHaveBeenCalledWith("b");
    expect(api.saveEnterpriseAiKey).not.toHaveBeenCalled();
  });
  it("saves/replaces and clears the key only through the dedicated backend", async () => {
    const store = useSettingsStore();
    await store.initAiConfigs();
    api.getEnterpriseAiState.mockResolvedValue({ ...policy, hasApiKey: true });
    await store.saveEnterpriseKey("employee-key");
    expect(store.isConfigured).toBe(true);
    api.getEnterpriseAiState.mockResolvedValue({ ...policy, hasApiKey: false });
    await store.saveEnterpriseKey("");
    expect(store.isConfigured).toBe(false);
    expect(api.saveEnterpriseAiKey.mock.calls).toEqual([["employee-key"], [""]]);
    expect(JSON.stringify(store.aiConfigs)).not.toContain("employee-key");
  });
  it("blocks legacy editing and ignores non-policy selections", async () => {
    const store = useSettingsStore();
    await store.initAiConfigs();
    await expect(store.updateAiConfigItem("__enterprise_ai__", { endpoint: "https://evil.example" })).rejects.toThrow("LOCKED");
    store.updateActiveModel({ configId: "old", modelId: "b" });
    store.updateActiveModel({ configId: "__enterprise_ai__", modelId: "other" });
    expect(api.saveAiConfigItem).not.toHaveBeenCalled();
    expect(api.saveEnterpriseAiModel).not.toHaveBeenCalled();
  });
  it("fails closed and permits retry when policy metadata cannot load", async () => {
    api.getEnterpriseAiState.mockRejectedValueOnce(new Error("unavailable"));
    const store = useSettingsStore();
    await expect(store.initAiConfigs()).rejects.toThrow("unavailable");
    expect(store.aiPolicyReady).toBe(false);
    expect(store.enterpriseAiLoadFailed).toBe(true);
    expect(store.isConfigured).toBe(false);
    expect(api.loadAiConfigs).not.toHaveBeenCalled();
    await store.initAiConfigs();
    expect(store.aiPolicyReady).toBe(true);
  });
});
