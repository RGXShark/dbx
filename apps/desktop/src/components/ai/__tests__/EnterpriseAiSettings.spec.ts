// @vitest-environment happy-dom
import { createApp, nextTick, reactive, type App } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n from "@/i18n";
import EnterpriseAiSettings from "../EnterpriseAiSettings.vue";
const mocks = vi.hoisted(() => ({ saveKey: vi.fn(), saveModel: vi.fn(), test: vi.fn() }));
const state = reactive({
  enterpriseAiState: { serviceName: "Company AI", models: ["a", "b"], hasApiKey: false },
  activeModel: { configId: "__enterprise_ai__", modelId: "a" },
  aiConfigs: [{ id: "__enterprise_ai__", provider: "openai-compatible", endpoint: "https://company.example/v1", apiKey: "", model: "a", apiStyle: "completions" }],
  saveEnterpriseKey: mocks.saveKey,
  saveEnterpriseModel: mocks.saveModel,
});
vi.mock("@/stores/settingsStore", () => ({ useSettingsStore: () => state }));
vi.mock("@/lib/backend/api", () => ({ aiTestConnection: mocks.test }));
let app: App;
let host: HTMLElement;
async function flush() {
  for (let i = 0; i < 8; i++) {
    await Promise.resolve();
    await nextTick();
  }
}
beforeEach(() => {
  vi.resetAllMocks();
  state.enterpriseAiState.hasApiKey = false;
  state.activeModel.modelId = "a";
  mocks.saveKey.mockImplementation(async (key: string) => {
    state.enterpriseAiState.hasApiKey = !!key;
  });
  mocks.saveModel.mockImplementation(async (model: string) => {
    state.activeModel.modelId = model;
  });
  host = document.createElement("div");
  document.body.append(host);
  app = createApp(EnterpriseAiSettings);
  app.use(i18n);
  app.mount(host);
});
afterEach(() => {
  app.unmount();
  host.remove();
});
describe("enterprise personal credential settings", () => {
  it("shows only a password input and the allowed models", () => {
    expect(host.querySelectorAll("input")).toHaveLength(1);
    expect(host.querySelector("input")?.type).toBe("password");
    expect([...host.querySelectorAll("option")].map((option) => option.value)).toEqual(["a", "b"]);
    expect(host.innerHTML).not.toContain("https://company.example");
    expect(host.textContent).toContain("IT");
  });
  it("clears the input after saving and never shows the stored key", async () => {
    const input = host.querySelector("input")!;
    input.value = "employee-key";
    input.dispatchEvent(new Event("input"));
    await nextTick();
    (host.querySelectorAll("button")[0] as HTMLButtonElement).click();
    await flush();
    expect(mocks.saveKey).toHaveBeenCalledWith("employee-key");
    expect(input.value).toBe("");
    expect(host.innerHTML).not.toContain("employee-key");
    (host.querySelectorAll("button")[1] as HTMLButtonElement).click();
    await flush();
    expect(mocks.saveKey).toHaveBeenLastCalledWith("");
  });
  it("changes a model without replacing the key", async () => {
    const select = host.querySelector("select")!;
    select.value = "b";
    select.dispatchEvent(new Event("change"));
    await flush();
    expect(mocks.saveModel).toHaveBeenCalledWith("b");
    expect(mocks.saveKey).not.toHaveBeenCalled();
  });
  it("shows a safe connection error and permits a replacement key", async () => {
    state.enterpriseAiState.hasApiKey = true;
    mocks.test.mockRejectedValue(new Error("[auth] HTTP 401 rejected-token"));
    await nextTick();
    (host.querySelectorAll("button")[2] as HTMLButtonElement).click();
    await flush();
    expect(host.textContent).toContain("IT");
    expect(host.textContent).not.toContain("rejected-token");
    expect(host.textContent).toContain("revoked");
    expect(host.querySelector("input")?.disabled).toBe(false);
  });
  it("explains a rejected personal key without displaying its value", async () => {
    state.enterpriseAiState.hasApiKey = true;
    mocks.test.mockResolvedValue({ success: false, errorCategory: "auth", message: "private-error", modelUsed: "a" });
    await nextTick();
    (host.querySelectorAll("button")[2] as HTMLButtonElement).click();
    await flush();
    expect(host.textContent).toContain("revoked");
    expect(host.textContent).not.toContain("private-error");
  });
});
